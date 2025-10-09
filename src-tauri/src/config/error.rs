use std::path::PathBuf;
use std::sync::PoisonError;

use thiserror::Error;

pub type ConfigResult<T> = Result<T, ConfigError>;
pub type ConfigPathsResult<T> = Result<T, ConfigPathsError>;
pub type TomlConfigResult<T> = Result<T, TomlConfigError>;
pub type ThemeConfigResult<T> = Result<T, ThemeConfigError>;
pub type ShortcutsResult<T> = Result<T, ShortcutsError>;
pub type ShortcutsActionResult<T> = Result<T, ShortcutsActionError>;
pub type TerminalConfigResult<T> = Result<T, TerminalConfigError>;
pub type ConfigCommandResult<T> = Result<T, ConfigCommandError>;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(transparent)]
    Paths(#[from] ConfigPathsError),
    #[error(transparent)]
    Toml(#[from] TomlConfigError),
    #[error(transparent)]
    Theme(#[from] ThemeConfigError),
    #[error(transparent)]
    Shortcuts(#[from] ShortcutsError),
    #[error(transparent)]
    Terminal(#[from] TerminalConfigError),
    #[error(transparent)]
    Commands(#[from] ConfigCommandError),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Config internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum ConfigPathsError {
    #[error("Failed to determine user home directory")]
    HomeDirectoryUnavailable,
    #[error("Failed to determine configuration directory")]
    ConfigDirectoryUnavailable,
    #[error("Invalid configuration path: {path}")]
    InvalidPath { path: PathBuf },
    #[error("Path validation failed: {reason}")]
    Validation { reason: String },
    #[error("Failed to create directory {path}: {source}")]
    DirectoryCreate {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to access directory {path}: {source}")]
    DirectoryAccess {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl ConfigPathsError {
    pub fn directory_create(path: PathBuf, source: std::io::Error) -> Self {
        ConfigPathsError::DirectoryCreate { path, source }
    }

    pub fn directory_access(path: PathBuf, source: std::io::Error) -> Self {
        ConfigPathsError::DirectoryAccess { path, source }
    }

    pub fn invalid_path(path: PathBuf) -> Self {
        ConfigPathsError::InvalidPath { path }
    }

    pub fn validation(reason: impl Into<String>) -> Self {
        ConfigPathsError::Validation {
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum TomlConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("TOML serialization error: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("Configuration path is invalid")]
    InvalidPath,
    #[error("Bundled configuration file not found")]
    BundledConfigMissing,
    #[error("Configuration backup failed: {reason}")]
    BackupFailed { reason: String },
    #[error("Configuration validation error: {reason}")]
    Validation { reason: String },
    #[error("Configuration watcher error: {reason}")]
    Watcher { reason: String },
    #[error("TOML manager lock poisoned: {source}")]
    LockPoisoned {
        #[source]
        source: PoisonError<()>,
    },
    #[error("Toml configuration internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum ThemeConfigError {
    #[error("Theme file I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Theme parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("Theme index lock poisoned: {source}")]
    LockPoisoned {
        #[source]
        source: PoisonError<()>,
    },
    #[error("Theme not found: {name}")]
    NotFound { name: String },
    #[error("Theme validation error: {reason}")]
    Validation { reason: String },
    #[error("Theme watcher error: {reason}")]
    Watcher { reason: String },
    #[error("Theme internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum ShortcutsError {
    #[error(transparent)]
    Action(#[from] ShortcutsActionError),
    #[error("Shortcuts file I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Shortcuts parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("Shortcuts validation error: {reason}")]
    Validation { reason: String },
    #[error("Shortcut action not registered: {action}")]
    ActionNotRegistered { action: String },
    #[error("Shortcut conflicts detected: {detail}")]
    Conflict { detail: String },
    #[error("Shortcuts manager lock poisoned: {source}")]
    LockPoisoned {
        #[source]
        source: PoisonError<()>,
    },
    #[error("Shortcuts internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum ShortcutsActionError {
    #[error("Shortcut action already registered: {action}")]
    AlreadyRegistered { action: String },
    #[error("Shortcut action not registered: {action}")]
    NotRegistered { action: String },
    #[error("Shortcut action handler failed: {action}: {reason}")]
    HandlerFailed { action: String, reason: String },
}

#[derive(Debug, Error)]
pub enum TerminalConfigError {
    #[error("Terminal configuration validation error: {reason}")]
    Validation { reason: String },
    #[error("System shell detection error: {reason}")]
    ShellDetection { reason: String },
    #[error("Terminal configuration internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum ConfigCommandError {
    #[error("Configuration command validation error: {reason}")]
    Validation { reason: String },
    #[error("Dialog operation failed: {reason}")]
    Dialog { reason: String },
    #[error("Configuration command internal error: {0}")]
    Internal(String),
}

impl TomlConfigError {
    pub fn from_poison<T>(_err: PoisonError<T>) -> Self {
        TomlConfigError::LockPoisoned {
            source: PoisonError::new(()),
        }
    }
}

impl ThemeConfigError {
    pub fn from_poison<T>(_err: PoisonError<T>) -> Self {
        ThemeConfigError::LockPoisoned {
            source: PoisonError::new(()),
        }
    }
}

impl ShortcutsError {
    pub fn from_poison<T>(_err: PoisonError<T>) -> Self {
        ShortcutsError::LockPoisoned {
            source: PoisonError::new(()),
        }
    }
}

impl From<ConfigError> for ShortcutsError {
    fn from(error: ConfigError) -> Self {
        ShortcutsError::Internal(error.to_string())
    }
}
