use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("E0101: Unknown action `{action}` at {file}:{line}:{col}")]
    UnknownAction {
        action: String,
        file: String,
        line: usize,
        col: usize,
    },

    #[error("E0421: Secret state `{field}` cannot be rendered in template at {file}:{line}:{col}")]
    SecretStateInTemplate {
        field: String,
        file: String,
        line: usize,
        col: usize,
    },

    #[error("E0702: Tenant-required route uses unscoped repository query")]
    UnscopedQuery { file: String, line: usize },

    #[error("E0201: Duplicate state declaration: {name}")]
    DuplicateState { name: String },

    #[error("E0301: Unknown component: {name}")]
    UnknownComponent { name: String },

    #[error("E0401: Missing required permission for action: {action}")]
    MissingPermission { action: String },

    #[error("E0501: Invalid template syntax at {file}:{line}:{col}: {message}")]
    TemplateSyntax {
        file: String,
        line: usize,
        col: usize,
        message: String,
    },

    #[error("Compilation failed with {count} errors")]
    Multiple {
        count: usize,
        errors: Vec<CompileError>,
    },
}
