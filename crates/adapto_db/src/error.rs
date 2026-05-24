use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Record not found")]
    NotFound,
    #[error("Duplicate record")]
    Duplicate,
    #[error("Tenant scope required")]
    TenantScopeRequired,
    #[error("Query error: {0}")]
    QueryError(String),
    #[error("Migration error: {0}")]
    MigrationError(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
}
