use adapto_auth::csrf;
use adapto_client_protocol::session::{BootstrapPayload, ComponentMeta, DynamicTarget};
use adapto_compiler::ir::{ComponentIR, DynamicSegment, EventIR, SegmentBody, SegmentType};
use adapto_runtime::state::StateStore;
use serde_json::Value;
use uuid::Uuid;

use crate::error::SsrError;

/// Server-side HTML renderer.
///
/// Takes a compiled `ComponentIR` and a populated `StateStore`, then
/// produces the initial HTML that the client will hydrate. The output
/// includes data attributes for the client runtime to locate dynamic
/// segments and event targets, plus a signed bootstrap payload
/// embedded as JSON for zero-roundtrip hydration.
pub struct Renderer {
    /// Shared secret used to sign CSRF tokens in the bootstrap payload.
    secret: Vec<u8>,
}

impl Renderer {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret: secret.to_vec(),
        }
    }

    // ------------------------------------------------------------------
    // Public API
    // ------------------------------------------------------------------

    /// Render a full HTML page: component content wrapped in an optional
    /// layout, plus the bootstrap script block and client JS tag.
    /// Render a full HTML page. Returns `(html, session_id)`.
    pub fn render_page(
        &self,
        ir: &ComponentIR,
        state: &StateStore,
        layout_html: Option<&str>,
    ) -> Result<(String, String), SsrError> {
        let content = self.render_component(ir, state)?;
        let session_id = Uuid::new_v4().to_string();
        let bootstrap = self.generate_bootstrap(ir, &session_id);
        let styles = ir.style.as_ref().map(|s| s.css.as_str());

        let page_content = match layout_html {
            Some(layout) => layout.replace("{slot}", &content),
            None => content,
        };

        Ok((self.wrap_page(&page_content, &bootstrap, styles), session_id))
    }

    /// Render a single component to an HTML fragment.
    ///
    /// The fragment is wrapped in a root `<div>` with `data-ar-root`
    /// set to the component's ID, providing a stable anchor for the
    /// client runtime.
    pub fn render_component(
        &self,
        ir: &ComponentIR,
        state: &StateStore,
    ) -> Result<String, SsrError> {
        let body = self.render_segments(&ir.static_segments, &ir.dynamic_segments, state);
        let events_html = self.render_event_attrs(&ir.events);

        Ok(format!(
            "<div data-ar-root=\"{}\">{}{}</div>",
            ir.id, body, events_html
        ))
    }

    // ------------------------------------------------------------------
    // Segment rendering
    // ------------------------------------------------------------------

    /// Interleave static HTML fragments with evaluated dynamic segments.
    ///
    /// Static parts are emitted verbatim. Each dynamic segment is
    /// rendered as a `<span>` with a `data-ar-dyn` attribute so the
    /// client runtime can patch it independently.
    fn render_segments(
        &self,
        static_parts: &[String],
        dynamic_parts: &[DynamicSegment],
        state: &StateStore,
    ) -> String {
        let mut out = String::new();

        for (i, static_part) in static_parts.iter().enumerate() {
            out.push_str(static_part);

            if let Some(dyn_seg) = dynamic_parts.get(i) {
                match &dyn_seg.segment_type {
                    SegmentType::Attribute { element_id, attr_name } => {
                        let value = self.eval_expr(&dyn_seg.expr, state);
                        out.push_str(&format!(
                            " data-ar-bind-{}=\"{}\" {}=\"{}\"",
                            element_id, dyn_seg.id, attr_name, html_escape(&value)
                        ));
                    }
                    SegmentType::Conditional => {
                        out.push_str(&format!("<span data-ar-dyn=\"{}\">", dyn_seg.id));
                        out.push_str(&self.render_conditional(dyn_seg, state));
                        out.push_str("</span>");
                    }
                    SegmentType::Loop => {
                        out.push_str(&format!("<span data-ar-dyn=\"{}\">", dyn_seg.id));
                        out.push_str(&self.render_loop(dyn_seg, state));
                        out.push_str("</span>");
                    }
                    SegmentType::Permission => {
                        out.push_str(&format!("<span data-ar-dyn=\"{}\">", dyn_seg.id));
                        if let Some(ref body) = dyn_seg.permission_body {
                            out.push_str(&self.render_body(body, state));
                        }
                        out.push_str("</span>");
                    }
                    SegmentType::Text | SegmentType::Html => {
                        let value = self.eval_expr(&dyn_seg.expr, state);
                        let content = if matches!(dyn_seg.segment_type, SegmentType::Html) {
                            value
                        } else {
                            html_escape(&value)
                        };
                        out.push_str(&format!(
                            "<span data-ar-dyn=\"{}\">{}</span>",
                            dyn_seg.id, content
                        ));
                    }
                }
            }
        }

        out
    }

    fn render_conditional(&self, seg: &DynamicSegment, state: &StateStore) -> String {
        let condition_val = self.eval_expr(&seg.expr, state);
        if is_truthy(&condition_val) {
            if let Some(ref body) = seg.then_body {
                return self.render_body(body, state);
            }
        }

        for (cond, body) in &seg.else_if_bodies {
            let val = self.eval_expr(cond, state);
            if is_truthy(&val) {
                return self.render_body(body, state);
            }
        }

        if let Some(ref body) = seg.else_body {
            return self.render_body(body, state);
        }

        String::new()
    }

    fn render_loop(&self, seg: &DynamicSegment, state: &StateStore) -> String {
        let Some(ref loop_body) = seg.loop_body else {
            return String::new();
        };

        let iterable_val = self.eval_expr_raw(&seg.expr, state);
        let items = match iterable_val {
            Value::Array(arr) => arr,
            _ => return String::new(),
        };

        let mut out = String::new();
        for (idx, item) in items.iter().enumerate() {
            let mut scoped = state.clone();
            scoped.set(&loop_body.item_var, item.clone());
            if let Some(ref index_var) = loop_body.index_var {
                scoped.set(index_var, Value::Number(idx.into()));
            }
            out.push_str(&self.render_body(&loop_body.body, &scoped));
        }
        out
    }

    fn render_body(&self, body: &SegmentBody, state: &StateStore) -> String {
        self.render_segments(&body.static_segments, &body.dynamic_segments, state)
    }

    fn eval_expr_raw(&self, expr: &str, state: &StateStore) -> Value {
        let trimmed = expr.trim();

        if let Some(val) = state.get(trimmed) {
            return val.clone();
        }

        let bare = trimmed.strip_prefix("state.").unwrap_or(trimmed);
        if bare != trimmed {
            if let Some(val) = state.get(bare) {
                return val.clone();
            }
        }

        for key in [bare, trimmed] {
            if let Some(dot_pos) = key.find('.') {
                let root = &key[..dot_pos];
                let rest = &key[dot_pos + 1..];
                if let Some(root_val) = state.get(root) {
                    return traverse_path(root_val, rest);
                }
            }
        }

        Value::Null
    }

    /// Render event binding markers as hidden data attributes.
    ///
    /// Each event binding produces a `data-ar-{event}="{handler}"`
    /// attribute on a marker element that the client runtime picks up
    /// during hydration.
    fn render_event_attrs(&self, events: &[EventIR]) -> String {
        if events.is_empty() {
            return String::new();
        }

        let mut parts = Vec::new();
        for ev in events {
            parts.push(format!(
                "<span data-ar-el=\"{}\" data-ar-{}=\"{}\" style=\"display:none\"></span>",
                ev.element_id, ev.event_type, ev.handler
            ));
        }
        parts.join("")
    }

    // ------------------------------------------------------------------
    // Expression evaluation
    // ------------------------------------------------------------------

    /// Evaluate a simple expression against the state store.
    ///
    /// Supports dotted paths like `"customer.name"` by traversing
    /// nested JSON values. Falls back to the raw expression string
    /// if the key is not found (preserving template intent for
    /// debugging).
    fn eval_expr(&self, expr: &str, state: &StateStore) -> String {
        let trimmed = expr.trim();

        // Try direct key first.
        if let Some(val) = state.get(trimmed) {
            return value_to_string(val);
        }

        // Strip "state." prefix if present (compiler emits state-qualified exprs).
        let bare = trimmed.strip_prefix("state.").unwrap_or(trimmed);
        if bare != trimmed {
            if let Some(val) = state.get(bare) {
                return value_to_string(val);
            }
        }

        // Try dotted path on bare (state.-stripped) expression first,
        // then fall back to the original.
        for key in [bare, trimmed] {
            if let Some(dot_pos) = key.find('.') {
                let root = &key[..dot_pos];
                let rest = &key[dot_pos + 1..];
                if let Some(root_val) = state.get(root) {
                    let result = traverse_path(root_val, rest);
                    return value_to_string(&result);
                }
            }
        }

        format!("{{{}}}", trimmed)
    }

    // ------------------------------------------------------------------
    // Bootstrap payload
    // ------------------------------------------------------------------

    /// Generate the bootstrap payload that the client runtime reads
    /// to establish a WebSocket connection and hydrate the component
    /// tree.
    fn generate_bootstrap(&self, ir: &ComponentIR, session_id: &str) -> BootstrapPayload {
        let dynamic_targets: Vec<DynamicTarget> = ir
            .dynamic_segments
            .iter()
            .map(|ds| DynamicTarget {
                id: ds.id.clone(),
                deps: ds.deps.clone(),
            })
            .collect();

        let component_meta = ComponentMeta {
            id: ir.id.clone(),
            name: ir.name.clone(),
            dynamic_targets,
        };

        let csrf_token = csrf::generate_token(&self.secret);

        // Hash the initial state by serializing the component ID +
        // dynamic segment count. A real implementation would hash the
        // full rendered state.
        let state_hash = format!("{}:{}", ir.id, ir.dynamic_segments.len());

        BootstrapPayload {
            session_id: session_id.to_string(),
            websocket_url: "/ws".to_string(),
            csrf_token,
            initial_state_hash: state_hash,
            component_tree: vec![component_meta],
        }
    }

    // ------------------------------------------------------------------
    // Page wrapping
    // ------------------------------------------------------------------

    /// Wrap rendered content in a full HTML document.
    ///
    /// Injects:
    /// - Optional `<style>` block for scoped CSS
    /// - The bootstrap payload as a JSON script tag
    /// - The client runtime script
    fn wrap_page(
        &self,
        content: &str,
        bootstrap: &BootstrapPayload,
        styles: Option<&str>,
    ) -> String {
        let style_block = match styles {
            Some(css) => format!("<style>{}</style>", css),
            None => String::new(),
        };

        let bootstrap_json =
            serde_json::to_string(bootstrap).unwrap_or_else(|_| "{}".to_string());

        format!(
            "<!DOCTYPE html>\
            <html><head><meta charset=\"utf-8\">{style_block}</head>\
            <body>\
            {content}\
            <script type=\"application/json\" id=\"__ADAPTO_BOOTSTRAP__\">{bootstrap_json}</script>\
            <script src=\"/assets/adapto-client.js\"></script>\
            </body></html>"
        )
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a `serde_json::Value` to a display string.
///
/// Strings are unwrapped (no surrounding quotes). Other types use
/// their JSON representation.
fn value_to_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        _ => val.to_string(),
    }
}

/// Traverse a nested JSON value using a dotted path.
fn traverse_path<'a>(val: &'a Value, path: &str) -> Value {
    let mut current = val;
    for segment in path.split('.') {
        match current {
            Value::Object(map) => {
                if let Some(next) = map.get(segment) {
                    current = next;
                } else {
                    return Value::Null;
                }
            }
            _ => return Value::Null,
        }
    }
    current.clone()
}

/// Check if a rendered value is truthy (for conditionals).
fn is_truthy(val: &str) -> bool {
    !val.is_empty() && val != "false" && val != "0" && val != "null" && val != "{}" && val != "[]"
}

/// Minimal HTML entity escaping for dynamic values.
fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapto_compiler::ir::*;

    fn minimal_ir() -> ComponentIR {
        ComponentIR {
            id: "page_01".into(),
            name: "TestPage".into(),
            route: None,
            static_segments: vec!["<h1>Hello</h1>".into()],
            dynamic_segments: vec![],
            events: vec![],
            actions: vec![],
            state_fields: vec![],
            form_schemas: vec![],
            permissions: vec![],
            children: vec![],
            is_island: false,
            style: None,
        }
    }

    fn ir_with_dynamics() -> ComponentIR {
        ComponentIR {
            id: "page_02".into(),
            name: "DynamicPage".into(),
            route: None,
            static_segments: vec![
                "<h1>".into(),
                "</h1><p>Count: ".into(),
                "</p>".into(),
            ],
            dynamic_segments: vec![
                DynamicSegment::new(
                    "dyn_0".into(),
                    "title".into(),
                    vec!["title".into()],
                    SegmentType::Text,
                ),
                DynamicSegment::new(
                    "dyn_1".into(),
                    "count".into(),
                    vec!["count".into()],
                    SegmentType::Text,
                ),
            ],
            events: vec![],
            actions: vec![],
            state_fields: vec![],
            form_schemas: vec![],
            permissions: vec![],
            children: vec![],
            is_island: false,
            style: None,
        }
    }

    fn ir_with_events() -> ComponentIR {
        ComponentIR {
            id: "page_03".into(),
            name: "EventPage".into(),
            route: None,
            static_segments: vec!["<button>Click me</button>".into()],
            dynamic_segments: vec![],
            events: vec![EventIR {
                id: "ev_0".into(),
                event_type: "click".into(),
                handler: "increment".into(),
                component_id: "page_03".into(),
                modifiers: vec![],
                element_id: "btn_0".into(),
            }],
            actions: vec![],
            state_fields: vec![],
            form_schemas: vec![],
            permissions: vec![],
            children: vec![],
            is_island: false,
            style: None,
        }
    }

    #[test]
    fn render_static_segments_only() {
        let renderer = Renderer::new(b"test-secret");
        let ir = minimal_ir();
        let state = StateStore::new();
        let html = renderer.render_component(&ir, &state).unwrap();
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("data-ar-root=\"page_01\""));
    }

    #[test]
    fn render_dynamic_text_expression() {
        let renderer = Renderer::new(b"test-secret");
        let ir = ir_with_dynamics();
        let mut state = StateStore::new();
        state.set("title", Value::String("Welcome".into()));
        state.set("count", Value::Number(42.into()));

        let html = renderer.render_component(&ir, &state).unwrap();
        assert!(html.contains("<span data-ar-dyn=\"dyn_0\">Welcome</span>"));
        assert!(html.contains("<span data-ar-dyn=\"dyn_1\">42</span>"));
    }

    #[test]
    fn render_multiple_dynamic_segments() {
        let renderer = Renderer::new(b"test-secret");
        let ir = ir_with_dynamics();
        let mut state = StateStore::new();
        state.set("title", Value::String("Page Title".into()));
        state.set("count", Value::Number(7.into()));

        let html = renderer.render_component(&ir, &state).unwrap();

        // Both dynamic segments present.
        assert!(html.contains("data-ar-dyn=\"dyn_0\""));
        assert!(html.contains("data-ar-dyn=\"dyn_1\""));

        // Structural integrity.
        assert!(html.contains("<h1>"));
        assert!(html.contains("</h1>"));
        assert!(html.contains("Count: "));
    }

    #[test]
    fn eval_expr_from_state() {
        let renderer = Renderer::new(b"test-secret");
        let mut state = StateStore::new();
        state.set("name", Value::String("Alice".into()));

        assert_eq!(renderer.eval_expr("name", &state), "Alice");
    }

    #[test]
    fn eval_expr_dotted_path() {
        let renderer = Renderer::new(b"test-secret");
        let mut state = StateStore::new();
        state.set(
            "customer",
            serde_json::json!({"name": "Bob", "address": {"city": "Almaty"}}),
        );

        assert_eq!(renderer.eval_expr("customer.name", &state), "Bob");
        assert_eq!(
            renderer.eval_expr("customer.address.city", &state),
            "Almaty"
        );
    }

    #[test]
    fn eval_expr_missing_returns_placeholder() {
        let renderer = Renderer::new(b"test-secret");
        let state = StateStore::new();
        assert_eq!(renderer.eval_expr("missing", &state), "{missing}");
    }

    #[test]
    fn generated_html_has_data_ar_dyn_attributes() {
        let renderer = Renderer::new(b"test-secret");
        let ir = ir_with_dynamics();
        let mut state = StateStore::new();
        state.set("title", Value::String("T".into()));
        state.set("count", Value::Number(0.into()));

        let html = renderer.render_component(&ir, &state).unwrap();
        assert!(html.contains("data-ar-dyn=\"dyn_0\""));
        assert!(html.contains("data-ar-dyn=\"dyn_1\""));
    }

    #[test]
    fn generated_html_has_data_ar_click_for_events() {
        let renderer = Renderer::new(b"test-secret");
        let ir = ir_with_events();
        let state = StateStore::new();

        let html = renderer.render_component(&ir, &state).unwrap();
        assert!(html.contains("data-ar-click=\"increment\""));
        assert!(html.contains("data-ar-el=\"btn_0\""));
    }

    #[test]
    fn page_wrapping_includes_bootstrap_script() {
        let renderer = Renderer::new(b"test-secret");
        let ir = minimal_ir();
        let state = StateStore::new();

        let (html, _) = renderer.render_page(&ir, &state, None).unwrap();
        assert!(html.contains("id=\"__ADAPTO_BOOTSTRAP__\""));
        assert!(html.contains("\"session_id\""));
        assert!(html.contains("\"csrf_token\""));
    }

    #[test]
    fn page_wrapping_includes_client_js() {
        let renderer = Renderer::new(b"test-secret");
        let ir = minimal_ir();
        let state = StateStore::new();

        let (html, _) = renderer.render_page(&ir, &state, None).unwrap();
        assert!(html.contains("src=\"/assets/adapto-client.js\""));
    }

    #[test]
    fn html_escaping() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"hello\""), "&quot;hello&quot;");
    }

    #[test]
    fn page_with_styles() {
        let renderer = Renderer::new(b"test-secret");
        let mut ir = minimal_ir();
        ir.style = Some(CompiledStyle {
            css: ".title { color: red; }".into(),
            scoped: true,
            scope_id: Some("sc_01".into()),
        });
        let state = StateStore::new();

        let (html, _) = renderer.render_page(&ir, &state, None).unwrap();
        assert!(html.contains("<style>.title { color: red; }</style>"));
    }
}
