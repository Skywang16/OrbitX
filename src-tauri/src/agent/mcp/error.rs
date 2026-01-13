use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum McpError {
    #[error("MCP server disabled")]
    Disabled,

    #[error("Invalid MCP config: {0}")]
    InvalidConfig(String),

    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("JSON error")]
    Json(#[from] serde_json::Error),

    #[error("Transport not connected")]
    NotConnected,

    #[error("Transport closed")]
    Closed,

    #[error("Request timed out")]
    Timeout,

    #[error("MCP protocol error: {0}")]
    Protocol(String),

    #[error("MCP server process failed: {0}")]
    Process(String),

    #[error("Failed to spawn MCP server: {command}")]
    SpawnFailed {
        command: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to read MCP stdout line")]
    ReadLineFailed(#[source] std::io::Error),

    #[error("Workspace root is not absolute: {0}")]
    WorkspaceNotAbsolute(PathBuf),
}

pub type McpResult<T> = Result<T, McpError>;
