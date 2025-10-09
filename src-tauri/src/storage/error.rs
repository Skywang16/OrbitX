use std::path::PathBuf;

use argon2::password_hash::Error as PasswordHashError;
use base64::DecodeError;
use chacha20poly1305::aead::Error as AeadError;
use rmp_serde::{
    decode::Error as MessagePackDecodeError,
    encode::Error as MessagePackEncodeError,
};
use sqlx::Error as SqlxError;
use thiserror::Error;

pub type StorageResult<T> = Result<T, StorageError>;
pub type DatabaseResult<T> = Result<T, DatabaseError>;
pub type MessagePackResult<T> = Result<T, MessagePackError>;
pub type StoragePathsResult<T> = Result<T, StoragePathsError>;
pub type StorageRecoveryResult<T> = Result<T, StorageRecoveryError>;
pub type StorageCoordinatorResult<T> = Result<T, StorageCoordinatorError>;
pub type RepositoryResult<T> = Result<T, RepositoryError>;
pub type QueryResult<T> = Result<T, QueryBuilderError>;
pub type CacheResult<T> = Result<T, CacheError>;
pub type SqlScriptResult<T> = Result<T, SqlScriptError>;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    MessagePack(#[from] MessagePackError),
    #[error(transparent)]
    Paths(#[from] StoragePathsError),
    #[error(transparent)]
    Recovery(#[from] StorageRecoveryError),
    #[error(transparent)]
    Coordinator(#[from] StorageCoordinatorError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Query(#[from] QueryBuilderError),
    #[error(transparent)]
    Cache(#[from] CacheError),
    #[error(transparent)]
    SqlScript(#[from] SqlScriptError),
    #[error("Storage internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    Sqlx(#[from] SqlxError),
    #[error("I/O error while {context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Key derivation error: {0}")]
    KeyDerivation(String),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Base64 decode error: {0}")]
    Base64(#[from] DecodeError),
    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("SQL script error: {0}")]
    SqlScript(#[from] SqlScriptError),
    #[error("Encryption not enabled")]
    EncryptionNotEnabled,
    #[error("Invalid encrypted data format")]
    InvalidEncryptedData,
    #[error("Insufficient key length")]
    InsufficientKeyLength,
    #[error("Invalid key length")]
    InvalidKeyLength,
    #[error("Key vault is unavailable")]
    KeyVaultUnavailable,
    #[error("Database internal error: {0}")]
    Internal(String),
}

impl DatabaseError {
    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        DatabaseError::Io {
            context: context.into(),
            source,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        DatabaseError::Internal(message.into())
    }
}

impl From<PasswordHashError> for DatabaseError {
    fn from(error: PasswordHashError) -> Self {
        DatabaseError::KeyDerivation(error.to_string())
    }
}

impl From<AeadError> for DatabaseError {
    fn from(error: AeadError) -> Self {
        DatabaseError::Encryption(error.to_string())
    }
}

#[derive(Debug, Error)]
pub enum MessagePackError {
    #[error("MessagePack encode error: {0}")]
    Encode(#[from] MessagePackEncodeError),
    #[error("MessagePack decode error: {0}")]
    Decode(#[from] MessagePackDecodeError),
    #[error("I/O error while {context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Serialized data too large: {size} bytes (max: {max} bytes)")]
    PayloadTooLarge { size: usize, max: usize },
    #[error("Invalid state file header")]
    InvalidHeader,
    #[error("Invalid state file magic number")]
    InvalidMagic,
    #[error("Unsupported state file version: {version}")]
    UnsupportedVersion { version: u8 },
    #[error("State file length mismatch")]
    LengthMismatch,
    #[error("State file checksum failed")]
    ChecksumFailed,
    #[error("Failed to restore state from backup")]
    RestoreFailed,
    #[error("MessagePack internal error: {0}")]
    Internal(String),
}

impl MessagePackError {
    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        MessagePackError::Io {
            context: context.into(),
            source,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        MessagePackError::Internal(message.into())
    }
}

#[derive(Debug, Error)]
pub enum StoragePathsError {
    #[error("Application directory is not set")]
    AppDirectoryMissing,
    #[error("Failed to access directory {path}: {source}")]
    DirectoryAccess {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to create directory {path}: {source}")]
    DirectoryCreate {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to compute directory size for {path}: {source}")]
    DirectorySize {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Storage paths internal error: {0}")]
    Internal(String),
}

impl StoragePathsError {
    pub fn directory_access(path: PathBuf, source: std::io::Error) -> Self {
        StoragePathsError::DirectoryAccess { path, source }
    }

    pub fn directory_create(path: PathBuf, source: std::io::Error) -> Self {
        StoragePathsError::DirectoryCreate { path, source }
    }

    pub fn directory_size(path: PathBuf, source: std::io::Error) -> Self {
        StoragePathsError::DirectorySize { path, source }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        StoragePathsError::Internal(message.into())
    }
}

#[derive(Debug, Error)]
pub enum StorageRecoveryError {
    #[error("I/O error while {context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Backup file does not exist: {path}")]
    BackupMissing { path: PathBuf },
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Recovery strategy {strategy} failed: {reason}")]
    StrategyFailed { strategy: &'static str, reason: String },
    #[error("Storage recovery internal error: {0}")]
    Internal(String),
}

impl StorageRecoveryError {
    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        StorageRecoveryError::Io {
            context: context.into(),
            source,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        StorageRecoveryError::Internal(message.into())
    }
}

#[derive(Debug, Error)]
pub enum StorageCoordinatorError {
    #[error(transparent)]
    Paths(#[from] StoragePathsError),
    #[error(transparent)]
    MessagePack(#[from] MessagePackError),
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Storage coordinator internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] SqlxError),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Timestamp parse error: {0}")]
    TimestampParse(#[from] chrono::ParseError),
    #[error("Query builder error: {0}")]
    Query(#[from] QueryBuilderError),

    #[error("AI model not found: {id}")]
    AiModelNotFound { id: String },
    #[error("AI feature not found: {name}")]
    AiFeatureNotFound { name: String },
    #[error("Command history entry not found: {id}")]
    CommandHistoryNotFound { id: String },
    #[error("Audit log entry not found: {id}")]
    AuditLogNotFound { id: String },

    #[error("AI model uses string identifiers; call {recommended} instead")]
    AiModelRequiresStringId { recommended: &'static str },
    #[error("AI feature uses string identifiers; call {recommended} instead")]
    AiFeatureRequiresStringId { recommended: &'static str },

    #[error("Audit logs do not support update operations")]
    AuditLogUpdateNotSupported,
    #[error("Command history entries do not support update operations")]
    CommandHistoryUpdateNotSupported,

    #[error("Unsupported number type")]
    UnsupportedNumberType,
    #[error("Unsupported parameter type: {name}")]
    UnsupportedParameterType { name: String },
    #[error("Repository validation error: {reason}")]
    Validation { reason: String },
    #[error("Repository internal error: {0}")]
    Internal(String),
}

impl RepositoryError {
    pub fn unsupported_parameter(name: impl Into<String>) -> Self {
        RepositoryError::UnsupportedParameterType {
            name: name.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        RepositoryError::Internal(message.into())
    }
}

#[derive(Debug, Error)]
pub enum QueryBuilderError {
    #[error("No fields specified for insert")]
    InsertFieldsEmpty,
    #[error("No fields specified for update")]
    UpdateFieldsEmpty,
    #[error("Query builder internal error: {0}")]
    Internal(String),
}

impl QueryBuilderError {
    pub fn internal(message: impl Into<String>) -> Self {
        QueryBuilderError::Internal(message.into())
    }
}

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum SqlScriptError {
    #[error("SQL directory does not exist: {path}")]
    DirectoryMissing { path: PathBuf },
    #[error("Failed to read SQL directory {path}: {source}")]
    ReadDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to traverse SQL directory {path}: {source}")]
    WalkDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Invalid SQL file name: {path}")]
    InvalidFileName { path: PathBuf },
    #[error("Failed to read SQL file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to parse file order from {filename}: {source}")]
    ParseOrder {
        filename: String,
        #[source]
        source: std::num::ParseIntError,
    },
    #[error("SQL filename does not start with order digits: {filename}")]
    MissingOrder { filename: String },
    #[error("SQL statement parsing error: {reason}")]
    ParseStatement { reason: String },
    #[error("SQL catalog is empty")]
    EmptyCatalog,
    #[error("SQL scripts internal error: {0}")]
    Internal(String),
}

impl SqlScriptError {
    pub fn read_directory(path: PathBuf, source: std::io::Error) -> Self {
        SqlScriptError::ReadDirectory { path, source }
    }

    pub fn walk_directory(path: PathBuf, source: std::io::Error) -> Self {
        SqlScriptError::WalkDirectory { path, source }
    }

    pub fn read_file(path: PathBuf, source: std::io::Error) -> Self {
        SqlScriptError::ReadFile { path, source }
    }

    pub fn parse_statement(reason: impl Into<String>) -> Self {
        SqlScriptError::ParseStatement {
            reason: reason.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        SqlScriptError::Internal(message.into())
    }
}
