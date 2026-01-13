//! Shell 执行错误类型

use thiserror::Error;

/// Shell 执行错误
#[derive(Debug, Error)]
pub enum ShellError {
    #[error("Command validation failed: {0}")]
    ValidationFailed(String),

    #[error("Command timed out after {0}ms")]
    Timeout(u64),

    #[error("Command was aborted")]
    Aborted,

    #[error("Command not found: {0}")]
    CommandNotFound(u64),

    #[error("Too many background commands (max: {0})")]
    TooManyBackgroundCommands(usize),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}
