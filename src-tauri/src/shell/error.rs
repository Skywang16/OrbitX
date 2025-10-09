use thiserror::Error;

pub type ShellScriptResult<T> = Result<T, ShellScriptError>;

#[derive(Debug, Error)]
pub enum ShellScriptError {
    #[error("Home directory is not available")]
    HomeDirectoryUnavailable,
    #[error("Unsupported shell type: {0}")]
    UnsupportedShell(String),
    #[error("I/O error during {operation}: {source}")]
    Io {
        operation: String,
        #[source]
        source: std::io::Error,
    },
}
