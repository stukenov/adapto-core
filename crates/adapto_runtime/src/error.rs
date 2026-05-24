use crate::types::UserId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Unauthenticated")]
    Unauthenticated,

    #[error("Permission denied: {permission}")]
    PermissionDenied {
        permission: String,
        user_id: Option<UserId>,
    },

    #[error("Tenant required")]
    TenantRequired,

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Invalid session")]
    InvalidSession,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Unknown handler: {0}")]
    UnknownHandler(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
