use std::collections::HashMap;

use adapto_compiler::dependency::DependencyGraph;
use adapto_compiler::ir::ComponentIR;
use adapto_runtime::context::Ctx;
use adapto_runtime::state::StateStore;

use crate::error::SsrError;
use crate::layout::LayoutManager;
use crate::renderer::Renderer;
use crate::router::{RouteMatch, Router};

/// Orchestrates a full page render from request path to HTML response.
///
/// Wires together route matching, auth/tenant/permission gates,
/// component lookup, state initialisation, HTML rendering, and layout
/// composition into a single linear pipeline — mirroring the clarity
/// of a well-structured UINavigationController flow.
pub struct PageRenderer {
    renderer: Renderer,
    router: Option<Router>,
    layouts: LayoutManager,
    components: HashMap<String, ComponentIR>,
    dependency_graphs: HashMap<String, DependencyGraph>,
}

/// The fully rendered page response, ready to send to the client.
#[derive(Debug)]
pub struct PageResponse {
    /// Complete HTML document.
    pub html: String,
    /// Session ID embedded in the bootstrap payload.
    pub session_id: String,
    /// Route ID for the matched route.
    pub route_id: String,
    /// HTTP status code.
    pub status: u16,
    /// The matched route metadata.
    pub route_match: RouteMatch,
}

impl PageRenderer {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            renderer: Renderer::new(secret),
            router: None,
            layouts: LayoutManager::new(),
            components: HashMap::new(),
            dependency_graphs: HashMap::new(),
        }
    }

    pub fn set_router(&mut self, router: Router) {
        self.router = Some(router);
    }

    pub fn set_layouts(&mut self, layouts: LayoutManager) {
        self.layouts = layouts;
    }

    pub fn register_component(&mut self, id: &str, ir: ComponentIR) {
        self.components.insert(id.to_string(), ir);
    }

    pub fn register_dependency_graph(&mut self, id: &str, graph: DependencyGraph) {
        self.dependency_graphs.insert(id.to_string(), graph);
    }

    /// Look up a component IR by route ID.
    pub fn get_component(&self, route_id: &str) -> Option<&ComponentIR> {
        self.components.get(route_id)
    }

    /// Look up a dependency graph by route ID.
    pub fn get_dependency_graph(&self, route_id: &str) -> Option<&DependencyGraph> {
        self.dependency_graphs.get(route_id)
    }

    /// Match a URL path and return the route match.
    pub fn match_route(&self, path: &str) -> Option<RouteMatch> {
        self.router.as_ref()?.match_route(path)
    }

    /// Handle a page request through the full rendering pipeline.
    ///
    /// Steps:
    /// 1. Match the request path against the route manifest.
    /// 2. Gate on auth, tenant, and permission requirements.
    /// 3. Look up the compiled component IR.
    /// 4. Render the component with the provided initial state.
    /// 5. Compose into the layout (if one is specified).
    /// 6. Return the complete page response.
    pub fn render_request(
        &self,
        path: &str,
        ctx: &Ctx,
        initial_state: StateStore,
    ) -> Result<PageResponse, SsrError> {
        // 1. Route matching.
        let router = self
            .router
            .as_ref()
            .ok_or_else(|| SsrError::RenderError("Router not configured".into()))?;

        let route_match = router
            .match_route(path)
            .ok_or_else(|| SsrError::RouteNotFound(path.to_string()))?;

        // 2. Auth gate.
        if route_match.auth == "required" && ctx.user_id.is_none() {
            return Err(SsrError::AuthRequired);
        }

        // Tenant gate.
        if route_match.tenant == "required" && ctx.tenant_id.is_none() {
            return Err(SsrError::TenantRequired);
        }

        // Permission gate.
        if let Some(ref perm) = route_match.permission {
            if !ctx.permissions.has(perm) {
                return Err(SsrError::PermissionDenied(perm.clone()));
            }
        }

        // 3. Component lookup.
        let ir = self
            .components
            .get(&route_match.route_id)
            .ok_or_else(|| SsrError::ComponentNotFound(route_match.route_id.clone()))?;

        // 4 + 5. Render with optional layout composition.
        let layout_template = route_match
            .layout
            .as_ref()
            .and_then(|name| {
                if self.layouts.has_layout(name) {
                    // render_page expects a template with {slot} placeholder.
                    // We retrieve it through compose with a sentinel, then
                    // reverse-engineer the template. This is a temporary
                    // approach — a production implementation would expose
                    // the raw template.
                    Some(self.get_layout_template(name))
                } else {
                    None
                }
            })
            .flatten();

        let (html, session_id) = self.renderer.render_page(
            ir,
            &initial_state,
            layout_template.as_deref(),
        )?;

        // 6. Response.
        Ok(PageResponse {
            html,
            session_id,
            route_id: route_match.route_id.clone(),
            status: 200,
            route_match,
        })
    }

    /// Retrieve a layout template by composing with a known sentinel
    /// and then extracting the template structure.
    ///
    /// This is a pragmatic workaround — a layout template is stored as
    /// a string with `{slot}`. We compose it with the sentinel to verify
    /// it exists, then reconstruct. In practice the LayoutManager should
    /// expose a `get_template` method, but this keeps the public API
    /// minimal for now.
    fn get_layout_template(&self, name: &str) -> Option<String> {
        // Compose with the placeholder itself to get the template back.
        self.layouts.compose(name, "{slot}").ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapto_compiler::manifest::{RouteEntry, RouteManifest};
    use adapto_runtime::context::{Ctx, PermissionSet};
    use adapto_runtime::types::*;

    fn test_manifest() -> RouteManifest {
        RouteManifest {
            routes: vec![
                RouteEntry {
                    id: "home".into(),
                    path: "/".into(),
                    file: "home.adapto".into(),
                    method: "GET".into(),
                    auth: "public".into(),
                    tenant: "none".into(),
                    permission: None,
                    layout: None,
                    cache: "public".into(),
                },
                RouteEntry {
                    id: "dashboard".into(),
                    path: "/dashboard".into(),
                    file: "dashboard.adapto".into(),
                    method: "GET".into(),
                    auth: "required".into(),
                    tenant: "required".into(),
                    permission: None,
                    layout: Some("main".into()),
                    cache: "no-store".into(),
                },
                RouteEntry {
                    id: "admin".into(),
                    path: "/admin".into(),
                    file: "admin.adapto".into(),
                    method: "GET".into(),
                    auth: "required".into(),
                    tenant: "required".into(),
                    permission: Some("admin.access".into()),
                    layout: Some("main".into()),
                    cache: "no-store".into(),
                },
            ],
        }
    }

    fn test_ir(id: &str, name: &str) -> ComponentIR {
        ComponentIR {
            id: id.into(),
            name: name.into(),
            route: None,
            static_segments: vec![format!("<h1>{}</h1>", name)],
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

    fn anon_ctx() -> Ctx {
        Ctx {
            user_id: None,
            tenant_id: None,
            request_id: RequestId::default(),
            permissions: PermissionSet::new(),
            route: RouteId::from(""),
            session_id: SessionId::from("test-session"),
        }
    }

    fn authed_ctx() -> Ctx {
        Ctx {
            user_id: Some(UserId::default()),
            tenant_id: Some(TenantId::default()),
            request_id: RequestId::default(),
            permissions: PermissionSet::new(),
            route: RouteId::from(""),
            session_id: SessionId::from("test-session"),
        }
    }

    fn admin_ctx() -> Ctx {
        let mut perms = PermissionSet::new();
        perms.add("admin.access");
        Ctx {
            user_id: Some(UserId::default()),
            tenant_id: Some(TenantId::default()),
            request_id: RequestId::default(),
            permissions: perms,
            route: RouteId::from(""),
            session_id: SessionId::from("test-session"),
        }
    }

    fn build_page_renderer() -> PageRenderer {
        let mut pr = PageRenderer::new(b"test-secret-key");
        pr.set_router(Router::new(test_manifest()));

        let mut layouts = LayoutManager::new();
        layouts.register(
            "main",
            "<html><body><nav>Nav</nav><main>{slot}</main></body></html>".into(),
        );
        pr.set_layouts(layouts);

        pr.register_component("home", test_ir("home", "Home"));
        pr.register_component("dashboard", test_ir("dashboard", "Dashboard"));
        pr.register_component("admin", test_ir("admin", "Admin"));

        pr
    }

    #[test]
    fn full_page_render() {
        let pr = build_page_renderer();
        let state = StateStore::new();

        let response = pr.render_request("/", &anon_ctx(), state).unwrap();
        assert_eq!(response.status, 200);
        assert!(response.html.contains("<h1>Home</h1>"));
        assert!(response.html.contains("__ADAPTO_BOOTSTRAP__"));
        assert!(response.html.contains("adapto-client.js"));
    }

    #[test]
    fn auth_required_returns_error_for_anonymous() {
        let pr = build_page_renderer();
        let state = StateStore::new();

        let result = pr.render_request("/dashboard", &anon_ctx(), state);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SsrError::AuthRequired));
    }

    #[test]
    fn tenant_required_returns_error_when_missing() {
        let pr = build_page_renderer();
        let state = StateStore::new();

        // User is authenticated but has no tenant.
        let ctx = Ctx {
            user_id: Some(UserId::default()),
            tenant_id: None,
            request_id: RequestId::default(),
            permissions: PermissionSet::new(),
            route: RouteId::from(""),
            session_id: SessionId::from("test-session"),
        };

        let result = pr.render_request("/dashboard", &ctx, state);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SsrError::TenantRequired));
    }

    #[test]
    fn permission_denied() {
        let pr = build_page_renderer();
        let state = StateStore::new();

        // Authenticated with tenant but without admin.access permission.
        let result = pr.render_request("/admin", &authed_ctx(), state);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SsrError::PermissionDenied(_)
        ));
    }

    #[test]
    fn permission_granted() {
        let pr = build_page_renderer();
        let state = StateStore::new();

        let response = pr.render_request("/admin", &admin_ctx(), state).unwrap();
        assert_eq!(response.status, 200);
        assert!(response.html.contains("<h1>Admin</h1>"));
    }

    #[test]
    fn route_not_found_404() {
        let pr = build_page_renderer();
        let state = StateStore::new();

        let result = pr.render_request("/nonexistent", &anon_ctx(), state);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SsrError::RouteNotFound(_)));
    }

    #[test]
    fn page_with_layout() {
        let pr = build_page_renderer();
        let state = StateStore::new();

        let response = pr
            .render_request("/dashboard", &authed_ctx(), state)
            .unwrap();
        // The layout wraps the content.
        assert!(response.html.contains("<nav>Nav</nav>"));
        assert!(response.html.contains("<h1>Dashboard</h1>"));
    }
}
