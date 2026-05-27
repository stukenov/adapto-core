use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid CSRF token")]
    InvalidCsrfToken,

    #[error("Expired CSRF token")]
    ExpiredCsrfToken,

    #[error("Invalid session signature")]
    InvalidSessionSignature,

    #[error("Malformed token")]
    MalformedToken,

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("Invalid password hash")]
    InvalidPasswordHash,

    #[error("Password verification failed")]
    PasswordMismatch,

    #[error("Invalid JWT: {0}")]
    InvalidJwt(String),

    #[error("Expired JWT")]
    ExpiredJwt,

    #[error("Session not found")]
    SessionNotFound,

    #[error("Session expired")]
    SessionExpired,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden: {0}")]
    Forbidden(String),
}
