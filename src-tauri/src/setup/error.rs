use crate::config::error::{ConfigError, ConfigPathsError, ShortcutsError, ThemeConfigError};
use crate::settings::SettingsError;
use crate::storage::error::MessagePackError;
use crate::window::error::WindowStateError;
use thiserror::Error;

pub type SetupResult<T> = Result<T, SetupError>;

#[derive(Debug, Error)]
pub enum SetupError {
    #[error("Terminal state initialization failed: {0}")]
    TerminalState(String),
    #[error("Config paths initialization failed: {0}")]
    ConfigPaths(#[from] ConfigPathsError),
    #[error("Config manager initialization failed: {0}")]
    Config(#[from] ConfigError),
    #[error("Shortcut manager initialization failed: {0}")]
    Shortcuts(#[from] ShortcutsError),
    #[error("Settings manager initialization failed: {0}")]
    Settings(#[from] SettingsError),
    #[error("Storage paths initialization failed: {0}")]
    StoragePaths(#[from] crate::storage::error::StoragePathsError),
    #[error("Database error: {0}")]
    Database(#[from] crate::storage::error::DatabaseError),
    #[error("MessagePack initialization failed: {0}")]
    MessagePack(#[from] MessagePackError),
    #[error("Theme service initialization failed: {0}")]
    Theme(#[from] ThemeConfigError),
    #[error("AI manager creation failed: {0}")]
    AIState(String),
    #[error("AI manager initialization failed: {0}")]
    AIInitialization(String),
    #[error("Window state initialization failed: {0}")]
    WindowState(#[from] WindowStateError),
}

impl From<&str> for SetupError {
    fn from(s: &str) -> Self {
        SetupError::TerminalState(s.to_string())
    }
}
