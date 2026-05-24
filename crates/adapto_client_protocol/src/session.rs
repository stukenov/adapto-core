use serde::{Deserialize, Serialize};

/// Initial payload delivered inline in the server-rendered HTML.
/// The client runtime reads this to establish a WebSocket connection
/// and hydrate the component tree without a separate round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapPayload {
    /// Unique session identifier assigned by the server.
    pub session_id: String,
    /// The WebSocket endpoint URL for this session.
    pub websocket_url: String,
    /// CSRF token for validating the WebSocket upgrade request.
    pub csrf_token: String,
    /// Hash of the initial rendered state, used by the client
    /// to detect stale HTML after reconnection.
    pub initial_state_hash: String,
    /// Flat list of components in the rendered tree, each carrying
    /// metadata the client needs for targeted patching.
    pub component_tree: Vec<ComponentMeta>,
}

/// Metadata about a single component in the rendered tree.
/// The client runtime uses this to map incoming patch targets
/// to their DOM counterparts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMeta {
    /// Stable component instance ID.
    pub id: String,
    /// Component type name (e.g., `"Counter"`, `"ChatInput"`).
    pub name: String,
    /// Dynamic targets within this component that can receive
    /// targeted patch operations.
    pub dynamic_targets: Vec<DynamicTarget>,
}

/// A named location within a component that the server can patch
/// independently, along with the state keys it depends on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicTarget {
    /// Target element ID within the component.
    pub id: String,
    /// State keys this target depends on. When any of these keys
    /// change, the server will emit a patch for this target.
    pub deps: Vec<String>,
}

/// Tuning knobs for the client-side connection manager.
/// Delivered alongside [`BootstrapPayload`] or negotiated
/// during the WebSocket handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Interval in milliseconds between heartbeat messages.
    pub heartbeat_interval_ms: u64,
    /// Maximum number of reconnection attempts before giving up.
    pub reconnect_max_retries: u32,
    /// Base backoff interval in milliseconds between reconnection attempts.
    /// The client runtime applies exponential backoff on top of this.
    pub reconnect_backoff_ms: u64,
    /// Maximum number of events per second the client may send.
    pub event_rate_limit: u32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: 30_000,
            reconnect_max_retries: 10,
            reconnect_backoff_ms: 1_000,
            event_rate_limit: 20,
        }
    }
}
