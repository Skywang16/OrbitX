use thiserror::Error;

pub type WindowStateResult<T> = Result<T, WindowStateError>;

#[derive(Debug, Error)]
pub enum WindowStateError {
    #[error("Window state error: {0}")]
    Message(String),
    #[error("I/O error during {operation}: {source}")]
    Io {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("Serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("Window API error: {0}")]
    WindowApi(String),
    #[error("Poisoned lock: {resource}")]
    Poisoned { resource: &'static str },
}

impl From<String> for WindowStateError {
    fn from(value: String) -> Self {
        WindowStateError::Message(value)
    }
}

impl From<&str> for WindowStateError {
    fn from(value: &str) -> Self {
        WindowStateError::Message(value.to_string())
    }
}
