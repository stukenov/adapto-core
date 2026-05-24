use adapto_compiler::ir::*;
use adapto_compiler::manifest::{RouteEntry, RouteManifest};
use adapto_runtime::context::{Ctx, PermissionSet};
use adapto_runtime::state::StateStore;
use adapto_runtime::types::*;
use adapto_ssr::error::SsrError;
use adapto_ssr::layout::LayoutManager;
use adapto_ssr::page::PageRenderer;
use adapto_ssr::renderer::Renderer;
use adapto_ssr::router::Router;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn simple_manifest() -> RouteManifest {
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
                id: "customers".into(),
                path: "/customers".into(),
                file: "customers.adapto".into(),
                method: "GET".into(),
                auth: "required".into(),
                tenant: "required".into(),
                permission: None,
                layout: Some("main".into()),
                cache: "no-store".into(),
            },
            RouteEntry {
                id: "customer_detail".into(),
                path: "/customers/:id".into(),
                file: "customer_detail.adapto".into(),
                method: "GET".into(),
                auth: "required".into(),
                tenant: "required".into(),
                permission: Some("customers.read".into()),
                layout: Some("main".into()),
                cache: "no-store".into(),
            },
            RouteEntry {
                id: "customer_orders".into(),
                path: "/customers/:id/orders/:order_id".into(),
                file: "customer_orders.adapto".into(),
                method: "GET".into(),
                auth: "required".into(),
                tenant: "required".into(),
                permission: Some("orders.read".into()),
                layout: Some("main".into()),
                cache: "no-store".into(),
            },
        ],
    }
}

fn static_ir(id: &str, name: &str) -> ComponentIR {
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

fn dynamic_ir() -> ComponentIR {
    ComponentIR {
        id: "page_dyn".into(),
        name: "DynamicPage".into(),
        route: None,
        static_segments: vec![
            "<h1>".into(),
            "</h1><p>Count: ".into(),
            "</p>".into(),
        ],
        dynamic_segments: vec![
            DynamicSegment {
                id: "dyn_0".into(),
                expr: "title".into(),
                deps: vec!["title".into()],
                segment_type: SegmentType::Text,
            },
            DynamicSegment {
                id: "dyn_1".into(),
                expr: "count".into(),
                deps: vec!["count".into()],
                segment_type: SegmentType::Text,
            },
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

fn event_ir() -> ComponentIR {
    ComponentIR {
        id: "page_ev".into(),
        name: "EventPage".into(),
        route: None,
        static_segments: vec!["<button>Click</button>".into()],
        dynamic_segments: vec![],
        events: vec![EventIR {
            id: "ev_0".into(),
            event_type: "click".into(),
            handler: "increment".into(),
            component_id: "page_ev".into(),
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

fn anon_ctx() -> Ctx {
    Ctx {
        user_id: None,
        tenant_id: None,
        request_id: RequestId::default(),
        permissions: PermissionSet::new(),
        route: RouteId::from(""),
        session_id: SessionId::from("sess"),
    }
}

fn authed_ctx() -> Ctx {
    Ctx {
        user_id: Some(UserId::default()),
        tenant_id: Some(TenantId::default()),
        request_id: RequestId::default(),
        permissions: PermissionSet::new(),
        route: RouteId::from(""),
        session_id: SessionId::from("sess"),
    }
}

fn authed_no_tenant_ctx() -> Ctx {
    Ctx {
        user_id: Some(UserId::default()),
        tenant_id: None,
        request_id: RequestId::default(),
        permissions: PermissionSet::new(),
        route: RouteId::from(""),
        session_id: SessionId::from("sess"),
    }
}

fn ctx_with_perm(perm: &str) -> Ctx {
    let mut perms = PermissionSet::new();
    perms.add(perm);
    Ctx {
        user_id: Some(UserId::default()),
        tenant_id: Some(TenantId::default()),
        request_id: RequestId::default(),
        permissions: perms,
        route: RouteId::from(""),
        session_id: SessionId::from("sess"),
    }
}

// ===========================================================================
// Router tests
// ===========================================================================

#[test]
fn router_exact_path_match() {
    let router = Router::new(simple_manifest());
    let m = router.match_route("/customers").unwrap();
    assert_eq!(m.route_id, "customers");
    assert!(m.params.is_empty());
    assert_eq!(m.auth, "required");
}

#[test]
fn router_dynamic_segment_match() {
    let router = Router::new(simple_manifest());
    let m = router.match_route("/customers/456").unwrap();
    assert_eq!(m.route_id, "customer_detail");
    assert_eq!(m.params["id"], "456");
}

#[test]
fn router_nested_dynamic_segments() {
    let router = Router::new(simple_manifest());
    let m = router.match_route("/customers/10/orders/20").unwrap();
    assert_eq!(m.route_id, "customer_orders");
    assert_eq!(m.params["id"], "10");
    assert_eq!(m.params["order_id"], "20");
}

#[test]
fn router_no_match_returns_none() {
    let router = Router::new(simple_manifest());
    assert!(router.match_route("/unknown/path").is_none());
}

#[test]
fn router_extract_params() {
    let router = Router::new(simple_manifest());
    let m = router.match_route("/customers/xyz").unwrap();
    assert_eq!(m.params.len(), 1);
    assert_eq!(m.params["id"], "xyz");
}

// ===========================================================================
// LayoutManager tests
// ===========================================================================

#[test]
fn layout_register_and_compose() {
    let mut mgr = LayoutManager::new();
    mgr.register(
        "shell",
        "<html><body><header>H</header>{slot}<footer>F</footer></body></html>".into(),
    );

    let html = mgr.compose("shell", "<main>Content</main>").unwrap();
    assert!(html.contains("<header>H</header>"));
    assert!(html.contains("<main>Content</main>"));
    assert!(html.contains("<footer>F</footer>"));
}

#[test]
fn layout_unknown_layout_error() {
    let mgr = LayoutManager::new();
    let result = mgr.compose("ghost", "content");
    assert!(result.is_err());
    match result.unwrap_err() {
        SsrError::LayoutNotFound(name) => assert_eq!(name, "ghost"),
        other => panic!("Expected LayoutNotFound, got: {:?}", other),
    }
}

// ===========================================================================
// Renderer tests
// ===========================================================================

#[test]
fn renderer_static_segments_only() {
    let renderer = Renderer::new(b"secret");
    let ir = static_ir("pg", "StaticPage");
    let state = StateStore::new();

    let html = renderer.render_component(&ir, &state).unwrap();
    assert!(html.contains("<h1>StaticPage</h1>"));
    assert!(html.contains("data-ar-root=\"pg\""));
    assert!(!html.contains("data-ar-dyn"));
}

#[test]
fn renderer_dynamic_text_expression() {
    let renderer = Renderer::new(b"secret");
    let ir = dynamic_ir();
    let mut state = StateStore::new();
    state.set("title", Value::String("Hello".into()));
    state.set("count", Value::Number(5.into()));

    let html = renderer.render_component(&ir, &state).unwrap();
    assert!(html.contains("<span data-ar-dyn=\"dyn_0\">Hello</span>"));
    assert!(html.contains("<span data-ar-dyn=\"dyn_1\">5</span>"));
}

#[test]
fn renderer_multiple_dynamic_segments() {
    let renderer = Renderer::new(b"secret");
    let ir = dynamic_ir();
    let mut state = StateStore::new();
    state.set("title", Value::String("Multi".into()));
    state.set("count", Value::Number(99.into()));

    let html = renderer.render_component(&ir, &state).unwrap();
    // Both markers present.
    let dyn_count = html.matches("data-ar-dyn=").count();
    assert_eq!(dyn_count, 2);
}

#[test]
fn renderer_eval_expr_from_state() {
    let renderer = Renderer::new(b"secret");
    let mut state = StateStore::new();
    state.set("greeting", Value::String("Hi there".into()));

    // We test eval_expr indirectly through render_component.
    let ir = ComponentIR {
        id: "eval_test".into(),
        name: "EvalTest".into(),
        route: None,
        static_segments: vec!["<p>".into(), "</p>".into()],
        dynamic_segments: vec![DynamicSegment {
            id: "dyn_eval".into(),
            expr: "greeting".into(),
            deps: vec!["greeting".into()],
            segment_type: SegmentType::Text,
        }],
        events: vec![],
        actions: vec![],
        state_fields: vec![],
        form_schemas: vec![],
        permissions: vec![],
        children: vec![],
        is_island: false,
        style: None,
    };

    let html = renderer.render_component(&ir, &state).unwrap();
    assert!(html.contains("Hi there"));
}

#[test]
fn renderer_data_ar_dyn_attributes() {
    let renderer = Renderer::new(b"secret");
    let ir = dynamic_ir();
    let mut state = StateStore::new();
    state.set("title", Value::String("X".into()));
    state.set("count", Value::Number(0.into()));

    let html = renderer.render_component(&ir, &state).unwrap();
    assert!(html.contains("data-ar-dyn=\"dyn_0\""));
    assert!(html.contains("data-ar-dyn=\"dyn_1\""));
}

#[test]
fn renderer_data_ar_click_for_events() {
    let renderer = Renderer::new(b"secret");
    let ir = event_ir();
    let state = StateStore::new();

    let html = renderer.render_component(&ir, &state).unwrap();
    assert!(html.contains("data-ar-click=\"increment\""));
}

#[test]
fn renderer_page_wrapping_bootstrap() {
    let renderer = Renderer::new(b"secret");
    let ir = static_ir("boot", "BootPage");
    let state = StateStore::new();

    let html = renderer.render_page(&ir, &state, None).unwrap();
    assert!(html.contains("id=\"__ADAPTO_BOOTSTRAP__\""));
    assert!(html.contains("\"session_id\""));
    assert!(html.contains("\"websocket_url\""));
    assert!(html.contains("\"csrf_token\""));
    assert!(html.contains("\"component_tree\""));
}

#[test]
fn renderer_page_wrapping_client_js() {
    let renderer = Renderer::new(b"secret");
    let ir = static_ir("js", "JsPage");
    let state = StateStore::new();

    let html = renderer.render_page(&ir, &state, None).unwrap();
    assert!(html.contains("<script src=\"/assets/adapto-client.js\"></script>"));
}

// ===========================================================================
// PageRenderer (orchestrator) tests
// ===========================================================================

fn build_page_renderer() -> PageRenderer {
    let mut pr = PageRenderer::new(b"test-key");
    pr.set_router(Router::new(simple_manifest()));

    let mut layouts = LayoutManager::new();
    layouts.register(
        "main",
        "<html><body><nav>Nav</nav><main>{slot}</main></body></html>".into(),
    );
    pr.set_layouts(layouts);

    pr.register_component("home", static_ir("home", "Home"));
    pr.register_component("customers", static_ir("customers", "Customers"));
    pr.register_component("customer_detail", static_ir("customer_detail", "Detail"));
    pr.register_component("customer_orders", static_ir("customer_orders", "Orders"));

    pr
}

#[test]
fn page_full_render() {
    let pr = build_page_renderer();
    let resp = pr
        .render_request("/", &anon_ctx(), StateStore::new())
        .unwrap();
    assert_eq!(resp.status, 200);
    assert!(resp.html.contains("<h1>Home</h1>"));
    assert!(resp.html.contains("__ADAPTO_BOOTSTRAP__"));
}

#[test]
fn page_auth_required_anonymous() {
    let pr = build_page_renderer();
    let result = pr.render_request("/customers", &anon_ctx(), StateStore::new());
    assert!(matches!(result.unwrap_err(), SsrError::AuthRequired));
}

#[test]
fn page_tenant_required_missing() {
    let pr = build_page_renderer();
    let result = pr.render_request("/customers", &authed_no_tenant_ctx(), StateStore::new());
    assert!(matches!(result.unwrap_err(), SsrError::TenantRequired));
}

#[test]
fn page_permission_denied() {
    let pr = build_page_renderer();
    // authed_ctx has no "customers.read" permission.
    let result = pr.render_request("/customers/1", &authed_ctx(), StateStore::new());
    assert!(matches!(
        result.unwrap_err(),
        SsrError::PermissionDenied(_)
    ));
}

#[test]
fn page_permission_granted() {
    let pr = build_page_renderer();
    let resp = pr
        .render_request("/customers/1", &ctx_with_perm("customers.read"), StateStore::new())
        .unwrap();
    assert_eq!(resp.status, 200);
    assert!(resp.html.contains("<h1>Detail</h1>"));
}

#[test]
fn page_route_not_found() {
    let pr = build_page_renderer();
    let result = pr.render_request("/nowhere", &anon_ctx(), StateStore::new());
    assert!(matches!(result.unwrap_err(), SsrError::RouteNotFound(_)));
}
