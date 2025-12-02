use thiserror::Error;

#[derive(Debug, Error)]
pub enum VectorDbError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("Index error: {0}")]
    Index(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid span: {0}")]
    InvalidSpan(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid vector dimension: expected {expected}, got {actual}")]
    InvalidDimension { expected: usize, actual: usize },

    #[error("Chunking error: {0}")]
    ChunkingError(String),
}

pub type Result<T> = std::result::Result<T, VectorDbError>;

impl From<VectorDbError> for String {
    fn from(err: VectorDbError) -> String {
        err.to_string()
    }
}
