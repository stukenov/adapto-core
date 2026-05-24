use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::ProtocolError;

/// The current protocol version. Clients and servers must agree on this
/// value for messages to be accepted.
pub const PROTOCOL_VERSION: u8 = 1;

/// Top-level envelope for all client-to-server messages.
/// The `v` field carries the protocol version so the server can reject
/// incompatible clients immediately, before attempting to parse the payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientMessage {
    /// Protocol version, always [`PROTOCOL_VERSION`].
    pub v: u8,
    #[serde(flatten)]
    pub payload: ClientPayload,
}

impl ClientMessage {
    /// Validates structural invariants of the message:
    /// - Protocol version must match [`PROTOCOL_VERSION`].
    /// - Session IDs must be non-empty.
    /// - Handler and event names must be non-empty where required.
    /// - Navigation paths must start with `/`.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.v != PROTOCOL_VERSION {
            return Err(ProtocolError::InvalidVersion(self.v));
        }

        match &self.payload {
            ClientPayload::Event(event) => {
                validate_session(&event.session)?;
                if event.component.is_empty() {
                    return Err(ProtocolError::MissingField("component".into()));
                }
                if event.event.is_empty() {
                    return Err(ProtocolError::MissingField("event".into()));
                }
                if event.handler.is_empty() {
                    return Err(ProtocolError::MissingField("handler".into()));
                }
            }
            ClientPayload::FormSubmit(form) => {
                validate_session(&form.session)?;
                if form.component.is_empty() {
                    return Err(ProtocolError::MissingField("component".into()));
                }
                if form.handler.is_empty() {
                    return Err(ProtocolError::MissingField("handler".into()));
                }
            }
            ClientPayload::Navigate(nav) => {
                validate_session(&nav.session)?;
                if nav.path.is_empty() {
                    return Err(ProtocolError::MissingField("path".into()));
                }
                if !nav.path.starts_with('/') {
                    return Err(ProtocolError::InvalidEventType(format!(
                        "navigation path must start with '/', got: {}",
                        nav.path
                    )));
                }
            }
            ClientPayload::Heartbeat(hb) => {
                validate_session(&hb.session)?;
            }
        }

        Ok(())
    }
}

/// Validates that a session ID is non-empty.
fn validate_session(session: &str) -> Result<(), ProtocolError> {
    if session.is_empty() {
        return Err(ProtocolError::InvalidSession);
    }
    Ok(())
}

/// Discriminated union of all client-to-server payload types.
/// Tagged by a `"type"` field in JSON so the server can route
/// without inspecting the full body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientPayload {
    /// A DOM event (click, input, change, etc.) bound to a handler.
    #[serde(rename = "event")]
    Event(ClientEvent),

    /// A form submission carrying all form field values.
    #[serde(rename = "form_submit")]
    FormSubmit(FormSubmitEvent),

    /// A client-side navigation request (pushState / popstate).
    #[serde(rename = "navigate")]
    Navigate(NavigateEvent),

    /// Keep-alive heartbeat to maintain the WebSocket connection.
    #[serde(rename = "heartbeat")]
    Heartbeat(HeartbeatEvent),
}

/// A DOM event dispatched from a component's event binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientEvent {
    /// Session identifier tying this event to server-side state.
    pub session: String,
    /// The component ID that sourced the event.
    pub component: String,
    /// DOM event type: `"click"`, `"input"`, `"change"`, etc.
    pub event: String,
    /// The handler action name declared in the template.
    pub handler: String,
    /// Arbitrary event data (e.g., input value, coordinates).
    pub payload: HashMap<String, serde_json::Value>,
    /// Monotonically increasing sequence number for ordering.
    pub seq: u64,
}

/// A form submission event carrying all form field values as a map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormSubmitEvent {
    pub session: String,
    pub component: String,
    /// The handler action name for this form submission.
    pub handler: String,
    /// Form field values keyed by field name.
    pub form: HashMap<String, serde_json::Value>,
    pub seq: u64,
}

/// A client-side navigation event triggered by link clicks or
/// browser history changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigateEvent {
    pub session: String,
    /// The target path, must start with `/`.
    pub path: String,
    pub seq: u64,
}

/// A keep-alive heartbeat sent at a regular interval to prevent
/// WebSocket timeout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatEvent {
    pub session: String,
    pub seq: u64,
}
