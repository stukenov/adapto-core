use thiserror::Error;

#[derive(Debug, Error)]
pub enum LiveError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Session expired")]
    SessionExpired,
    #[error("Max sessions exceeded for user")]
    MaxSessionsExceeded,
    #[error("Unknown handler: {0}")]
    UnknownHandler(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Invalid event: {0}")]
    InvalidEvent(String),
    #[error("State error: {0}")]
    StateError(String),
    #[error("Handler error: {0}")]
    HandlerError(String),
}
