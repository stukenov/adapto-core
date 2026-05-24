use thiserror::Error;

/// All errors produced by the adapto_store crate.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Duplicate key in unique index `{index}`: {key}")]
    DuplicateKey { index: String, key: String },

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Invalid update: {0}")]
    InvalidUpdate(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Index not found: {0}")]
    IndexNotFound(String),

    #[error("WAL corrupted: {0}")]
    WalCorrupted(String),

    #[error("Tenant mismatch: expected `{expected}`, got `{actual}`")]
    TenantMismatch { expected: String, actual: String },
}
