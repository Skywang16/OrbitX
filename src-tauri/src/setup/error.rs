use crate::config::error::{
    ConfigError, ConfigPathsError, ShortcutsError, ThemeConfigError,
};
use crate::storage::error::StorageCoordinatorError;
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
    #[error("Storage coordinator initialization failed: {0}")]
    StorageCoordinator(#[from] StorageCoordinatorError),
    #[error("Theme service initialization failed: {0}")]
    Theme(#[from] ThemeConfigError),
    #[error("AI manager creation failed: {0}")]
    AIState(String),
    #[error("AI manager initialization failed: {0}")]
    AIInitialization(String),
    #[error("Window state initialization failed: {0}")]
    WindowState(#[from] WindowStateError),
}
