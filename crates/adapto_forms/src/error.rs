use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormError {
    #[error("Invalid schema: {0}")]
    InvalidSchema(String),
    #[error("Validation failed: {0} error(s)")]
    ValidationFailed(usize),
    #[error("Unknown field: {0}")]
    UnknownField(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}
