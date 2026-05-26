use adapto_compiler::compiler::Compiler;
use adapto_compiler::ir::*;
use adapto_compiler::manifest::{RouteEntry, RouteManifest};
use adapto_parser;
use adapto_runtime::context::{Ctx, PermissionSet};
use adapto_runtime::state::StateStore;
use adapto_runtime::types::*;
use adapto_ssr::error::SsrError;
use adapto_ssr::layout::LayoutManager;
use adapto_ssr::page::PageRenderer;
use adapto_ssr::renderer::Renderer;
use adapto_ssr::router::Router;
use serde_json::{json, Value};

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

// ===========================================================================
// Additional LayoutManager tests
// ===========================================================================

#[test]
fn layout_has_layout_true_after_register() {
    let mut mgr = LayoutManager::new();
    assert!(!mgr.has_layout("admin"));
    mgr.register("admin", "<div>{slot}</div>".into());
    assert!(mgr.has_layout("admin"));
}

#[test]
fn layout_compose_replaces_slot_exactly_once() {
    let mut mgr = LayoutManager::new();
    mgr.register("wrap", "<main>{slot}</main>".into());
    let html = mgr.compose("wrap", "<p>inner</p>").unwrap();
    assert_eq!(html, "<main><p>inner</p></main>");
}

#[test]
fn layout_multiple_layouts_independent() {
    let mut mgr = LayoutManager::new();
    mgr.register("a", "<div>A:{slot}</div>".into());
    mgr.register("b", "<section>B:{slot}</section>".into());

    let ha = mgr.compose("a", "X").unwrap();
    let hb = mgr.compose("b", "Y").unwrap();
    assert_eq!(ha, "<div>A:X</div>");
    assert_eq!(hb, "<section>B:Y</section>");
}

#[test]
fn layout_missing_layout_returns_correct_name() {
    let mgr = LayoutManager::new();
    match mgr.compose("nonexistent", "c").unwrap_err() {
        SsrError::LayoutNotFound(name) => assert_eq!(name, "nonexistent"),
        other => panic!("Expected LayoutNotFound, got: {:?}", other),
    }
}

// ===========================================================================
// Additional Router tests
// ===========================================================================

#[test]
fn router_root_path_match() {
    let router = Router::new(simple_manifest());
    let m = router.match_route("/").unwrap();
    assert_eq!(m.route_id, "home");
    assert_eq!(m.auth, "public");
    assert_eq!(m.tenant, "none");
}

#[test]
fn router_trailing_slash_normalized() {
    let router = Router::new(simple_manifest());
    let m = router.match_route("/customers/").unwrap();
    assert_eq!(m.route_id, "customers");
}

#[test]
fn router_preserves_layout_and_permission() {
    let router = Router::new(simple_manifest());
    let m = router.match_route("/customers/1").unwrap();
    assert_eq!(m.layout, Some("main".into()));
    assert_eq!(m.permission, Some("customers.read".into()));
}

#[test]
fn router_no_match_deep_path() {
    let router = Router::new(simple_manifest());
    assert!(router.match_route("/customers/1/orders/2/items").is_none());
}

#[test]
fn router_multiple_params_extracted() {
    let router = Router::new(simple_manifest());
    let m = router.match_route("/customers/abc/orders/def").unwrap();
    assert_eq!(m.params.len(), 2);
    assert_eq!(m.params["id"], "abc");
    assert_eq!(m.params["order_id"], "def");
}

// ===========================================================================
// Additional Renderer tests
// ===========================================================================

#[test]
fn renderer_empty_state_shows_placeholder() {
    let renderer = Renderer::new(b"secret");
    let ir = dynamic_ir();
    let state = StateStore::new();

    let html = renderer.render_component(&ir, &state).unwrap();
    assert!(html.contains("{title}"));
    assert!(html.contains("{count}"));
}

#[test]
fn renderer_static_only_no_dyn_markers() {
    let renderer = Renderer::new(b"secret");
    let ir = static_ir("clean", "CleanPage");
    let state = StateStore::new();

    let html = renderer.render_component(&ir, &state).unwrap();
    assert!(!html.contains("data-ar-dyn"));
    assert!(html.contains("data-ar-root=\"clean\""));
}

#[test]
fn renderer_html_escapes_dynamic_values() {
    let renderer = Renderer::new(b"secret");
    let ir = ComponentIR {
        id: "esc".into(),
        name: "EscPage".into(),
        route: None,
        static_segments: vec!["<p>".into(), "</p>".into()],
        dynamic_segments: vec![DynamicSegment {
            id: "dyn_esc".into(),
            expr: "val".into(),
            deps: vec!["val".into()],
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

    let mut state = StateStore::new();
    state.set("val", Value::String("<script>alert(1)</script>".into()));

    let html = renderer.render_component(&ir, &state).unwrap();
    assert!(html.contains("&lt;script&gt;"));
    assert!(!html.contains("<script>alert"));
}

#[test]
fn renderer_page_with_layout() {
    let renderer = Renderer::new(b"secret");
    let ir = static_ir("laid", "LaidPage");
    let state = StateStore::new();

    let layout = "<html><body><nav>N</nav>{slot}<footer>F</footer></body></html>";
    let html = renderer.render_page(&ir, &state, Some(layout)).unwrap();
    assert!(html.contains("<nav>N</nav>"));
    assert!(html.contains("<h1>LaidPage</h1>"));
    assert!(html.contains("<footer>F</footer>"));
}

// ===========================================================================
// END-TO-END INTEGRATION: parse → compile → render
// ===========================================================================

#[test]
fn e2e_counter_parse_compile_render() {
    let dsl = r#"
<route>
  path: "/counter"
  method: GET
  auth: public
</route>

<script lang="rust">
  state count: i32 = 0

  action fn increment() {
    count += 1;
  }
</script>

<template>
  <div class="counter">
    <h1>Counter</h1>
    <span class="value">{count}</span>
    <button on:click="increment">+1</button>
  </div>
</template>

<style scoped>
  .counter { text-align: center; }
</style>
"#;

    // Step 1: Parse
    let ast = adapto_parser::parse(dsl).expect("Parse should succeed");
    assert!(ast.route.is_some());
    assert!(ast.script.is_some());
    assert!(ast.template.is_some());
    assert!(ast.style.is_some());

    let route = ast.route.as_ref().unwrap();
    assert_eq!(route.path.as_deref(), Some("/counter"));

    let script = ast.script.as_ref().unwrap();
    assert_eq!(script.states.len(), 1);
    assert_eq!(script.states[0].name, "count");
    assert_eq!(script.actions.len(), 1);
    assert_eq!(script.actions[0].name, "increment");

    // Step 2: Compile
    let mut compiler = Compiler::new();
    let output = compiler.compile_file(&ast, "counter.adapto")
        .expect("Compile should succeed");

    let ir = &output.component_ir;
    assert!(!ir.static_segments.is_empty(), "Should have static HTML segments");
    assert!(!ir.dynamic_segments.is_empty(), "Should have dynamic segments for count");
    assert!(!ir.events.is_empty(), "Should have event IR for on:click");
    assert!(!ir.actions.is_empty(), "Should have action IR for increment");

    // Verify dependency graph tracks count → dyn segment
    let deps = &output.dependency_graph;
    let affected = deps.get_affected_segments(&["count"]);
    assert!(!affected.is_empty(), "Changing 'count' should affect at least one segment");

    // Verify generated Rust code
    assert!(!output.generated_rust.is_empty());
    assert!(output.generated_rust.contains("CounterState"));

    // Verify route entry
    assert!(output.route_entry.is_some());
    let re = output.route_entry.as_ref().unwrap();
    assert_eq!(re.path, "/counter");

    // Step 3: Render
    let renderer = Renderer::new(b"test_secret");
    let mut state = StateStore::new();
    state.set("count", json!(42));

    let html = renderer.render_component(ir, &state)
        .expect("Render should succeed");

    assert!(html.contains("Counter"), "Should render heading");
    assert!(html.contains("42"), "Should render state value");
    assert!(html.contains("data-ar-click"), "Should have event binding attribute");
}

#[test]
fn e2e_customer_page_parse_compile_render() {
    let dsl = r#"
<route>
  path: "/customers"
  method: GET
  auth: required
  tenant: required
  permission: "customers.read"
  layout: "dashboard"
</route>

<script lang="rust">
  state query: String = ""
  state customers: Vec<Customer> = []
  state loading: bool = false

  load async fn load(ctx: Ctx) {
    customers = CustomerRepo::for_tenant(ctx.tenant_id).await?;
  }

  action async fn search(ctx: Ctx) {
    loading = true;
    customers = CustomerRepo::search(ctx.tenant_id, query.clone()).await?;
    loading = false;
  }

  #[permission("customers.delete")]
  #[audit("customer.deleted")]
  action async fn delete(id: Uuid, ctx: Ctx) {
    CustomerRepo::delete(ctx.tenant_id, id).await?;
  }
</script>

<template>
  <div class="page">
    <h1>Customers</h1>
    <input bind:value="query" on:input.debounce.300="search" />
    {#if loading}
      <span>Loading...</span>
    {:else}
      {#each customers as customer (customer.id)}
        <div class="row">{customer.name}</div>
      {/each}
    {/if}
    {#can "customers.delete"}
      <button on:click="delete">Delete</button>
    {/can}
  </div>
</template>
"#;

    // Step 1: Parse
    let ast = adapto_parser::parse(dsl).expect("Parse should succeed");

    let route = ast.route.as_ref().unwrap();
    assert_eq!(route.path.as_deref(), Some("/customers"));
    assert_eq!(route.auth, Some(adapto_parser::ast::AuthLevel::Required));
    assert_eq!(route.tenant, Some(adapto_parser::ast::TenantLevel::Required));
    assert_eq!(route.permission.as_deref(), Some("customers.read"));
    assert_eq!(route.layout.as_deref(), Some("dashboard"));

    let script = ast.script.as_ref().unwrap();
    assert_eq!(script.states.len(), 3);
    assert_eq!(script.loaders.len(), 1);
    assert_eq!(script.actions.len(), 2);
    assert_eq!(script.actions[1].permission.as_deref(), Some("customers.delete"));
    assert_eq!(script.actions[1].audit.as_deref(), Some("customer.deleted"));

    // Step 2: Compile
    let mut compiler = Compiler::new();
    let output = compiler.compile_file(&ast, "customers.adapto")
        .expect("Compile should succeed");

    let ir = &output.component_ir;

    // Security: should have permission requirement
    assert!(ir.permissions.contains(&"customers.read".to_string())
         || ir.permissions.contains(&"customers.delete".to_string()),
        "Should track permissions");

    // Actions should carry audit metadata
    let delete_action = ir.actions.iter().find(|a| a.name == "delete");
    assert!(delete_action.is_some(), "Should have delete action");
    assert!(delete_action.unwrap().audit.is_some(), "Delete action should have audit");

    // Verify route manifest
    let manifest = compiler.route_manifest();
    let found = manifest.find_by_path("/customers");
    assert!(found.is_some());
    assert_eq!(found.unwrap().auth, "required");
    assert_eq!(found.unwrap().tenant, "required");

    // Step 3: Render
    let renderer = Renderer::new(b"test_secret");
    let mut state = StateStore::new();
    state.set("query", json!(""));
    state.set("loading", json!(false));
    state.set("customers", json!([
        {"id": "1", "name": "Alice Corp"},
        {"id": "2", "name": "Bob LLC"}
    ]));

    let html = renderer.render_component(ir, &state)
        .expect("Render should succeed");

    assert!(html.contains("Customers"), "Should render page title");
}

#[test]
fn e2e_resource_parse() {
    let dsl = r#"
<resource name="Customer" table="customers">
  tenant: required
  primary_key: id

  field id: Uuid readonly
  field name: String required max=120 searchable
  field email: Email required unique
  field status: Enum[active, inactive] default=active
  field created_at: DateTime readonly

  permission read: "customers.read"
  permission create: "customers.create"
</resource>
"#;

    let ast = adapto_parser::parse(dsl).expect("Resource parse should succeed");
    let resource = ast.resource.as_ref().expect("Should have resource block");

    assert_eq!(resource.name, "Customer");
    assert_eq!(resource.table, "customers");
    assert_eq!(resource.primary_key, "id");
    assert_eq!(resource.fields.len(), 5);

    let name_field = &resource.fields[1];
    assert_eq!(name_field.name, "name");
    assert!(name_field.searchable);

    let email_field = &resource.fields[2];
    assert_eq!(email_field.name, "email");

    assert_eq!(resource.permissions.len(), 2);
    assert_eq!(resource.permissions[0].action, "read");
    assert_eq!(resource.permissions[0].permission, "customers.read");
}

#[test]
fn e2e_compile_generates_codegen_output() {
    let dsl = r#"
<route>
  path: "/hello"
  method: GET
  auth: public
</route>

<script lang="rust">
  state name: String = "World"
</script>

<template>
  <h1>Hello, {name}!</h1>
</template>
"#;

    let ast = adapto_parser::parse(dsl).unwrap();
    let mut compiler = Compiler::new();
    let output = compiler.compile_file(&ast, "hello.adapto").unwrap();

    // Generated Rust should contain state struct and component impl
    let rust = &output.generated_rust;
    assert!(rust.contains("HelloState") || rust.contains("State"),
        "Should generate state struct");
    assert!(rust.contains("render") || rust.contains("Rendered"),
        "Should generate render method");

    // IR should have correct static/dynamic split
    let ir = &output.component_ir;
    let has_name_dep = ir.dynamic_segments.iter()
        .any(|seg| seg.deps.contains(&"name".to_string()));
    assert!(has_name_dep, "Dynamic segment should depend on 'name'");
}

#[test]
fn e2e_page_renderer_full_flow() {
    let dsl = r#"
<route>
  path: "/test"
  method: GET
  auth: public
</route>

<script lang="rust">
  state message: String = "Hello"
</script>

<template>
  <p>{message}</p>
</template>
"#;

    let ast = adapto_parser::parse(dsl).unwrap();
    let mut compiler = Compiler::new();
    let output = compiler.compile_file(&ast, "test.adapto").unwrap();

    // Set up PageRenderer with router
    let manifest = compiler.route_manifest().clone();
    let route_entry = manifest.routes.first().expect("Should have route entry");
    let route_id = route_entry.id.clone();

    let router = Router::new(manifest);
    let mut page_renderer = PageRenderer::new(b"secret");
    page_renderer.set_router(router);
    // Register component with the route ID that PageRenderer uses for lookup
    page_renderer.register_component(&route_id, output.component_ir.clone());

    // Create context
    let ctx = Ctx {
        user_id: None,
        tenant_id: None,
        request_id: RequestId::default(),
        permissions: PermissionSet::new(),
        route: RouteId::from("/test"),
        session_id: SessionId::from("s1"),
    };

    let mut state = StateStore::new();
    state.set("message", json!("Greetings"));

    let response = page_renderer.render_request("/test", &ctx, state)
        .expect("Page render should succeed");

    assert!(!response.html.is_empty(), "Should produce HTML output");
}
