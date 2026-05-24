use crate::budget::BudgetError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("AI action not found: {0}")]
    ActionNotFound(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("AI execution failed: {0}")]
    ExecutionFailed(String),

    #[error("AI action timed out after {0}ms")]
    Timeout(u64),

    #[error("Output schema validation failed: {0}")]
    OutputValidationFailed(String),

    #[error("Permission denied for AI action: {0}")]
    PermissionDenied(String),

    #[error("Budget exceeded: {0}")]
    BudgetExceeded(#[from] BudgetError),

    #[error("PII redaction failed: {0}")]
    PiiRedactionFailed(String),

    #[error("All retries exhausted")]
    RetriesExhausted,
}
