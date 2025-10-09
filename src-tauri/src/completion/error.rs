use crate::storage::error::CacheError;
use regex::Error as RegexError;
use std::io;
use thiserror::Error;

pub type CompletionEngineResult<T> = Result<T, CompletionEngineError>;
pub type CompletionProviderResult<T> = Result<T, CompletionProviderError>;
pub type OutputAnalyzerResult<T> = Result<T, OutputAnalyzerError>;
pub type SmartExtractorResult<T> = Result<T, SmartExtractorError>;
pub type ContextCollectorResult<T> = Result<T, ContextCollectorError>;
pub type CompletionStateResult<T> = Result<T, CompletionStateError>;

#[derive(Debug, Error)]
pub enum CompletionEngineError {
    #[error("Cache operation failed: {0}")]
    Cache(#[from] CacheError),
}

#[derive(Debug, Error)]
pub enum CompletionProviderError {
    #[error("I/O error while {operation}: {context}: {source}")]
    Io {
        operation: &'static str,
        context: String,
        #[source]
        source: io::Error,
    },
    #[error("Mutex poisoned while accessing {resource}")]
    MutexPoisoned { resource: &'static str },
    #[error("Regex compilation failed for pattern {pattern}: {source}")]
    RegexCompile {
        pattern: String,
        #[source]
        source: RegexError,
    },
    #[error("JSON serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Smart extractor error: {0}")]
    SmartExtractor(#[from] SmartExtractorError),
    #[error("Completion provider internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum SmartExtractorError {
    #[error("Regex compilation failed for pattern {pattern}: {source}")]
    RegexCompile {
        pattern: String,
        #[source]
        source: RegexError,
    },
    #[error("Compiled regex missing for key {pattern_key}")]
    MissingCompiledPattern { pattern_key: String },
    #[error("Failed to read configuration {path}: {source}")]
    ConfigRead { path: String, #[source] source: io::Error },
    #[error("Failed to write configuration {path}: {source}")]
    ConfigWrite { path: String, #[source] source: io::Error },
    #[error("Configuration serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

impl From<io::Error> for CompletionProviderError {
    fn from(error: io::Error) -> Self {
        CompletionProviderError::Io { operation: "IO operation", context: String::new(), source: error }
    }
}

impl From<RegexError> for CompletionProviderError {
    fn from(error: RegexError) -> Self {
        CompletionProviderError::RegexCompile { pattern: String::new(), source: error }
    }
}

#[derive(Debug, Error)]
pub enum OutputAnalyzerError {
    #[error("Mutex poisoned while accessing {resource}")]
    MutexPoisoned { resource: &'static str },
    #[error(transparent)]
    Provider(#[from] CompletionProviderError),
    #[error(transparent)]
    SmartExtractor(#[from] SmartExtractorError),
    #[error("Safe truncation exceeded maximum attempts")]
    TruncationGuard,
}

#[derive(Debug, Error)]
pub enum ContextCollectorError {
    #[error("Mutex poisoned while accessing {resource}")]
    MutexPoisoned { resource: &'static str },
    #[error(transparent)]
    SmartExtractor(#[from] SmartExtractorError),
}

#[derive(Debug, Error)]
pub enum CompletionStateError {
    #[error("Completion engine is not initialised")]
    NotInitialized,
    #[error("Failed to acquire engine lock")]
    LockPoisoned,
}

impl CompletionProviderError {
    pub fn io(operation: &'static str, context: impl Into<String>, source: io::Error) -> Self {
        CompletionProviderError::Io {
            operation,
            context: context.into(),
            source,
        }
    }
}
