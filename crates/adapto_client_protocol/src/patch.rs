use serde::{Deserialize, Serialize};

use crate::event::PROTOCOL_VERSION;

/// Top-level envelope for all server-to-client messages.
/// Mirrors [`crate::event::ClientMessage`] with a version field
/// for forward-compatibility negotiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMessage {
    /// Protocol version, always [`PROTOCOL_VERSION`].
    pub v: u8,
    #[serde(flatten)]
    pub payload: ServerPayload,
}

impl ServerMessage {
    /// Convenience constructor that stamps the current protocol version.
    pub fn new(payload: ServerPayload) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            payload,
        }
    }
}

/// Discriminated union of all server-to-client payload types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerPayload {
    /// A batch of DOM patch operations to apply.
    #[serde(rename = "patch")]
    Patch(PatchMessage),

    /// An error response, optionally tied to a client sequence number.
    #[serde(rename = "error")]
    Error(ErrorMessage),

    /// A full-page redirect instruction.
    #[serde(rename = "redirect")]
    Redirect(RedirectMessage),

    /// Acknowledgement of a client heartbeat.
    #[serde(rename = "heartbeat_ack")]
    HeartbeatAck(HeartbeatAck),
}

/// A batch of ordered patch operations the client must apply
/// to reconcile its DOM with server state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchMessage {
    /// Sequence number echoed from the client event that triggered
    /// these patches, enabling the client to correlate responses.
    pub seq: u64,
    /// Ordered list of DOM operations.
    pub ops: Vec<PatchOp>,
}

/// Individual DOM patch operations. Each variant maps to a single,
/// atomic DOM mutation the client runtime executes.
///
/// Target strings use a compact selector format: `"c:component_id#element_id"`
/// or a plain CSS selector, depending on the runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum PatchOp {
    /// Replace the text content of the target element.
    #[serde(rename = "replace_text")]
    ReplaceText { target: String, value: String },

    /// Replace the inner HTML of the target element.
    #[serde(rename = "replace_html")]
    ReplaceHtml { target: String, html: String },

    /// Set an attribute on the target element.
    #[serde(rename = "set_attr")]
    SetAttr {
        target: String,
        name: String,
        value: String,
    },

    /// Remove an attribute from the target element.
    #[serde(rename = "remove_attr")]
    RemoveAttr { target: String, name: String },

    /// Add a CSS class to the target element.
    #[serde(rename = "add_class")]
    AddClass { target: String, class: String },

    /// Remove a CSS class from the target element.
    #[serde(rename = "remove_class")]
    RemoveClass { target: String, class: String },

    /// Insert HTML before the target element (as a preceding sibling).
    #[serde(rename = "insert_before")]
    InsertBefore { target: String, html: String },

    /// Insert HTML after the target element (as a following sibling).
    #[serde(rename = "insert_after")]
    InsertAfter { target: String, html: String },

    /// Remove the target element from the DOM entirely.
    #[serde(rename = "remove_node")]
    RemoveNode { target: String },

    /// Move keyboard focus to the target element.
    #[serde(rename = "focus")]
    Focus { target: String },

    /// Scroll the viewport so the target element is visible.
    #[serde(rename = "scroll_to")]
    ScrollTo { target: String },

    /// Trigger a client-side navigation redirect.
    #[serde(rename = "redirect")]
    Redirect { url: String },

    /// Display a transient flash notification.
    #[serde(rename = "flash")]
    Flash { level: FlashLevel, message: String },

    /// Open a modal dialog with the given HTML content.
    #[serde(rename = "modal_open")]
    ModalOpen { id: String, html: String },

    /// Close an open modal dialog by ID.
    #[serde(rename = "modal_close")]
    ModalClose { id: String },
}

/// Severity levels for flash notifications, mapping to standard
/// visual treatments in the client runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FlashLevel {
    Success,
    Info,
    Warning,
    Danger,
}

/// Error response from the server. The optional `seq` ties the error
/// back to the client event that caused it; `None` means a session-level error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub seq: Option<u64>,
    pub code: String,
    pub message: String,
}

/// A server-initiated redirect, optionally accompanied by a flash message
/// to display after navigation completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectMessage {
    pub url: String,
    pub flash: Option<(FlashLevel, String)>,
}

/// Server acknowledgement of a client heartbeat, echoing the sequence
/// number so the client can measure round-trip latency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatAck {
    pub seq: u64,
}
