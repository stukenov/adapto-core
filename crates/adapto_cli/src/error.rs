use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Compile error: {0}")]
    CompileError(String),
    #[error("Check failed with {0} error(s)")]
    CheckFailed(usize),
    #[error("Config error: {0}")]
    ConfigError(String),
    #[error("Not an Adapto project (adapto.toml not found)")]
    NotAProject,
}
