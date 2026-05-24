use adapto_live::handler::EventDispatcher;
use adapto_live::manager::SessionManager;
use adapto_ssr::page::PageRenderer;
use adapto_ssr::server::{AdaptoServer, AppState};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_app_state() -> Arc<AppState> {
    Arc::new(AppState {
        page_renderer: PageRenderer::new(b"integration-secret"),
        session_manager: SessionManager::new(10),
        event_dispatcher: std::sync::Mutex::new(EventDispatcher::new(100)),
        secret: b"integration-secret".to_vec(),
    })
}

fn test_server() -> AdaptoServer {
    AdaptoServer::new(
        PageRenderer::new(b"integration-secret"),
        SessionManager::new(10),
        100,
        b"integration-secret".to_vec(),
    )
}

// ===========================================================================
// AppState creation
// ===========================================================================

#[test]
fn app_state_holds_secret() {
    let state = test_app_state();
    assert_eq!(state.secret, b"integration-secret");
}

#[test]
fn app_state_session_manager_starts_empty() {
    let state = test_app_state();
    assert_eq!(state.session_manager.count(), 0);
}

#[test]
fn app_state_event_dispatcher_accessible() {
    let state = test_app_state();
    // Locking succeeds without panic.
    let _guard = state.event_dispatcher.lock().unwrap();
}

// ===========================================================================
// AdaptoServer builder
// ===========================================================================

#[test]
fn server_new_creates_instance() {
    let server = test_server();
    let state = server.state();
    assert_eq!(state.secret, b"integration-secret");
}

#[test]
fn server_state_is_shared() {
    let server = test_server();
    let s1 = server.state();
    let s2 = server.state();
    // Both Arc references point to the same allocation.
    assert!(Arc::ptr_eq(&s1, &s2));
}

#[test]
fn server_builder_with_different_rate_limits() {
    let server_low = AdaptoServer::new(
        PageRenderer::new(b"s"),
        SessionManager::new(5),
        10,
        b"s".to_vec(),
    );
    let server_high = AdaptoServer::new(
        PageRenderer::new(b"s"),
        SessionManager::new(5),
        10000,
        b"s".to_vec(),
    );

    // Both construct successfully with different rate limits.
    let _ = server_low.router();
    let _ = server_high.router();
}

// ===========================================================================
// Router construction
// ===========================================================================

#[test]
fn router_builds_without_panic() {
    let server = test_server();
    let _router = server.router();
}

#[test]
fn router_can_be_built_multiple_times() {
    let server = test_server();
    let _r1 = server.router();
    let _r2 = server.router();
    // Both should work because state is Arc-cloned.
}

// ===========================================================================
// Health check (via axum test utilities)
// ===========================================================================

#[tokio::test]
async fn health_check_returns_ok() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let server = test_server();
    let app = server.router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["sessions"], 0);
}

// ===========================================================================
// Client JS endpoint
// ===========================================================================

#[tokio::test]
async fn client_js_returns_javascript() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let server = test_server();
    let app = server.router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/assets/adapto-client.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("javascript"));

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let js = String::from_utf8(body.to_vec()).unwrap();
    assert!(js.contains("__ADAPTO_BOOTSTRAP__"));
    assert!(js.contains("WebSocket"));
    assert!(js.contains("ws.onmessage"));
}

// ===========================================================================
// Page handler (route not found for unconfigured router)
// ===========================================================================

#[tokio::test]
async fn page_handler_returns_500_without_router() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let server = test_server();
    let app = server.router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/some/unknown/page")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Without a configured router on the PageRenderer, page
    // requests return 500 (render error, not a route miss).
    assert_eq!(response.status(), 500);
}

#[tokio::test]
async fn page_handler_returns_404_for_unknown_route() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use adapto_compiler::manifest::{RouteEntry, RouteManifest};
    use adapto_ssr::router::Router;

    // Build a server with a configured router but no matching route.
    let mut page_renderer = PageRenderer::new(b"integration-secret");
    page_renderer.set_router(Router::new(RouteManifest {
        routes: vec![RouteEntry {
            id: "home".into(),
            path: "/".into(),
            file: "home.adapto".into(),
            method: "GET".into(),
            auth: "public".into(),
            tenant: "none".into(),
            permission: None,
            layout: None,
            cache: "public".into(),
        }],
    }));

    let server = AdaptoServer::new(
        page_renderer,
        SessionManager::new(10),
        100,
        b"integration-secret".to_vec(),
    );
    let app = server.router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/nonexistent/path")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
}
