use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("Failed to write audit event: {0}")]
    WriteError(String),

    #[error("Audit channel closed")]
    ChannelClosed,
}
