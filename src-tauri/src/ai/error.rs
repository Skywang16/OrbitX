use reqwest::header::InvalidHeaderValue;
use reqwest::StatusCode;
use thiserror::Error;

use crate::storage::error::{RepositoryError, StoragePathsError};

pub type AIServiceResult<T> = Result<T, AIServiceError>;
pub type FileSystemToolResult<T> = Result<T, FileSystemToolError>;

#[derive(Debug, Error)]
pub enum AIServiceError {
    #[error("Repository operation {operation} failed: {source}")]
    Repository {
        operation: &'static str,
        #[source]
        source: RepositoryError,
    },
    #[error("Model not found: {model_id}")]
    ModelNotFound { model_id: String },
    #[error("Invalid update payload: {0}")]
    InvalidUpdatePayload(#[from] serde_json::Error),
    #[error("Failed to build HTTP client: {0}")]
    HttpClient(#[from] reqwest::Error),
    #[error("{provider} request failed: {source}")]
    ProviderRequest {
        provider: &'static str,
        #[source]
        source: reqwest::Error,
    },
    #[error("{provider} API error: {status} - {message}")]
    ProviderApi {
        provider: &'static str,
        status: StatusCode,
        message: String,
    },
    #[error("Invalid header value for {name}: {source}")]
    InvalidHeaderValue {
        name: &'static str,
        #[source]
        source: InvalidHeaderValue,
    },
    #[error("Gemini API error: {0}")]
    GeminiApi(String),
    #[error(transparent)]
    FileSystemTool(#[from] FileSystemToolError),
}

#[derive(Debug, Error)]
pub enum FileSystemToolError {
    #[error(transparent)]
    Paths(#[from] StoragePathsError),
    #[error("Filesystem I/O error: {operation}: {source}")]
    Io {
        operation: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Filesystem backup error: {0}")]
    Backup(String),
    #[error("Filesystem tool error: {0}")]
    Internal(String),
}
