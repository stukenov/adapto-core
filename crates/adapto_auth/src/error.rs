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
}
