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

use axum::body::Bytes;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{self, MethodRouter};
use axum::Router;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

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
    /// 200 OK with JSON body.
    Json(Value),
    /// 400 Bad Request with message.
    BadRequest(String),
    /// 403 Forbidden with message.
    Forbidden(String),
    /// 500 Internal Server Error with message.
    InternalError(String),
    /// Custom response with arbitrary status, content type, body, and headers.
    Custom {
        status: u16,
        body: String,
        content_type: String,
        headers: Vec<(String, String)>,
    },
}

impl PageResponse {
    /// Create a JSON response from a serializable value.
    pub fn json<T: Serialize>(value: &T) -> Self {
        match serde_json::to_value(value) {
            Ok(v) => PageResponse::Json(v),
            Err(e) => PageResponse::InternalError(format!("JSON serialization error: {e}")),
        }
    }

    /// Create a custom response with status code and body.
    pub fn with_status(status: u16, body: impl Into<String>) -> Self {
        PageResponse::Custom {
            status,
            body: body.into(),
            content_type: "text/html; charset=utf-8".to_string(),
            headers: Vec::new(),
        }
    }

    /// Create a custom response with status, body, and content type.
    pub fn raw(status: u16, body: impl Into<String>, content_type: impl Into<String>) -> Self {
        PageResponse::Custom {
            status,
            body: body.into(),
            content_type: content_type.into(),
            headers: Vec::new(),
        }
    }
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

fn page_response_to_axum(response: PageResponse) -> Response {
    match response {
        PageResponse::Ok(html) => Html(html).into_response(),
        PageResponse::NotFound => {
            (StatusCode::NOT_FOUND, Html("<h1>Not Found</h1>".to_string())).into_response()
        }
        PageResponse::Redirect(url) => {
            axum::response::Redirect::permanent(&url).into_response()
        }
        PageResponse::Json(value) => {
            (StatusCode::OK, axum::Json(value)).into_response()
        }
        PageResponse::BadRequest(msg) => {
            (StatusCode::BAD_REQUEST, Html(msg)).into_response()
        }
        PageResponse::Forbidden(msg) => {
            (StatusCode::FORBIDDEN, Html(msg)).into_response()
        }
        PageResponse::InternalError(msg) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Html(msg)).into_response()
        }
        PageResponse::Custom { status, body, content_type, headers } => {
            let status = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let mut resp = (
                status,
                [(axum::http::header::CONTENT_TYPE, content_type)],
                body,
            ).into_response();
            for (k, v) in headers {
                if let (Ok(name), Ok(val)) = (
                    axum::http::header::HeaderName::from_bytes(k.as_bytes()),
                    HeaderValue::from_str(&v),
                ) {
                    resp.headers_mut().insert(name, val);
                }
            }
            resp
        }
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
pub trait ResourceMeta: Send + Sync + 'static {
    fn collection_name() -> &'static str;
    fn field_names() -> &'static [&'static str];
    fn resource_label() -> &'static str;
    fn resource_label_plural() -> &'static str;
    fn route_prefix() -> &'static str;
    fn ensure_indexes(store: &adapto_store::AdaptoStore);
}

// ---------------------------------------------------------------------------
// Resource registration
// ---------------------------------------------------------------------------

struct ResourceEntry {
    collection_name: String,
    label: String,
    label_plural: String,
    route_prefix: String,
    ensure_indexes: Box<dyn Fn(&adapto_store::AdaptoStore) + Send + Sync>,
}

// ---------------------------------------------------------------------------
// RequestContext — enriched with body, headers, method
// ---------------------------------------------------------------------------

/// Request context for route handlers — includes store, path params, query,
/// headers, body, method, and language info.
pub struct RequestContext {
    pub state: Arc<handler::AppState>,
    pub params: HashMap<String, String>,
    pub query: String,
    pub request_path: String,
    pub lang_code: String,
    pub lang_prefix: String,
    pub method: Method,
    pub headers: HeaderMap,
    pub body: Bytes,
    pub remote_addr: Option<SocketAddr>,
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

    /// HTTP method (GET, POST, PUT, DELETE, etc.).
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Get a request header value by name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    /// Get all request headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Parse the request body as JSON into a typed value.
    pub fn body_json<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }

    /// Get raw request body as bytes.
    pub fn body_bytes(&self) -> &[u8] {
        &self.body
    }

    /// Get request body as string.
    pub fn body_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.body)
    }

    /// Get a cookie value by name.
    pub fn cookie(&self, name: &str) -> Option<&str> {
        self.headers
            .get(axum::http::header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|cookies| {
                cookies.split(';').find_map(|pair| {
                    let pair = pair.trim();
                    let (k, v) = pair.split_once('=')?;
                    if k.trim() == name { Some(v.trim()) } else { None }
                })
            })
    }

    /// Client IP address (from socket, not X-Forwarded-For).
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    /// Parse query string into key-value pairs.
    pub fn query_pairs(&self) -> Vec<(String, String)> {
        if self.query.is_empty() {
            return Vec::new();
        }
        self.query
            .split('&')
            .filter_map(|pair| {
                let (k, v) = pair.split_once('=')?;
                Some((
                    urlencoding_decode(k),
                    urlencoding_decode(v),
                ))
            })
            .collect()
    }

    /// Get a single query parameter by name.
    pub fn query_param(&self, name: &str) -> Option<String> {
        self.query_pairs()
            .into_iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v)
    }
}

fn urlencoding_decode(s: &str) -> String {
    let s = s.replace('+', " ");
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else {
            result.push(c);
        }
    }
    result
}

// ---------------------------------------------------------------------------
// HTTP Method enum for route registration
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

// ---------------------------------------------------------------------------
// BoxHandler — unified async handler type
// ---------------------------------------------------------------------------

type BoxHandler = Arc<
    dyn Fn(RequestContext) -> Pin<Box<dyn Future<Output = PageResponse> + Send>>
        + Send
        + Sync
        + 'static,
>;

// ---------------------------------------------------------------------------
// Custom route
// ---------------------------------------------------------------------------

struct CustomRoute {
    path: String,
    method: HttpMethod,
    handler: BoxHandler,
    lang_code: String,
    lang_prefix: String,
    wrap_layout: bool,
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
    bind_addr: String,
    resources: Vec<ResourceEntry>,
    custom_routes: Vec<CustomRoute>,
    action_handlers:
        HashMap<String, Box<dyn Fn(&mut ActionContext<'_>) -> ActionResult + Send + Sync>>,
    index_page_content: Option<BoxHandler>,
    custom_fallback:
        Option<Arc<dyn Fn(String) -> FallbackResponse + Send + Sync + 'static>>,
    language_routes: Vec<LanguageRoute>,
    static_dirs: Vec<(String, String)>,
    layers: Vec<Box<dyn FnOnce(Router) -> Router + Send>>,
    shutdown_hooks: Vec<Box<dyn FnOnce() + Send>>,
    health_path: Option<String>,
    error_handler: Option<Arc<dyn Fn(StatusCode, String) -> String + Send + Sync>>,
}

impl App {
    /// Create a new App with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            port: 3000,
            store_path: None,
            prebuilt_store: None,
            bind_addr: "0.0.0.0".to_string(),
            resources: Vec::new(),
            custom_routes: Vec::new(),
            action_handlers: HashMap::new(),
            index_page_content: None,
            custom_fallback: None,
            language_routes: Vec::new(),
            static_dirs: Vec::new(),
            layers: Vec::new(),
            shutdown_hooks: Vec::new(),
            health_path: None,
            error_handler: None,
        }
    }

    /// Set the port to listen on. Defaults to 3000.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the store path for persistent storage.
    pub fn store_path(mut self, path: impl Into<String>) -> Self {
        self.store_path = Some(path.into());
        self
    }

    /// Use a pre-built store instead of creating one in `run()`.
    pub fn store(mut self, store: adapto_store::AdaptoStore) -> Self {
        self.prebuilt_store = Some(store);
        self
    }

    /// Set the bind address. Defaults to `"0.0.0.0"`.
    pub fn bind(mut self, addr: impl Into<String>) -> Self {
        self.bind_addr = addr.into();
        self
    }

    /// Apply configuration from environment variables.
    ///
    /// Reads: `PORT`, `BIND_ADDR`, `STORE_PATH`.
    /// Only overrides values not already set explicitly via builder methods.
    pub fn from_env(mut self) -> Self {
        if let Ok(port) = std::env::var("PORT") {
            if let Ok(p) = port.parse::<u16>() {
                self.port = p;
            }
        }
        if let Ok(addr) = std::env::var("BIND_ADDR") {
            self.bind_addr = addr;
        }
        if let Ok(path) = std::env::var("STORE_PATH") {
            if self.store_path.is_none() {
                self.store_path = Some(path);
            }
        }
        self
    }

    /// Register a resource type with the app.
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
    pub fn on(
        mut self,
        action: impl Into<String>,
        handler: impl Fn(&mut ActionContext<'_>) -> ActionResult + Send + Sync + 'static,
    ) -> Self {
        self.action_handlers.insert(action.into(), Box::new(handler));
        self
    }

    // -----------------------------------------------------------------------
    // Sync route registration (backward-compatible)
    // -----------------------------------------------------------------------

    /// Register a GET route that returns raw HTML (no layout).
    /// Handler can return `String` (always 200) or `PageResponse` (200/404/301/JSON/etc).
    pub fn page<R: Into<PageResponse> + 'static>(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> R + Send + Sync + 'static,
    ) -> Self {
        let handler = Arc::new(handler);
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            method: HttpMethod::Get,
            handler: Arc::new(move |ctx| {
                let h = handler.clone();
                Box::pin(async move { h(ctx).into() })
            }),
            lang_code: String::new(),
            lang_prefix: String::new(),
            wrap_layout: false,
        });
        self
    }

    /// Register a GET route with layout wrapping.
    pub fn get_route(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> String + Send + Sync + 'static,
    ) -> Self {
        let handler = Arc::new(handler);
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            method: HttpMethod::Get,
            handler: Arc::new(move |ctx| {
                let h = handler.clone();
                Box::pin(async move { PageResponse::Ok(h(ctx)) })
            }),
            lang_code: String::new(),
            lang_prefix: String::new(),
            wrap_layout: true,
        });
        self
    }

    /// Register a GET route returning raw HTML (no layout, no path params).
    pub fn raw_get(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(Arc<handler::AppState>) -> String + Send + Sync + 'static,
    ) -> Self {
        let handler = Arc::new(handler);
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            method: HttpMethod::Get,
            handler: Arc::new(move |ctx| {
                let h = handler.clone();
                Box::pin(async move { PageResponse::Ok(h(ctx.state)) })
            }),
            lang_code: String::new(),
            lang_prefix: String::new(),
            wrap_layout: false,
        });
        self
    }

    /// Register a POST route.
    pub fn post<R: Into<PageResponse> + 'static>(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> R + Send + Sync + 'static,
    ) -> Self {
        let handler = Arc::new(handler);
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            method: HttpMethod::Post,
            handler: Arc::new(move |ctx| {
                let h = handler.clone();
                Box::pin(async move { h(ctx).into() })
            }),
            lang_code: String::new(),
            lang_prefix: String::new(),
            wrap_layout: false,
        });
        self
    }

    /// Register a PUT route.
    pub fn put<R: Into<PageResponse> + 'static>(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> R + Send + Sync + 'static,
    ) -> Self {
        let handler = Arc::new(handler);
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            method: HttpMethod::Put,
            handler: Arc::new(move |ctx| {
                let h = handler.clone();
                Box::pin(async move { h(ctx).into() })
            }),
            lang_code: String::new(),
            lang_prefix: String::new(),
            wrap_layout: false,
        });
        self
    }

    /// Register a DELETE route.
    pub fn delete<R: Into<PageResponse> + 'static>(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> R + Send + Sync + 'static,
    ) -> Self {
        let handler = Arc::new(handler);
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            method: HttpMethod::Delete,
            handler: Arc::new(move |ctx| {
                let h = handler.clone();
                Box::pin(async move { h(ctx).into() })
            }),
            lang_code: String::new(),
            lang_prefix: String::new(),
            wrap_layout: false,
        });
        self
    }

    /// Register a PATCH route.
    pub fn patch<R: Into<PageResponse> + 'static>(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> R + Send + Sync + 'static,
    ) -> Self {
        let handler = Arc::new(handler);
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            method: HttpMethod::Patch,
            handler: Arc::new(move |ctx| {
                let h = handler.clone();
                Box::pin(async move { h(ctx).into() })
            }),
            lang_code: String::new(),
            lang_prefix: String::new(),
            wrap_layout: false,
        });
        self
    }

    // -----------------------------------------------------------------------
    // Async route registration
    // -----------------------------------------------------------------------

    /// Register an async GET route.
    pub fn async_page<Fut>(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        Fut: Future<Output = PageResponse> + Send + 'static,
    {
        let handler = Arc::new(handler);
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            method: HttpMethod::Get,
            handler: Arc::new(move |ctx| {
                let h = handler.clone();
                Box::pin(h(ctx))
            }),
            lang_code: String::new(),
            lang_prefix: String::new(),
            wrap_layout: false,
        });
        self
    }

    /// Register an async POST route.
    pub fn async_post<Fut>(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        Fut: Future<Output = PageResponse> + Send + 'static,
    {
        let handler = Arc::new(handler);
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            method: HttpMethod::Post,
            handler: Arc::new(move |ctx| {
                let h = handler.clone();
                Box::pin(h(ctx))
            }),
            lang_code: String::new(),
            lang_prefix: String::new(),
            wrap_layout: false,
        });
        self
    }

    /// Register an async PUT route.
    pub fn async_put<Fut>(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        Fut: Future<Output = PageResponse> + Send + 'static,
    {
        let handler = Arc::new(handler);
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            method: HttpMethod::Put,
            handler: Arc::new(move |ctx| {
                let h = handler.clone();
                Box::pin(h(ctx))
            }),
            lang_code: String::new(),
            lang_prefix: String::new(),
            wrap_layout: false,
        });
        self
    }

    /// Register an async DELETE route.
    pub fn async_delete<Fut>(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        Fut: Future<Output = PageResponse> + Send + 'static,
    {
        let handler = Arc::new(handler);
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            method: HttpMethod::Delete,
            handler: Arc::new(move |ctx| {
                let h = handler.clone();
                Box::pin(h(ctx))
            }),
            lang_code: String::new(),
            lang_prefix: String::new(),
            wrap_layout: false,
        });
        self
    }

    /// Register an async PATCH route.
    pub fn async_patch<Fut>(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        Fut: Future<Output = PageResponse> + Send + 'static,
    {
        let handler = Arc::new(handler);
        self.custom_routes.push(CustomRoute {
            path: path.into(),
            method: HttpMethod::Patch,
            handler: Arc::new(move |ctx| {
                let h = handler.clone();
                Box::pin(h(ctx))
            }),
            lang_code: String::new(),
            lang_prefix: String::new(),
            wrap_layout: false,
        });
        self
    }

    // -----------------------------------------------------------------------
    // Localized routes
    // -----------------------------------------------------------------------

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

    /// Register a GET route for all configured languages.
    pub fn localized_page<R: Into<PageResponse> + 'static>(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> R + Send + Sync + 'static,
    ) -> Self {
        let path = path.into();
        if self.language_routes.is_empty() {
            panic!(
                "localized_page(\"{}\") called but no languages configured. Call .languages() first.",
                path
            );
        }
        let handler: Arc<dyn Fn(RequestContext) -> PageResponse + Send + Sync> =
            Arc::new(move |ctx: RequestContext| -> PageResponse { handler(ctx).into() });
        for lang in &self.language_routes {
            let full_path = format!("{}{}", lang.prefix, path);
            let h = handler.clone();
            self.custom_routes.push(CustomRoute {
                path: full_path,
                method: HttpMethod::Get,
                handler: Arc::new(move |ctx| {
                    let h = h.clone();
                    Box::pin(async move { h(ctx) })
                }),
                lang_code: lang.code.clone(),
                lang_prefix: lang.prefix.clone(),
                wrap_layout: false,
            });
        }
        self
    }

    /// Register a POST route for all configured languages.
    pub fn localized_post<R: Into<PageResponse> + 'static>(
        mut self,
        path: impl Into<String>,
        handler: impl Fn(RequestContext) -> R + Send + Sync + 'static,
    ) -> Self {
        let path = path.into();
        if self.language_routes.is_empty() {
            panic!(
                "localized_post(\"{}\") called but no languages configured. Call .languages() first.",
                path
            );
        }
        let handler: Arc<dyn Fn(RequestContext) -> PageResponse + Send + Sync> =
            Arc::new(move |ctx: RequestContext| -> PageResponse { handler(ctx).into() });
        for lang in &self.language_routes {
            let full_path = format!("{}{}", lang.prefix, path);
            let h = handler.clone();
            self.custom_routes.push(CustomRoute {
                path: full_path,
                method: HttpMethod::Post,
                handler: Arc::new(move |ctx| {
                    let h = h.clone();
                    Box::pin(async move { h(ctx) })
                }),
                lang_code: lang.code.clone(),
                lang_prefix: lang.prefix.clone(),
                wrap_layout: false,
            });
        }
        self
    }

    // -----------------------------------------------------------------------
    // Index page
    // -----------------------------------------------------------------------

    /// Set a custom index page renderer.
    pub fn index_page<R: Into<PageResponse> + 'static>(
        mut self,
        renderer: impl Fn(RequestContext) -> R + Send + Sync + 'static,
    ) -> Self {
        let renderer = Arc::new(renderer);
        self.index_page_content = Some(Arc::new(move |ctx| {
            let r = renderer.clone();
            Box::pin(async move { r(ctx).into() })
        }));
        self
    }

    /// Set a custom fallback handler for unmatched routes.
    pub fn fallback_fn(
        mut self,
        handler: impl Fn(String) -> FallbackResponse + Send + Sync + 'static,
    ) -> Self {
        self.custom_fallback = Some(Arc::new(handler));
        self
    }

    // -----------------------------------------------------------------------
    // Static files
    // -----------------------------------------------------------------------

    /// Serve static files from a directory.
    /// Example: `.static_dir("/static", "./public")` serves `./public/file.css` at `/static/file.css`.
    pub fn static_dir(mut self, url_path: impl Into<String>, dir: impl Into<String>) -> Self {
        self.static_dirs.push((url_path.into(), dir.into()));
        self
    }

    // -----------------------------------------------------------------------
    // Middleware
    // -----------------------------------------------------------------------

    /// Add a middleware function that transforms the Router.
    /// Use this to add tower layers, CORS, compression, etc.
    ///
    /// Example:
    /// ```rust,no_run
    /// use tower_http::cors::CorsLayer;
    /// # use adapto_app::App;
    /// App::new("My App")
    ///     .with_middleware(|router| router.layer(CorsLayer::permissive()));
    /// ```
    pub fn with_middleware(mut self, f: impl FnOnce(Router) -> Router + Send + 'static) -> Self {
        self.layers.push(Box::new(f));
        self
    }

    // -----------------------------------------------------------------------
    // Graceful shutdown
    // -----------------------------------------------------------------------

    /// Register a function to run on graceful shutdown.
    pub fn on_shutdown(mut self, hook: impl FnOnce() + Send + 'static) -> Self {
        self.shutdown_hooks.push(Box::new(hook));
        self
    }

    /// Add a health check endpoint (returns 200 "ok").
    pub fn health_check(mut self, path: impl Into<String>) -> Self {
        self.health_path = Some(path.into());
        self
    }

    // -----------------------------------------------------------------------
    // Error handling
    // -----------------------------------------------------------------------

    /// Set a custom error page renderer for HTTP error responses.
    /// Receives the status code and default message, returns HTML.
    pub fn error_handler(
        mut self,
        handler: impl Fn(StatusCode, String) -> String + Send + Sync + 'static,
    ) -> Self {
        self.error_handler = Some(Arc::new(handler));
        self
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn configured_port(&self) -> u16 {
        self.port
    }

    pub fn configured_store_path(&self) -> Option<&str> {
        self.store_path.as_deref()
    }

    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    // -----------------------------------------------------------------------
    // Build router (extracted for test_client)
    // -----------------------------------------------------------------------

    /// Build the configured Router without binding to a TCP listener.
    /// Useful for testing with `axum::extract::connect_info::MockConnectInfo`.
    pub fn build(self) -> Result<(Router, Vec<Box<dyn FnOnce() + Send>>), Box<dyn std::error::Error>> {
        let _ = tracing_subscriber::fmt::try_init();

        let store = if let Some(s) = self.prebuilt_store {
            s
        } else {
            adapto_store::AdaptoStore::open(self.store_path.as_deref())?
        };

        for resource in &self.resources {
            (resource.ensure_indexes)(&store);
            tracing::info!(
                collection = resource.collection_name,
                label = resource.label,
                "Initialized resource: {}",
                resource.label_plural
            );
        }

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

        let mut router = Router::new();

        // Serve live.js
        router = router.route("/_adapto/live.js", routing::get(serve_live_js));

        // WebSocket endpoint
        router = router.route("/ws", routing::get(handle_ws));

        // Health check
        if let Some(ref health_path) = self.health_path {
            router = router.route(health_path, routing::get(|| async { "ok" }));
        }

        // Resource placeholder routes
        for resource in &self.resources {
            let label_plural = resource.label_plural.clone();
            let route_prefix = resource.route_prefix.clone();
            let title = self.title.clone();

            let rp = route_prefix.clone();
            router = router.route(
                &route_prefix,
                routing::get(move |State(_state): State<Arc<handler::AppState>>| {
                    let content = views::render_resource_placeholder(&label_plural, &rp);
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

        // Custom routes — group by path, merge methods
        let has_custom_index = self.custom_routes.iter().any(|r| r.path == "/");
        let title = self.title.clone();
        let error_handler = self.error_handler.clone();

        for custom in self.custom_routes {
            let handler = custom.handler.clone();
            let lang_code = custom.lang_code.clone();
            let lang_prefix = custom.lang_prefix.clone();
            let wrap_layout = custom.wrap_layout;
            let title_clone = title.clone();
            let err_handler = error_handler.clone();

            let axum_handler = move |
                State(state): State<Arc<handler::AppState>>,
                method: Method,
                uri: Uri,
                headers: HeaderMap,
                params_opt: Option<Path<HashMap<String, String>>>,
                body: Bytes,
            | {
                let handler = handler.clone();
                let lang_code = lang_code.clone();
                let lang_prefix = lang_prefix.clone();
                let title_clone = title_clone.clone();
                let err_handler = err_handler.clone();
                async move {
                    let params = params_opt.map(|p| p.0).unwrap_or_default();
                    let query_str = uri.query().unwrap_or("").to_string();
                    let ctx = RequestContext {
                        state: state.clone(),
                        params,
                        query: query_str,
                        request_path: uri.path().to_string(),
                        lang_code,
                        lang_prefix,
                        method,
                        headers,
                        body,
                        remote_addr: None,
                    };
                    let response = handler(ctx).await;

                    if wrap_layout {
                        if let PageResponse::Ok(ref content) = response {
                            let html = layout::render_layout(&LayoutConfig {
                                title: &title_clone,
                                nav_items: &[],
                                breadcrumbs: &[],
                                stats_html: "",
                                content_html: content,
                                extra_css: "",
                            });
                            return Html(html).into_response();
                        }
                    }

                    let mut resp = page_response_to_axum(response);

                    if let Some(ref eh) = err_handler {
                        let status = resp.status();
                        if status.is_client_error() || status.is_server_error() {
                            let msg = status.canonical_reason().unwrap_or("Error").to_string();
                            let html = eh(status, msg);
                            resp = (status, Html(html)).into_response();
                        }
                    }

                    resp
                }
            };

            let method_router: MethodRouter<Arc<handler::AppState>> = match custom.method {
                HttpMethod::Get => routing::get(axum_handler),
                HttpMethod::Post => routing::post(axum_handler),
                HttpMethod::Put => routing::put(axum_handler),
                HttpMethod::Delete => routing::delete(axum_handler),
                HttpMethod::Patch => routing::patch(axum_handler),
            };

            router = router.route(&custom.path, method_router);
        }

        // Index route
        if !has_custom_index {
            let title_for_index = title.clone();
            let resource_info: Vec<(String, String)> = self
                .resources
                .iter()
                .map(|r| (r.label_plural.clone(), r.route_prefix.clone()))
                .collect();

            if let Some(index_renderer) = self.index_page_content {
                let title_c = title.clone();
                router = router.route(
                    "/",
                    routing::get(move |
                        State(state): State<Arc<handler::AppState>>,
                        method: Method,
                        uri: Uri,
                        headers: HeaderMap,
                    | {
                        let index_renderer = index_renderer.clone();
                        let title_c = title_c.clone();
                        async move {
                            let ctx = RequestContext {
                                state: state.clone(),
                                params: HashMap::new(),
                                query: uri.query().unwrap_or("").to_string(),
                                request_path: "/".to_string(),
                                lang_code: String::new(),
                                lang_prefix: String::new(),
                                method,
                                headers,
                                body: Bytes::new(),
                                remote_addr: None,
                            };
                            let response = index_renderer(ctx).await;
                            match response {
                                PageResponse::Ok(content) => {
                                    let html = layout::render_layout(&LayoutConfig {
                                        title: &title_c,
                                        nav_items: &[],
                                        breadcrumbs: &[],
                                        stats_html: "",
                                        content_html: &content,
                                        extra_css: "",
                                    });
                                    Html(html).into_response()
                                }
                                other => page_response_to_axum(other),
                            }
                        }
                    }),
                );
            } else {
                router = router.route(
                    "/",
                    routing::get(move |State(_state): State<Arc<handler::AppState>>| {
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
                    }),
                );
            }
        }

        let mut router = router.with_state(state);

        // Static file directories
        for (url_path, dir_path) in &self.static_dirs {
            let url = url_path.trim_end_matches('/');
            let serve = ServeDir::new(dir_path);
            router = router.nest_service(url, serve);
        }

        // Trailing-slash normalization + custom fallback
        let custom_fb = self.custom_fallback.clone();
        router = router.fallback(move |req: axum::http::Request<axum::body::Body>| {
            let custom_fb = custom_fb.clone();
            async move {
                let path = req.uri().path();
                if path.len() > 1 && path.ends_with('/') {
                    let trimmed = path.trim_end_matches('/');
                    if trimmed.starts_with("//") || !trimmed.starts_with('/') {
                        return (StatusCode::BAD_REQUEST, "Bad Request").into_response();
                    }
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
                (StatusCode::NOT_FOUND, "Not Found").into_response()
            }
        });

        // Apply middleware layers
        for layer_fn in self.layers {
            router = layer_fn(router);
        }

        Ok((router, self.shutdown_hooks))
    }

    /// Start the application with graceful shutdown support.
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("{}:{}", self.bind_addr, self.port);
        let title = self.title.clone();
        let store_path = self.store_path.clone();

        let (router, shutdown_hooks) = self.build()?;

        println!();
        println!("  {} running at http://{}", title, addr);
        if let Some(ref path) = store_path {
            println!("  Database: {}/store.wal", path);
        } else {
            println!("  Database: in-memory");
        }
        println!("  Press Ctrl+C to stop.");
        println!();

        let listener = TcpListener::bind(&addr).await?;

        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        for hook in shutdown_hooks {
            hook();
        }

        Ok(())
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("\n  Shutting down gracefully...");
}

// ---------------------------------------------------------------------------
// Test client
// ---------------------------------------------------------------------------

/// A test client for making HTTP requests against the app without TCP binding.
pub struct TestClient {
    inner: axum_test_client::InnerClient,
}

mod axum_test_client {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    pub struct InnerClient {
        router: Router,
    }

    impl InnerClient {
        pub fn new(router: Router) -> Self {
            Self { router }
        }

        pub async fn request(&self, req: Request<Body>) -> TestResponse {
            let resp = self.router.clone().oneshot(req).await.unwrap();
            let status = resp.status();
            let headers = resp.headers().clone();
            let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
            TestResponse {
                status,
                headers,
                body: body_bytes,
            }
        }
    }

    /// Response from TestClient with assertion helpers.
    pub struct TestResponse {
        pub status: StatusCode,
        pub headers: HeaderMap,
        pub body: Bytes,
    }

    impl TestResponse {
        pub fn status(&self) -> StatusCode {
            self.status
        }

        pub fn text(&self) -> String {
            String::from_utf8_lossy(&self.body).to_string()
        }

        pub fn json<T: DeserializeOwned>(&self) -> T {
            serde_json::from_slice(&self.body).unwrap()
        }

        pub fn header(&self, name: &str) -> Option<&str> {
            self.headers.get(name).and_then(|v| v.to_str().ok())
        }
    }
}

pub use axum_test_client::TestResponse;

impl App {
    /// Create a test client for this app (no TCP binding).
    pub fn test_client(self) -> TestClient {
        let (router, _hooks) = self.build().expect("failed to build app for testing");
        TestClient {
            inner: axum_test_client::InnerClient::new(router),
        }
    }
}

impl TestClient {
    pub async fn get(&self, path: &str) -> TestResponse {
        let req = axum::http::Request::builder()
            .method(Method::GET)
            .uri(path)
            .body(axum::body::Body::empty())
            .unwrap();
        self.inner.request(req).await
    }

    pub async fn post(&self, path: &str, body: &str) -> TestResponse {
        let req = axum::http::Request::builder()
            .method(Method::POST)
            .uri(path)
            .header("content-type", "application/json")
            .body(axum::body::Body::from(body.to_string()))
            .unwrap();
        self.inner.request(req).await
    }

    pub async fn put(&self, path: &str, body: &str) -> TestResponse {
        let req = axum::http::Request::builder()
            .method(Method::PUT)
            .uri(path)
            .header("content-type", "application/json")
            .body(axum::body::Body::from(body.to_string()))
            .unwrap();
        self.inner.request(req).await
    }

    pub async fn delete(&self, path: &str) -> TestResponse {
        let req = axum::http::Request::builder()
            .method(Method::DELETE)
            .uri(path)
            .body(axum::body::Body::empty())
            .unwrap();
        self.inner.request(req).await
    }

    pub async fn request(
        &self,
        method: Method,
        path: &str,
        headers: Vec<(&str, &str)>,
        body: Option<&str>,
    ) -> TestResponse {
        let mut builder = axum::http::Request::builder()
            .method(method)
            .uri(path);
        for (k, v) in headers {
            builder = builder.header(k, v);
        }
        let body = match body {
            Some(b) => axum::body::Body::from(b.to_string()),
            None => axum::body::Body::empty(),
        };
        let req = builder.body(body).unwrap();
        self.inner.request(req).await
    }
}

// ---------------------------------------------------------------------------
// Built-in route handlers
// ---------------------------------------------------------------------------

async fn serve_live_js() -> Response {
    (
        [("content-type", "application/javascript; charset=utf-8")],
        LIVE_JS,
    )
        .into_response()
}

async fn handle_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<handler::AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handler::ws_event_loop(socket, state))
}

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

    #[test]
    fn page_response_json() {
        let resp = PageResponse::json(&serde_json::json!({"ok": true}));
        match resp {
            PageResponse::Json(v) => assert_eq!(v["ok"], true),
            _ => panic!("expected Json variant"),
        }
    }

    #[test]
    fn page_response_with_status() {
        let resp = PageResponse::with_status(418, "I'm a teapot");
        match resp {
            PageResponse::Custom { status, body, .. } => {
                assert_eq!(status, 418);
                assert_eq!(body, "I'm a teapot");
            }
            _ => panic!("expected Custom variant"),
        }
    }

    #[test]
    fn url_decode() {
        assert_eq!(urlencoding_decode("hello+world"), "hello world");
        assert_eq!(urlencoding_decode("test%20value"), "test value");
        assert_eq!(urlencoding_decode("a%26b"), "a&b");
    }

    #[tokio::test]
    async fn test_client_get() {
        let app = App::new("Test")
            .page("/hello", |_ctx| "Hello, World!");

        let client = app.test_client();
        let resp = client.get("/hello").await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.text().contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_client_post_json() {
        let app = App::new("Test")
            .post("/api/echo", |ctx: RequestContext| {
                let body: serde_json::Value = ctx.body_json().unwrap_or_default();
                PageResponse::Json(body)
            });

        let client = app.test_client();
        let resp = client.post("/api/echo", r#"{"name":"test"}"#).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json: serde_json::Value = resp.json();
        assert_eq!(json["name"], "test");
    }

    #[tokio::test]
    async fn test_client_not_found() {
        let app = App::new("Test");
        let client = app.test_client();
        let resp = client.get("/nonexistent").await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_client_page_response_variants() {
        let app = App::new("Test")
            .page("/ok", |_| PageResponse::Ok("ok".to_string()))
            .page("/bad", |_| PageResponse::BadRequest("bad".to_string()))
            .page("/forbidden", |_| PageResponse::Forbidden("no".to_string()))
            .page("/error", |_| PageResponse::InternalError("oops".to_string()))
            .page("/redirect", |_| PageResponse::Redirect("/target".to_string()));

        let client = app.test_client();

        assert_eq!(client.get("/ok").await.status(), StatusCode::OK);
        assert_eq!(client.get("/bad").await.status(), StatusCode::BAD_REQUEST);
        assert_eq!(client.get("/forbidden").await.status(), StatusCode::FORBIDDEN);
        assert_eq!(client.get("/error").await.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(client.get("/redirect").await.status(), StatusCode::PERMANENT_REDIRECT);
    }

    #[tokio::test]
    async fn test_health_check() {
        let app = App::new("Test")
            .health_check("/health");

        let client = app.test_client();
        let resp = client.get("/health").await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.text(), "ok");
    }

    #[tokio::test]
    async fn test_query_params() {
        let app = App::new("Test")
            .page("/search", |ctx: RequestContext| {
                let q = ctx.query_param("q").unwrap_or_default();
                PageResponse::Ok(format!("query={}", q))
            });

        let client = app.test_client();
        let resp = client.get("/search?q=hello&page=1").await;
        assert!(resp.text().contains("query=hello"));
    }

    #[tokio::test]
    async fn test_request_headers() {
        let app = App::new("Test")
            .page("/headers", |ctx: RequestContext| {
                let auth = ctx.header("x-custom").unwrap_or("none");
                PageResponse::Ok(format!("custom={}", auth))
            });

        let client = app.test_client();
        let resp = client.request(
            Method::GET,
            "/headers",
            vec![("x-custom", "myvalue")],
            None,
        ).await;
        assert!(resp.text().contains("custom=myvalue"));
    }

    #[tokio::test]
    async fn test_put_and_delete() {
        let app = App::new("Test")
            .put("/items/:id", |ctx: RequestContext| {
                let id = ctx.param("id").to_string();
                PageResponse::Json(serde_json::json!({"updated": id}))
            })
            .delete("/items/:id", |ctx: RequestContext| {
                let id = ctx.param("id").to_string();
                PageResponse::Json(serde_json::json!({"deleted": id}))
            });

        let client = app.test_client();

        let resp = client.put("/items/42", "{}").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json: serde_json::Value = resp.json();
        assert_eq!(json["updated"], "42");

        let resp = client.delete("/items/42").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json: serde_json::Value = resp.json();
        assert_eq!(json["deleted"], "42");
    }
}
