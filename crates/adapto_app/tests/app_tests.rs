use adapto_app::{App, LayoutConfig, LIVE_JS};

// ---------------------------------------------------------------------------
// App builder tests
// ---------------------------------------------------------------------------

#[test]
fn app_default_configuration() {
    let app = App::new("Test App");
    assert_eq!(app.title(), "Test App");
    assert_eq!(app.configured_port(), 3000);
    assert!(app.configured_store_path().is_none());
    assert_eq!(app.resource_count(), 0);
}

#[test]
fn app_custom_configuration() {
    let app = App::new("CRM")
        .port(8080)
        .store_path("./data/crm");

    assert_eq!(app.title(), "CRM");
    assert_eq!(app.configured_port(), 8080);
    assert_eq!(app.configured_store_path(), Some("./data/crm"));
}

#[test]
fn app_fluent_builder_chain() {
    // Verify the builder pattern is fully chainable
    let app = App::new("Chain Test")
        .port(4000)
        .store_path("./test")
        .on("test_action", |_ctx| adapto_app::ActionResult::none());

    assert_eq!(app.title(), "Chain Test");
    assert_eq!(app.configured_port(), 4000);
}

// ---------------------------------------------------------------------------
// Resource registration tests
// ---------------------------------------------------------------------------

struct MockResource;

impl adapto_app::ResourceMeta for MockResource {
    fn collection_name() -> &'static str {
        "widgets"
    }
    fn field_names() -> &'static [&'static str] {
        &["name", "color", "weight"]
    }
    fn resource_label() -> &'static str {
        "Widget"
    }
    fn resource_label_plural() -> &'static str {
        "Widgets"
    }
    fn route_prefix() -> &'static str {
        "/widgets"
    }
    fn ensure_indexes(_store: &adapto_store::AdaptoStore) {}
}

#[test]
fn resource_registration_increments_count() {
    let app = App::new("Test").resource::<MockResource>();
    assert_eq!(app.resource_count(), 1);
}

#[test]
fn multiple_resources() {
    struct SecondResource;
    impl adapto_app::ResourceMeta for SecondResource {
        fn collection_name() -> &'static str { "gadgets" }
        fn field_names() -> &'static [&'static str] { &["model"] }
        fn resource_label() -> &'static str { "Gadget" }
        fn resource_label_plural() -> &'static str { "Gadgets" }
        fn route_prefix() -> &'static str { "/gadgets" }
        fn ensure_indexes(_store: &adapto_store::AdaptoStore) {}
    }

    let app = App::new("Multi")
        .resource::<MockResource>()
        .resource::<SecondResource>();
    assert_eq!(app.resource_count(), 2);
}

// ---------------------------------------------------------------------------
// Layout rendering tests
// ---------------------------------------------------------------------------

#[test]
fn layout_produces_valid_html_document() {
    let config = LayoutConfig {
        title: "Test App",
        nav_items: &[("Dashboard", "/", true), ("Settings", "/settings", false)],
        breadcrumbs: &[("Home", Some("/")), ("Current", None)],
        stats_html: "<div>42 items</div>",
        content_html: "<p>Hello, world.</p>",
        extra_css: ".custom { color: red; }",
    };
    let html = adapto_app::layout::render_layout(&config);

    // Document structure
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("<html lang=\"en\">"));
    assert!(html.contains("</html>"));

    // Title
    assert!(html.contains("<title>Test App</title>"));

    // CSS
    assert!(html.contains("--au-color-")); // tokens are present
    assert!(html.contains(".custom { color: red; }"));

    // Navigation
    assert!(html.contains("Dashboard"));
    assert!(html.contains("au-nav__item--active"));
    assert!(html.contains("Settings"));

    // Breadcrumbs
    assert!(html.contains("Home"));
    assert!(html.contains("Current"));

    // Content areas with correct IDs
    assert!(html.contains("id=\"app-content\""));
    assert!(html.contains("id=\"app-stats\""));
    assert!(html.contains("id=\"app-breadcrumb\""));

    // Actual content
    assert!(html.contains("<p>Hello, world.</p>"));
    assert!(html.contains("<div>42 items</div>"));
}

#[test]
fn layout_html_escapes_title() {
    let config = LayoutConfig {
        title: "App <script>",
        nav_items: &[],
        breadcrumbs: &[],
        stats_html: "",
        content_html: "",
        extra_css: "",
    };
    let html = adapto_app::layout::render_layout(&config);
    assert!(!html.contains("<title>App <script></title>"));
    assert!(html.contains("&lt;script&gt;"));
}

// ---------------------------------------------------------------------------
// Live.js content tests
// ---------------------------------------------------------------------------

#[test]
fn live_js_contains_websocket_client() {
    assert!(LIVE_JS.contains("new WebSocket(url)"));
}

#[test]
fn live_js_contains_event_dispatch() {
    assert!(LIVE_JS.contains("data-action"));
    assert!(LIVE_JS.contains("data-route"));
    assert!(LIVE_JS.contains("__adapto_navigate"));
}

#[test]
fn live_js_contains_reconnect_logic() {
    assert!(LIVE_JS.contains("reconnecting"));
    assert!(LIVE_JS.contains("connect()"));
}

#[test]
fn live_js_contains_search_debounce() {
    assert!(LIVE_JS.contains("searchTimer"));
    assert!(LIVE_JS.contains("setTimeout"));
}

#[test]
fn live_js_handles_patch_ops() {
    assert!(LIVE_JS.contains("replace_html"));
    assert!(LIVE_JS.contains("replace_text"));
}

// ---------------------------------------------------------------------------
// Views tests
// ---------------------------------------------------------------------------

#[test]
fn placeholder_view_renders() {
    let html = adapto_app::views::render_resource_placeholder("Customers", "/customers");
    assert!(html.contains("Customers"));
    assert!(html.contains("/customers"));
    assert!(html.contains("au-card"));
}

#[test]
fn empty_state_view_renders() {
    let html = adapto_app::views::render_empty_state("Products");
    assert!(html.contains("No Products yet."));
}

// ---------------------------------------------------------------------------
// ActionResult tests
// ---------------------------------------------------------------------------

#[test]
fn action_result_replace_html() {
    let result = adapto_app::ActionResult::replace_html("target", "<div>new</div>");
    assert_eq!(result.ops.len(), 1);
}

#[test]
fn action_result_none() {
    let result = adapto_app::ActionResult::none();
    assert!(result.ops.is_empty());
}
