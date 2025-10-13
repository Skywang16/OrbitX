use crate::storage::error::RepositoryError;
use reqwest::StatusCode;
use serde_json::Error as SerdeError;
use thiserror::Error;

pub type LlmResult<T> = Result<T, LlmError>;
pub type LlmProviderResult<T> = Result<T, LlmProviderError>;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error("Model not found: {model_id}")]
    ModelNotFound { model_id: String },
    #[error("Unsupported provider type: {provider}")]
    UnsupportedProvider { provider: String },
    #[error("Failed to parse provider options")]
    OptionsParse {
        #[source]
        source: SerdeError,
    },
    #[error("Invalid request: {reason}")]
    InvalidRequest { reason: String },
    #[error(transparent)]
    Provider(#[from] LlmProviderError),
}

#[derive(Debug, Error)]
pub enum LlmProviderError {
    #[error(transparent)]
    OpenAi(#[from] OpenAiError),
    #[error(transparent)]
    Anthropic(#[from] AnthropicError),
    #[error(transparent)]
    Gemini(#[from] GeminiError),
    #[error("Unsupported provider: {provider}")]
    UnsupportedProvider { provider: String },
    #[error("Provider operation unsupported: {provider}::{operation}")]
    UnsupportedOperation {
        provider: &'static str,
        operation: &'static str,
    },
}

#[derive(Debug, Error)]
pub enum OpenAiError {
    #[error("OpenAI HTTP request failed")]
    Http {
        #[source]
        source: reqwest::Error,
    },
    #[error("OpenAI API error {status}: {message}")]
    Api { status: StatusCode, message: String },
    #[error("OpenAI response missing field: {field}")]
    MissingField { field: &'static str },
    #[error("OpenAI tool call arguments parse failed")]
    ToolCallArguments {
        #[source]
        source: SerdeError,
    },
    #[error("OpenAI JSON parse failed")]
    Json {
        #[source]
        source: SerdeError,
    },
    #[error("OpenAI embedding response missing field: {field}")]
    EmbeddingField { field: &'static str },
    #[error("OpenAI streaming error: {message}")]
    Stream { message: String },
}

#[derive(Debug, Error)]
pub enum AnthropicError {
    #[error("Anthropic HTTP request failed")]
    Http {
        #[source]
        source: reqwest::Error,
    },
    #[error("Anthropic API error {status}: {message}")]
    Api { status: StatusCode, message: String },
    #[error("Anthropic JSON parse failed")]
    Json {
        #[source]
        source: SerdeError,
    },
    #[error("Anthropic streaming error: {message}")]
    Stream { message: String },
}

#[derive(Debug, Error)]
pub enum GeminiError {
    #[error("Gemini HTTP request failed")]
    Http {
        #[source]
        source: reqwest::Error,
    },
    #[error("Gemini API error {status}: {message}")]
    Api { status: StatusCode, message: String },
    #[error("Gemini response missing field: {field}")]
    MissingField { field: &'static str },
    #[error("Gemini JSON parse failed")]
    Json {
        #[source]
        source: SerdeError,
    },
    #[error("Gemini streaming error: {message}")]
    Stream { message: String },
}
