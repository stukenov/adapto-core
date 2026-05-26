//! WebSocket event handler for Adapto apps.
//!
//! Provides a generic WebSocket loop that dispatches incoming events
//! to registered action handlers. Apps register handlers via the
//! `App` builder; this module runs the event loop.

use adapto_client_protocol::patch::{PatchMessage, PatchOp, ServerMessage, ServerPayload};
use adapto_store::AdaptoStore;
use axum::extract::ws::{Message, WebSocket};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Shared application state passed into every WebSocket connection.
pub struct AppState {
    pub store: AdaptoStore,
    pub handlers: ActionHandlerMap,
    pub title: String,
}

/// The result of handling an action: a list of DOM patches to send back.
pub struct ActionResult {
    pub ops: Vec<PatchOp>,
}

impl ActionResult {
    /// Create a result with a single `replace_html` operation.
    pub fn replace_html(target: impl Into<String>, html: impl Into<String>) -> Self {
        Self {
            ops: vec![PatchOp::ReplaceHtml {
                target: target.into(),
                html: html.into(),
            }],
        }
    }

    /// Create a result with multiple patch operations.
    pub fn with_ops(ops: Vec<PatchOp>) -> Self {
        Self { ops }
    }

    /// An empty result — no DOM changes.
    pub fn none() -> Self {
        Self { ops: vec![] }
    }
}

/// Context passed to action handlers with everything they need to
/// process the event and produce a response.
pub struct ActionContext<'a> {
    /// The embedded document store.
    pub store: &'a AdaptoStore,
    /// The JSON payload from the client event.
    pub payload: &'a Value,
    /// Per-connection mutable session state (string key-value pairs).
    pub session: &'a mut HashMap<String, String>,
}

/// A boxed action handler function.
pub type ActionHandler =
    Box<dyn Fn(&mut ActionContext<'_>) -> ActionResult + Send + Sync + 'static>;

/// Map of action names to their handlers.
pub type ActionHandlerMap = Arc<HashMap<String, ActionHandler>>;

/// Run the WebSocket event loop for a single connection.
///
/// Reads JSON messages from the client, dispatches to the matching
/// action handler, and sends back patch messages.
pub async fn ws_event_loop(mut socket: WebSocket, state: Arc<AppState>) {
    let mut session: HashMap<String, String> = HashMap::new();

    while let Some(Ok(msg)) = socket.recv().await {
        match &msg {
            Message::Ping(data) => {
                if socket.send(Message::Pong(data.clone())).await.is_err() {
                    break;
                }
                continue;
            }
            Message::Close(_) => break,
            _ => {}
        }
        if let Message::Text(text) = msg {
            let val: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let handler_name = match val.get("handler").and_then(|h| h.as_str()) {
                Some(h) => h,
                None => continue,
            };
            let payload = val
                .get("payload")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            let client_seq = val.get("seq").and_then(|s| s.as_u64()).unwrap_or(0);

            let result = if let Some(handler) = state.handlers.get(handler_name) {
                let mut ctx = ActionContext {
                    store: &state.store,
                    payload: &payload,
                    session: &mut session,
                };
                handler(&mut ctx)
            } else {
                tracing::warn!(action = handler_name, "No handler registered for action");
                ActionResult::none()
            };

            if result.ops.is_empty() {
                continue;
            }

            let patch = PatchMessage {
                seq: client_seq,
                ops: result.ops,
            };

            let server_msg = ServerMessage::new(ServerPayload::Patch(patch));
            if let Ok(json_str) = serde_json::to_string(&server_msg) {
                if socket.send(Message::Text(json_str.into())).await.is_err() {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_result_replace_html() {
        let result = ActionResult::replace_html("app-content", "<p>hello</p>");
        assert_eq!(result.ops.len(), 1);
        match &result.ops[0] {
            PatchOp::ReplaceHtml { target, html } => {
                assert_eq!(target, "app-content");
                assert_eq!(html, "<p>hello</p>");
            }
            _ => panic!("Expected ReplaceHtml"),
        }
    }

    #[test]
    fn action_result_none_is_empty() {
        let result = ActionResult::none();
        assert!(result.ops.is_empty());
    }

    #[test]
    fn action_result_with_ops() {
        let ops = vec![
            PatchOp::ReplaceHtml {
                target: "a".into(),
                html: "1".into(),
            },
            PatchOp::ReplaceText {
                target: "b".into(),
                value: "2".into(),
            },
        ];
        let result = ActionResult::with_ops(ops);
        assert_eq!(result.ops.len(), 2);
    }
}
