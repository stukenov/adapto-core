use thiserror::Error;

/// Protocol-level errors that can occur during message validation,
/// encoding, or decoding. Each variant maps to a clear failure mode
/// the client or server can act on.
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Invalid protocol version: {0}")]
    InvalidVersion(u8),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid session ID")]
    InvalidSession,

    #[error("Sequence number mismatch: expected {expected}, got {got}")]
    SequenceMismatch { expected: u64, got: u64 },

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Invalid event type: {0}")]
    InvalidEventType(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}
