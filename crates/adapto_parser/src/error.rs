use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Parse error at line {line}, col {col}: {message}")]
    Syntax {
        line: usize,
        col: usize,
        message: String,
    },
    #[error("Unknown block type: {0}")]
    UnknownBlock(String),
    #[error("Duplicate block: {0}")]
    DuplicateBlock(String),
    #[error("Missing required field '{field}' in {block}")]
    MissingField { block: String, field: String },
    #[error("Invalid value '{value}' for field '{field}': {reason}")]
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },
    #[error("Unclosed block: {0}")]
    UnclosedBlock(String),
    #[error("Unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type ParseResult<T> = Result<T, ParseError>;
