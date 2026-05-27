use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing;

use adapto_client_protocol::event::ClientPayload;
use adapto_client_protocol::message;
use adapto_client_protocol::patch::{ErrorMessage, ServerMessage, ServerPayload};
use adapto_live::handler::EventDispatcher;
use adapto_live::manager::SessionManager;
use adapto_runtime::types::SessionId;

use crate::page::PageRenderer;

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

/// Shared state for the Adapto HTTP + WebSocket server.
///
/// Held behind an `Arc` so axum handlers can access it concurrently.
/// The `Mutex` on `event_dispatcher` serialises event processing --
/// a deliberate choice: events within a single server instance are
/// processed in order, matching the sequential mental model users
/// have of their own interactions.
pub struct AppState {
    pub page_renderer: PageRenderer,
    pub session_manager: SessionManager,
    pub event_dispatcher: std::sync::Mutex<EventDispatcher>,
    pub secret: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Server builder
// ---------------------------------------------------------------------------

/// The Adapto server. Binds HTTP page serving, a WebSocket live
/// channel, static asset delivery, and a health-check endpoint into
/// a single `axum::Router`.
///
/// Designed for progressive disclosure: construct with `new`, inspect
/// the `Router` with `router()` for embedding in a larger app, or
/// call `serve` for standalone operation.
pub struct AdaptoServer {
    state: Arc<AppState>,
}

impl AdaptoServer {
    /// Create a new server with the provided dependencies.
    ///
    /// `event_rate_limit` controls the maximum events per second a
    /// single session may submit before being throttled.
    pub fn new(
        page_renderer: PageRenderer,
        session_manager: SessionManager,
        event_rate_limit: u32,
        secret: Vec<u8>,
    ) -> Self {
        let state = Arc::new(AppState {
            page_renderer,
            session_manager,
            event_dispatcher: std::sync::Mutex::new(EventDispatcher::new(event_rate_limit)),
            secret,
        });

        Self { state }
    }

    /// Build the axum `Router` with all Adapto routes.
    ///
    /// The route hierarchy follows a clear information architecture:
    ///
    /// - `/health` -- operational readiness (no auth, no state)
    /// - `/assets/adapto-client.js` -- the client runtime script
    /// - `/ws` -- WebSocket upgrade endpoint for live sessions
    /// - `/*path` -- catch-all page renderer (deepest level)
    ///
    /// This ordering ensures specific routes take priority over the
    /// catch-all, mirroring the specificity principle in CSS: more
    /// precise selectors win.
    pub fn router(&self) -> Router {
        Router::new()
            .route("/health", get(handle_health))
            .route("/assets/adapto-client.js", get(handle_client_js))
            .route("/ws", get(handle_ws))
            .route("/*path", get(handle_page))
            .with_state(self.state.clone())
    }

    /// Start listening on `host:port` and serve requests until shutdown.
    ///
    /// This is the simplest way to run Adapto as a standalone server.
    /// For more control (graceful shutdown, layered middleware), use
    /// `router()` and compose with your own `axum::serve` call.
    pub async fn serve(self, host: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let router = self.router();
        let addr = format!("{}:{}", host, port);
        let listener = TcpListener::bind(&addr).await?;

        tracing::info!("Adapto server listening on {}", addr);

        axum::serve(listener, router).await?;

        Ok(())
    }

    /// Access the shared application state.
    ///
    /// Useful for tests or embedding scenarios where you need to
    /// pre-populate sessions or inspect state.
    pub fn state(&self) -> Arc<AppState> {
        self.state.clone()
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Serve a rendered page for the given path.
///
/// Delegates to `PageRenderer` which handles route matching, auth
/// gating, component lookup, and layout composition. The path is
/// passed through verbatim -- the SSR router normalises it
/// internally.
async fn handle_page(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Response {
    let full_path = format!("/{}", path);

    // For page rendering we use an anonymous context -- real auth
    // integration will inject the authenticated context from session
    // cookies or bearer tokens in a middleware layer.
    let ctx = adapto_runtime::context::Ctx {
        user_id: None,
        tenant_id: None,
        request_id: adapto_runtime::types::RequestId::default(),
        permissions: adapto_runtime::context::PermissionSet::new(),
        route: adapto_runtime::types::RouteId::from(full_path.as_str()),
        session_id: adapto_runtime::types::SessionId::from("anon"),
    };

    let state_store = adapto_runtime::state::StateStore::new();

    match state.page_renderer.render_request(&full_path, &ctx, state_store) {
        Ok(response) => {
            (StatusCode::from_u16(response.status).unwrap_or(StatusCode::OK), Html(response.html))
                .into_response()
        }
        Err(crate::error::SsrError::RouteNotFound(_)) => {
            (StatusCode::NOT_FOUND, Html("<h1>404 - Page not found</h1>".to_string()))
                .into_response()
        }
        Err(crate::error::SsrError::AuthRequired) => {
            (StatusCode::UNAUTHORIZED, Html("<h1>401 - Authentication required</h1>".to_string()))
                .into_response()
        }
        Err(crate::error::SsrError::TenantRequired) => {
            (StatusCode::FORBIDDEN, Html("<h1>403 - Tenant context required</h1>".to_string()))
                .into_response()
        }
        Err(crate::error::SsrError::PermissionDenied(perm)) => {
            (
                StatusCode::FORBIDDEN,
                Html(format!("<h1>403 - Permission denied: {}</h1>", perm)),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Render error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<h1>500 - Internal server error</h1>".to_string()),
            )
                .into_response()
        }
    }
}

/// Upgrade an HTTP connection to a WebSocket for the live session
/// channel.
///
/// The upgrade response is immediate -- the actual message loop
/// runs in `ws_handler` after the protocol switch completes.
async fn handle_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_handler(socket, state))
}

/// WebSocket message loop.
///
/// Reads client messages, validates them against the protocol,
/// dispatches to the appropriate session handler via
/// `EventDispatcher`, and sends back server responses.
///
/// The loop terminates cleanly on client disconnect or protocol
/// errors, ensuring sessions are cleaned up.
async fn ws_handler(mut socket: WebSocket, state: Arc<AppState>) {
    while let Some(msg) = socket.recv().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                tracing::debug!("WebSocket receive error: {}", e);
                break;
            }
        };

        match msg {
            Message::Text(text) => {
                let response = process_ws_message(&text, &state);
                let json = match message::encode_server_message(&response) {
                    Ok(json) => json,
                    Err(e) => {
                        tracing::error!("Failed to encode server message: {}", e);
                        break;
                    }
                };

                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            // Binary frames and pings are handled by axum's WebSocket
            // layer automatically; we ignore them here.
            _ => {}
        }
    }
}

/// Process a single WebSocket text message and produce a server response.
///
/// Separated from the message loop for testability. Each step --
/// decode, validate, dispatch -- is a clear phase with its own error
/// path, following the principle that error messages should be as
/// helpful as an Apple dialog: specific, actionable, recoverable.
fn process_ws_message(text: &str, state: &AppState) -> ServerMessage {
    // 1. Decode
    let client_msg = match message::decode_client_message(text) {
        Ok(msg) => msg,
        Err(e) => {
            return ServerMessage::new(ServerPayload::Error(ErrorMessage {
                seq: None,
                code: "DECODE_ERROR".into(),
                message: format!("Invalid message format: {}", e),
            }));
        }
    };

    // 2. Validate protocol version and structural invariants.
    if let Err(e) = client_msg.validate() {
        return ServerMessage::new(ServerPayload::Error(ErrorMessage {
            seq: None,
            code: "VALIDATION_ERROR".into(),
            message: format!("Message validation failed: {}", e),
        }));
    }

    // 3. Extract the session ID from the payload.
    let session_id_str = extract_session_id(&client_msg.payload);
    let session_id = SessionId::from(session_id_str.as_str());

    // 4. Dispatch the event to the session via EventDispatcher.
    let dispatch_result = state.session_manager.with_session(&session_id, |session| {
        let mut dispatcher = state.event_dispatcher.lock().unwrap();
        dispatcher.dispatch(session, &client_msg.payload)
    });

    match dispatch_result {
        Ok(Ok(payload)) => ServerMessage::new(payload),
        Ok(Err(live_err)) => ServerMessage::new(ServerPayload::Error(ErrorMessage {
            seq: None,
            code: "HANDLER_ERROR".into(),
            message: format!("{}", live_err),
        })),
        Err(live_err) => ServerMessage::new(ServerPayload::Error(ErrorMessage {
            seq: None,
            code: "SESSION_ERROR".into(),
            message: format!("{}", live_err),
        })),
    }
}

/// Extract the session identifier from any client payload variant.
fn extract_session_id(payload: &ClientPayload) -> String {
    match payload {
        ClientPayload::Event(e) => e.session.clone(),
        ClientPayload::FormSubmit(f) => f.session.clone(),
        ClientPayload::Navigate(n) => n.session.clone(),
        ClientPayload::Heartbeat(h) => h.session.clone(),
    }
}

/// Serve the client-side JavaScript runtime.
async fn handle_client_js() -> impl IntoResponse {
    const JS: &str = include_str!("../static/adapto-client.js");
    (
        StatusCode::OK,
        [("content-type", "application/javascript; charset=utf-8")],
        JS,
    )
}

/// Health check endpoint.
///
/// Returns a minimal JSON body with the server status and active
/// session count. Deliberately lightweight -- health checks should
/// complete in microseconds.
async fn handle_health(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let session_count = state.session_manager.count();
    let body = serde_json::json!({
        "status": "ok",
        "sessions": session_count,
    });

    (
        StatusCode::OK,
        [("content-type", "application/json")],
        body.to_string(),
    )
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::PageRenderer;
    use adapto_live::handler::EventDispatcher;
    use adapto_live::manager::SessionManager;

    fn test_app_state() -> Arc<AppState> {
        Arc::new(AppState {
            page_renderer: PageRenderer::new(b"test-secret"),
            session_manager: SessionManager::new(10),
            event_dispatcher: std::sync::Mutex::new(EventDispatcher::new(100)),
            secret: b"test-secret".to_vec(),
        })
    }

    #[test]
    fn app_state_creation() {
        let state = test_app_state();
        assert_eq!(state.secret, b"test-secret");
        assert_eq!(state.session_manager.count(), 0);
    }

    #[test]
    fn server_builder() {
        let server = AdaptoServer::new(
            PageRenderer::new(b"s"),
            SessionManager::new(5),
            100,
            b"s".to_vec(),
        );

        // State should be accessible.
        let state = server.state();
        assert_eq!(state.session_manager.count(), 0);
    }

    #[test]
    fn router_construction() {
        let server = AdaptoServer::new(
            PageRenderer::new(b"r"),
            SessionManager::new(5),
            100,
            b"r".to_vec(),
        );

        // router() should return without panicking.
        let _router = server.router();
    }

    #[test]
    fn extract_session_id_from_event() {
        use adapto_client_protocol::event::*;
        use std::collections::HashMap;

        let payload = ClientPayload::Event(ClientEvent {
            session: "sess-123".into(),
            component: "comp".into(),
            event: "click".into(),
            handler: "handle".into(),
            payload: HashMap::new(),
            seq: 1,
        });

        assert_eq!(extract_session_id(&payload), "sess-123");
    }

    #[test]
    fn extract_session_id_from_heartbeat() {
        use adapto_client_protocol::event::*;

        let payload = ClientPayload::Heartbeat(HeartbeatEvent {
            session: "hb-456".into(),
            seq: 5,
        });

        assert_eq!(extract_session_id(&payload), "hb-456");
    }

    #[test]
    fn extract_session_id_from_navigate() {
        use adapto_client_protocol::event::*;

        let payload = ClientPayload::Navigate(NavigateEvent {
            session: "nav-789".into(),
            path: "/home".into(),
            seq: 3,
        });

        assert_eq!(extract_session_id(&payload), "nav-789");
    }

    #[test]
    fn extract_session_id_from_form_submit() {
        use adapto_client_protocol::event::*;
        use std::collections::HashMap;

        let payload = ClientPayload::FormSubmit(FormSubmitEvent {
            session: "form-101".into(),
            component: "form_comp".into(),
            handler: "submit".into(),
            form: HashMap::new(),
            seq: 2,
        });

        assert_eq!(extract_session_id(&payload), "form-101");
    }

    #[test]
    fn process_ws_message_invalid_json() {
        let state = test_app_state();
        let response = process_ws_message("not json", &state);
        match response.payload {
            ServerPayload::Error(e) => {
                assert_eq!(e.code, "DECODE_ERROR");
                assert!(e.message.contains("Invalid message format"));
            }
            other => panic!("Expected Error payload, got: {:?}", other),
        }
    }

    #[test]
    fn process_ws_message_invalid_version() {
        let state = test_app_state();
        // Valid JSON but wrong protocol version.
        let msg = r#"{"v":99,"type":"heartbeat","session":"s","seq":1}"#;
        let response = process_ws_message(msg, &state);
        match response.payload {
            ServerPayload::Error(e) => {
                assert_eq!(e.code, "VALIDATION_ERROR");
            }
            other => panic!("Expected Error payload, got: {:?}", other),
        }
    }

    #[test]
    fn process_ws_message_session_not_found() {
        let state = test_app_state();
        let msg = r#"{"v":1,"type":"heartbeat","session":"nonexistent","seq":1}"#;
        let response = process_ws_message(msg, &state);
        match response.payload {
            ServerPayload::Error(e) => {
                assert_eq!(e.code, "SESSION_ERROR");
                assert!(e.message.contains("not found"));
            }
            other => panic!("Expected Error payload, got: {:?}", other),
        }
    }
}
