//! # adapto_app
//!
//! The App builder for Adapto — replaces manual axum/WebSocket setup with a
//! declarative API. Register resources, add action handlers, and call `run()`.
//!
//! ```rust,no_run
//! use adapto_app::App;
//!
//! # async fn example() {
//! App::new("My App")
//!     .port(3000)
//!     .store_path("./data/myapp")
//!     .run()
//!     .await
//!     .unwrap();
//! # }
//! ```

pub mod handler;
pub mod layout;
pub mod views;

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, Query as AxumQuery, State};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;

pub use handler::{ActionContext, ActionHandler, ActionResult};
pub use layout::{LayoutConfig, LIVE_JS};

pub enum FallbackResponse {
    Html(String),
    Raw { body: String, content_type: &'static str },
    NotFound,
}

// ---------------------------------------------------------------------------
// PageResponse — return type for page handlers
// ---------------------------------------------------------------------------

/// Response from a page handler. Supports proper HTTP status codes.
pub enum PageResponse {
    /// 200 OK with HTML body.
    Ok(String),
    /// 404 Not Found.
    NotFound,
    /// 301 Permanent Redirect.
    Redirect(String),
}

impl From<String> for PageResponse {
    fn from(s: String) -> Self {
        PageResponse::Ok(s)
    }
}

impl From<&str> for PageResponse {
    fn from(s: &str) -> Self {
        PageResponse::Ok(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// LangConfig — trait for language-aware routing
// ---------------------------------------------------------------------------

/// Implement this trait on your language enum to use `localized_page()`.
pub trait LangConfig: Clone + Send + Sync + 'static {
    /// Language code (e.g., "ru", "kk", "en").
    fn code(&self) -> &str;
    /// URL prefix for this language (e.g., "", "/kz", "/en").
    fn prefix(&self) -> &str;
}

struct LanguageRoute {
    prefix: String,
    code: String,
}

// ---------------------------------------------------------------------------
// ResourceMeta trait
// ---------------------------------------------------------------------------

/// Trait that `#[derive(Resource)]` will implement. The App builder uses this
/// to auto-generate routes, views, and indexes.
///
/// Until the derive macro exists, implement this manually for each resource
/// struct to register it with the app.
pub trait ResourceMeta: Send + Sync + 'static {
    /// The collection name in the store (e.g., `"customers"`).
    fn collection_name() -> &'static str;

    /// The field names for this resource (e.g., `["name", "email", "company"]`).
    fn field_names() -> &'static [&'static str];

    /// Human-readable singular label (e.g., `"Customer"`).
    fn resource_label() -> &'static str;

    /// Human-readable plural label (e.g., `"Customers"`).
    fn resource_label_plural() -> &'static str;

    /// URL route prefix (e.g., `"/customers"`).
    fn route_prefix() -> &'static str;

    /// Ensure required indexes exist on the collection.
    fn ensure_indexes(store: &adapto_store::AdaptoStore);
}

// ---------------------------------------------------------------------------
// Resource registration
// ---------------------------------------------------------------------------

/// Metadata collected from a ResourceMeta implementation, stored in the builder
/// as type-erased data so the App can work with heterogeneous resources.
struct ResourceEntry {
    collection_name: String,
    label: String,
    label_plural: String,
    route_prefix: String,
    ensure_indexes: Box<dyn Fn(&adapto_store::AdaptoStore) + Send + Sync>,
}

// ---------------------------------------------------------------------------
// Custom route
// ---------------------------------------------------------------------------

/// A custom route registered with the App, consisting of a method + path
/// and a handler that receives the shared state.
struct CustomRoute {
    path: String,
    handler: RouteHandler,
}

/// Request context for route handlers — includes store, path params, query string, language.
pub struct RequestContext {
    pub state: Arc<handler::AppState>,
    pub params: HashMap<String, String>,
    pub query: String,
    pub request_path: String,
    pub lang_code: String,
    pub lang_prefix: String,
}

impl RequestContext {
    pub fn store(&self) -> &adapto_store::AdaptoStore {
        &self.state.store
    }

    pub fn param(&self, name: &str) -> &str {
        self.params.get(name).map(|s| s.as_str()).unwrap_or("")
    }

    pub fn path(&self) -> &str {
        &self.request_path
    }

    /// Language code for this request (e.g., "ru", "kk"). Empty if not using localized routes.
    pub fn lang_code(&self) -> &str {
        &self.lang_code
    }

    /// URL prefix for this request's language (e.g., "", "/kz"). Empty if not using localized routes.
    pub fn lang_prefix(&self) -> &str {
        &self.lang_prefix
    }
}

/// Supported route handlers — for now just GET with HTML response.
enum RouteHandler {
    Get(Arc<dyn Fn(Arc<handler::AppState>) -> String + Send + Sync + 'static>),
    RawGet(Arc<dyn Fn(Arc<handler::AppState>) -> String + Send + Sync + 'static>),
    RawGetParams {
        handler: Arc<dyn Fn(RequestContext) -> PageResponse + Send + Sync + 'static>,
        lang_code: String,
        lang_prefix: String,
    },
}

// ---------------------------------------------------------------------------
// App builder
// ---------------------------------------------------------------------------

/// The Adapto App builder. Configure your application declaratively,
/// then call `run()` to start serving.
pub struct App {
    title: String,
    port: u16,
    store_path: Option<String>,
    prebuilt_store: Option<adapto_store::AdaptoStore>,
    resources: Vec<ResourceEntry>,
    custom_routes: Vec<CustomRoute>,
    action_handlers:
        HashMap<String, Box<dyn Fn(&mut ActionContext<'_>) -> ActionResult + Send + Sync>>,
    index_page_content:
        Option<Arc<dyn Fn(Arc<handler::AppState>) -> String + Send + Sync + 'static>>,
    custom_fallback:
        Option<Arc<dyn Fn(String) -> FallbackResponse + Send + Sync + 'static>>,
    language_routes: Vec<LanguageRoute>,
}

impl App {
    /// Create a new App with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            port: 3000,
            store_path: None,
            prebuilt_store: None,
            resources: Vec::new(),
            custom_routes: Vec::new(),
            action_handlers: HashMap::new(),
            index_page_content: None,
            custom_fallback: None,
            language_routes: Vec::new(),
        }
    }

    /// Set the port to listen on. Defaults to 3000.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the store path for persistent storage.
    /// If not set, uses an in-memory store.
    pub fn store_path(mut self, path: impl Into<String>) -> Self {
        self.store_path = Some(path.into());
        self
    }

    /// Use a pre-built store instead of creating one in `run()`.
    /// This allows importing data before starting the server.
    pub fn store(mut self, store: adapto_store::AdaptoStore) -> Self {
        self.prebuilt_store = Some(store);
        self
    }

    /// Register a resource type with the app.
    ///
    /// This will:
    /// 1. Ensure the collection exists in the store
    /// 2. Call `ensure_indexes()` to set up required indexes
    /// 3. Register a default placeholder route at the resource's `route_prefix`
    pub fn resource<R: ResourceMeta>(mut self) -> Self {
        self.resources.push(ResourceEntry {
            collection_name: R::collection_name().to_string(),
            label: R::resource_label().to_string(),
            label_plural: R::resource_label_plural().to_string(),
            route_prefix: R::route_prefix().to_string(),
            ensure_indexes: Box::new(|store| R::ensure_indexes(store)),
        });
        self
    }

    /// Register a WebSocket action handler.
    ///
    /// When the client sends an event with `handler: "action_name"`,
    /// the registered function is called with the store and payload.
    pub fn on(
        mut self,
        action: impl Into<String>,
        handler: impl Fn(&mut ActionContext<'_>) -> ActionResult + Send + Sync + 'static,
    ) -> Self {
        self.action_handlers.insert(action.into(), Box::new(handler));
        self
    }

    /// Register a custom GET route that returns HTML.
    pub fn get_route(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(Arc<handler::AppState>) -> String + Send + Sync + 'static,
    ) -> Self {
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            handler: RouteHandler::Get(Arc::new(handler)),
        });
        self
    }

    /// Register a custom GET route that returns raw HTML (no layout wrapping).
    pub fn raw_get(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(Arc<handler::AppState>) -> String + Send + Sync + 'static,
    ) -> Self {
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            handler: RouteHandler::RawGet(Arc::new(handler)),
        });
        self
    }

    /// Register a GET route with path parameters, returning raw HTML.
    /// Path params use axum syntax: `/law/laws/:slug`
    ///
    /// Handler can return `String` (always 200) or `PageResponse` (200/404/301).
    pub fn page<R: Into<PageResponse> + 'static>(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> R + Send + Sync + 'static,
    ) -> Self {
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            handler: RouteHandler::RawGetParams {
                handler: Arc::new(move |ctx| handler(ctx).into()),
                lang_code: String::new(),
                lang_prefix: String::new(),
            },
        });
        self
    }

    /// Configure language prefixes for `localized_page()`.
    pub fn languages<L: LangConfig>(mut self, langs: Vec<L>) -> Self {
        self.language_routes = langs
            .iter()
            .map(|l| LanguageRoute {
                prefix: l.prefix().to_string(),
                code: l.code().to_string(),
            })
            .collect();
        self
    }

    /// Register a route for all configured languages.
    ///
    /// Automatically registers the route with each language prefix.
    /// The handler receives `RequestContext` with `lang_code` and `lang_prefix` populated.
    pub fn localized_page<R: Into<PageResponse> + 'static>(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> R + Send + Sync + 'static,
    ) -> Self {
        let path = path.into();
        let handler = Arc::new(move |ctx: RequestContext| -> PageResponse { handler(ctx).into() });
        for lang in &self.language_routes {
            let full_path = format!("{}{}", lang.prefix, path);
            self.custom_routes.push(CustomRoute {
                path: full_path,
                handler: RouteHandler::RawGetParams {
                    handler: handler.clone(),
                    lang_code: lang.code.clone(),
                    lang_prefix: lang.prefix.clone(),
                },
            });
        }
        self
    }

    /// Set a custom index page renderer. If not set, the index page
    /// shows a welcome message listing registered resources.
    pub fn index_page(
        mut self,
        renderer: impl Fn(Arc<handler::AppState>) -> String + Send + Sync + 'static,
    ) -> Self {
        self.index_page_content = Some(Arc::new(renderer));
        self
    }

    /// Set a custom fallback handler for unmatched routes.
    /// The closure receives the request path and returns `Some(html)` to serve
    /// or `None` to return 404.
    pub fn fallback_fn(
        mut self,
        handler: impl Fn(String) -> FallbackResponse + Send + Sync + 'static,
    ) -> Self {
        self.custom_fallback = Some(Arc::new(handler));
        self
    }

    /// The configured application title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The configured port.
    pub fn configured_port(&self) -> u16 {
        self.port
    }

    /// The configured store path (if any).
    pub fn configured_store_path(&self) -> Option<&str> {
        self.store_path.as_deref()
    }

    /// The number of registered resources.
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    /// Start the application.
    ///
    /// This will:
    /// 1. Open the AdaptoStore (persistent or in-memory)
    /// 2. Call `ensure_indexes()` for each registered resource
    /// 3. Build an axum Router with all routes
    /// 4. Serve `/_adapto/live.js` for the client runtime
    /// 5. Set up the WebSocket endpoint at `/ws`
    /// 6. Bind to the configured port and begin serving
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        tracing_subscriber::fmt::init();

        // 1. Open store (use prebuilt if provided)
        let store = if let Some(s) = self.prebuilt_store {
            s
        } else {
            adapto_store::AdaptoStore::open(self.store_path.as_deref())?
        };

        // 2. Ensure indexes for each resource
        for resource in &self.resources {
            (resource.ensure_indexes)(&store);
            tracing::info!(
                collection = resource.collection_name,
                label = resource.label,
                "Initialized resource: {}",
                resource.label_plural
            );
        }

        // 3. Build action handler map
        let handlers: handler::ActionHandlerMap = Arc::new(
            self.action_handlers
                .into_iter()
                .map(|(k, v)| (k, v as ActionHandler))
                .collect(),
        );

        let state = Arc::new(handler::AppState {
            store,
            handlers,
            title: self.title.clone(),
        });

        // 4. Build router
        let mut router = Router::new();

        // Serve live.js
        router = router.route("/_adapto/live.js", get(serve_live_js));

        // WebSocket endpoint
        router = router.route("/ws", get(handle_ws));

        // Resource placeholder routes
        for resource in &self.resources {
            let label_plural = resource.label_plural.clone();
            let route_prefix = resource.route_prefix.clone();
            let title = self.title.clone();

            let rp = route_prefix.clone();
            router = router.route(
                &route_prefix,
                get(move |State(_state): State<Arc<handler::AppState>>| {
                    let content =
                        views::render_resource_placeholder(&label_plural, &rp);
                    let html = layout::render_layout(&LayoutConfig {
                        title: &title,
                        nav_items: &[],
                        breadcrumbs: &[],
                        stats_html: "",
                        content_html: &content,
                        extra_css: "",
                    });
                    async move { Html(html) }
                }),
            );
        }

        // Custom routes
        let has_custom_index = self.custom_routes.iter().any(|r| r.path == "/");
        for custom in self.custom_routes {
            let title = self.title.clone();
            match custom.handler {
                RouteHandler::Get(handler) => {
                    router = router.route(
                        &custom.path,
                        get(
                            move |State(state): State<Arc<handler::AppState>>| {
                                let content = handler(state.clone());
                                let html = layout::render_layout(&LayoutConfig {
                                    title: &title,
                                    nav_items: &[],
                                    breadcrumbs: &[],
                                    stats_html: "",
                                    content_html: &content,
                                    extra_css: "",
                                });
                                async move { Html(html) }
                            },
                        ),
                    );
                }
                RouteHandler::RawGet(handler) => {
                    router = router.route(
                        &custom.path,
                        get(
                            move |State(state): State<Arc<handler::AppState>>| {
                                let html = handler(state.clone());
                                async move { Html(html) }
                            },
                        ),
                    );
                }
                RouteHandler::RawGetParams { handler, lang_code, lang_prefix } => {
                    router = router.route(
                        &custom.path,
                        get(
                            move |State(state): State<Arc<handler::AppState>>,
                                  Path(params): Path<HashMap<String, String>>,
                                  AxumQuery(query_map): AxumQuery<HashMap<String, String>>,
                                  req_uri: axum::http::Uri| {
                                let query_str = query_map
                                    .iter()
                                    .map(|(k, v)| format!("{k}={v}"))
                                    .collect::<Vec<_>>()
                                    .join("&");
                                let ctx = RequestContext {
                                    state: state.clone(),
                                    params,
                                    query: query_str,
                                    request_path: req_uri.path().to_string(),
                                    lang_code: lang_code.clone(),
                                    lang_prefix: lang_prefix.clone(),
                                };
                                let response = handler(ctx);
                                async move {
                                    match response {
                                        PageResponse::Ok(html) => Html(html).into_response(),
                                        PageResponse::NotFound => {
                                            (axum::http::StatusCode::NOT_FOUND, Html("<h1>Not Found</h1>".to_string())).into_response()
                                        }
                                        PageResponse::Redirect(url) => {
                                            axum::response::Redirect::permanent(&url).into_response()
                                        }
                                    }
                                }
                            },
                        ),
                    );
                }
            }
        }

        // Index route (skip if a custom route already registered "/")
        let title_for_index = self.title.clone();
        let resource_info: Vec<(String, String)> = self
            .resources
            .iter()
            .map(|r| (r.label_plural.clone(), r.route_prefix.clone()))
            .collect();

        if !has_custom_index {
            if let Some(index_renderer) = self.index_page_content {
                router = router.route(
                    "/",
                    get(
                        move |State(state): State<Arc<handler::AppState>>| {
                            let content = index_renderer(state.clone());
                            let html = layout::render_layout(&LayoutConfig {
                                title: &title_for_index,
                                nav_items: &[],
                                breadcrumbs: &[],
                                stats_html: "",
                                content_html: &content,
                                extra_css: "",
                            });
                            async move { Html(html) }
                        },
                    ),
                );
            } else {
                router = router.route(
                    "/",
                    get(
                        move |State(_state): State<Arc<handler::AppState>>| {
                            let content = render_default_index(&title_for_index, &resource_info);
                            let html = layout::render_layout(&LayoutConfig {
                                title: &title_for_index,
                                nav_items: &[],
                                breadcrumbs: &[],
                                stats_html: "",
                                content_html: &content,
                                extra_css: "",
                            });
                            async move { Html(html) }
                        },
                    ),
                );
            }
        }

        let router = router.with_state(state);

        // Normalize trailing slashes + custom fallback
        let custom_fb = self.custom_fallback.clone();
        let router = router.fallback(move |req: axum::http::Request<axum::body::Body>| {
            let custom_fb = custom_fb.clone();
            async move {
                let path = req.uri().path();
                if path.len() > 1 && path.ends_with('/') {
                    let trimmed = path.trim_end_matches('/');
                    let new_uri = if let Some(q) = req.uri().query() {
                        format!("{trimmed}?{q}")
                    } else {
                        trimmed.to_string()
                    };
                    return axum::response::Redirect::permanent(&new_uri).into_response();
                }
                if let Some(ref fb) = custom_fb {
                    match fb(path.to_string()) {
                        FallbackResponse::Html(html) => return Html(html).into_response(),
                        FallbackResponse::Raw { body, content_type } => {
                            return (
                                [(axum::http::header::CONTENT_TYPE, content_type)],
                                body,
                            ).into_response();
                        }
                        FallbackResponse::NotFound => {}
                    }
                }
                (axum::http::StatusCode::NOT_FOUND, "Not Found").into_response()
            }
        });

        // 5. Bind and serve
        let addr = format!("127.0.0.1:{}", self.port);
        println!();
        println!("  {} running at http://{}", self.title, addr);
        if let Some(ref path) = self.store_path {
            println!("  Database: {}/store.wal", path);
        } else {
            println!("  Database: in-memory");
        }
        println!("  Press Ctrl+C to stop.");
        println!();

        let listener = TcpListener::bind(&addr).await?;
        axum::serve(listener, router).await?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Built-in route handlers
// ---------------------------------------------------------------------------

/// Serve the embedded live.js file with the correct content type.
async fn serve_live_js() -> Response {
    (
        [("content-type", "application/javascript; charset=utf-8")],
        LIVE_JS,
    )
        .into_response()
}

/// Handle WebSocket upgrade requests.
async fn handle_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<handler::AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handler::ws_event_loop(socket, state))
}

/// Render the default index page listing registered resources.
fn render_default_index(title: &str, resources: &[(String, String)]) -> String {
    let title_esc = adapto_ui::html_escape(title);

    if resources.is_empty() {
        return format!(
            r#"<div class="au-card au-card--flat" style="text-align:center;padding:var(--au-space-10)">
  <h1 style="font-size:var(--au-text-3xl);font-weight:var(--au-weight-bold);margin:0 0 var(--au-space-2)">{title_esc}</h1>
  <p style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary);margin:0">
    No resources registered yet. Use <code>.resource::&lt;MyType&gt;()</code> to add one.
  </p>
</div>"#,
        );
    }

    let links: String = resources
        .iter()
        .map(|(label, prefix)| {
            format!(
                r#"<a href="{prefix}" data-route="{prefix}" class="au-btn au-btn--secondary" style="margin:var(--au-space-1)">{label}</a>"#,
                prefix = adapto_ui::html_escape(prefix),
                label = adapto_ui::html_escape(label),
            )
        })
        .collect();

    format!(
        r#"<div class="au-card au-card--flat" style="text-align:center;padding:var(--au-space-10)">
  <h1 style="font-size:var(--au-text-3xl);font-weight:var(--au-weight-bold);margin:0 0 var(--au-space-3)">{title_esc}</h1>
  <p style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary);margin:0 0 var(--au-space-5)">
    {count} resource{s} registered
  </p>
  <div class="au-flex au-justify-center au-gap-3" style="flex-wrap:wrap">{links}</div>
</div>"#,
        count = resources.len(),
        s = if resources.len() == 1 { "" } else { "s" },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_builder_defaults() {
        let app = App::new("Test");
        assert_eq!(app.title(), "Test");
        assert_eq!(app.configured_port(), 3000);
        assert!(app.configured_store_path().is_none());
        assert_eq!(app.resource_count(), 0);
    }

    #[test]
    fn app_builder_configuration() {
        let app = App::new("My App")
            .port(8080)
            .store_path("./data/test");

        assert_eq!(app.title(), "My App");
        assert_eq!(app.configured_port(), 8080);
        assert_eq!(app.configured_store_path(), Some("./data/test"));
    }

    #[test]
    fn app_builder_with_resource() {
        struct TestResource;
        impl ResourceMeta for TestResource {
            fn collection_name() -> &'static str { "tests" }
            fn field_names() -> &'static [&'static str] { &["name"] }
            fn resource_label() -> &'static str { "Test" }
            fn resource_label_plural() -> &'static str { "Tests" }
            fn route_prefix() -> &'static str { "/tests" }
            fn ensure_indexes(_store: &adapto_store::AdaptoStore) {}
        }

        let app = App::new("Test App").resource::<TestResource>();
        assert_eq!(app.resource_count(), 1);
    }

    #[test]
    fn live_js_is_embedded() {
        assert!(!LIVE_JS.is_empty());
        assert!(LIVE_JS.contains("WebSocket"));
        assert!(LIVE_JS.contains("__adapto_navigate"));
        assert!(LIVE_JS.contains("data-action"));
        assert!(LIVE_JS.contains("data-route"));
    }

    #[test]
    fn default_index_no_resources() {
        let html = render_default_index("My App", &[]);
        assert!(html.contains("My App"));
        assert!(html.contains("No resources registered"));
    }

    #[test]
    fn default_index_with_resources() {
        let resources = vec![
            ("Customers".to_string(), "/customers".to_string()),
            ("Orders".to_string(), "/orders".to_string()),
        ];
        let html = render_default_index("CRM", &resources);
        assert!(html.contains("CRM"));
        assert!(html.contains("2 resources registered"));
        assert!(html.contains("/customers"));
        assert!(html.contains("/orders"));
    }
}
