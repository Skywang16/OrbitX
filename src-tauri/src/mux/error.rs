use std::sync::PoisonError;

use crossbeam_channel::SendError;
use thiserror::Error;

use super::{PaneId, PtySize};

pub type MuxResult<T> = Result<T, MuxError>;
pub type TerminalMuxResult<T> = Result<T, TerminalMuxError>;
pub type PaneResult<T> = Result<T, PaneError>;
pub type IoHandlerResult<T> = Result<T, IoHandlerError>;
pub type MuxConfigResult<T> = Result<T, MuxConfigError>;

#[derive(Debug, Error)]
pub enum MuxError {
    #[error(transparent)]
    TerminalMux(#[from] TerminalMuxError),
    #[error(transparent)]
    Pane(#[from] PaneError),
    #[error(transparent)]
    IoHandler(#[from] IoHandlerError),
    #[error(transparent)]
    Config(#[from] MuxConfigError),
    #[error("Shell integration error: {0}")]
    ShellIntegration(String),
    #[error("Mux internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum TerminalMuxError {
    #[error("Failed to acquire read lock for {context}: {source}")]
    ReadLockPoisoned {
        context: &'static str,
        #[source]
        source: PoisonError<()>,
    },
    #[error("Failed to acquire write lock for {context}: {source}")]
    WriteLockPoisoned {
        context: &'static str,
        #[source]
        source: PoisonError<()>,
    },
    #[error("Pane already exists: {pane_id}")]
    PaneExists { pane_id: PaneId },
    #[error("Pane does not exist: {pane_id}")]
    PaneNotFound { pane_id: PaneId },
    #[error("Subscriber does not exist: {subscriber_id}")]
    SubscriberNotFound { subscriber_id: usize },
    #[error("Notification channel closed")]
    NotificationChannelClosed,
    #[error("Notification send error: {0}")]
    NotificationSend(String),
    #[error("Invalid pane size: {size:?}")]
    InvalidPaneSize { size: PtySize },
    #[error("Mux is shutting down")]
    ShuttingDown,
    #[error("Mux internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum PaneError {
    #[error("PTY operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to acquire pane lock: {context}: {source}")]
    LockPoisoned {
        context: &'static str,
        #[source]
        source: PoisonError<()>,
    },
    #[error("Pane writer unavailable")]
    WriterUnavailable,
    #[error("Pane reader unavailable")]
    ReaderUnavailable,
    #[error("Pane is marked dead")]
    PaneDead,
    #[error("Shell process spawn failed: {reason}")]
    Spawn { reason: String },
    #[error("Pane internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum IoHandlerError {
    #[error("Pane reader unavailable: {reason}")]
    PaneReader { reason: String },
    #[error("Notification send error: {0}")]
    NotificationSend(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("IoHandler internal error: {0}")]
    Internal(String),
}

impl<T> From<SendError<T>> for IoHandlerError
where
    T: std::fmt::Debug,
{
    fn from(err: SendError<T>) -> Self {
        IoHandlerError::NotificationSend(err.to_string())
    }
}

#[derive(Debug, Error)]
pub enum MuxConfigError {
    #[error("Failed to read config file {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to write config file {path}: {source}")]
    WriteFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to parse config file {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("Failed to serialize config: {0}")]
    Serialize(String),
    #[error("Configuration validation failed: {reason}")]
    Validation { reason: String },
    #[error("Mux config internal error: {0}")]
    Internal(String),
}

impl PaneError {
    pub fn from_poison<T>(context: &'static str, _err: PoisonError<T>) -> Self {
        PaneError::LockPoisoned {
            context,
            source: PoisonError::new(()),
        }
    }
}

impl TerminalMuxError {
    pub fn from_read_poison<T>(context: &'static str, _err: PoisonError<T>) -> Self {
        TerminalMuxError::ReadLockPoisoned {
            context,
            source: PoisonError::new(()),
        }
    }

    pub fn from_write_poison<T>(context: &'static str, _err: PoisonError<T>) -> Self {
        TerminalMuxError::WriteLockPoisoned {
            context,
            source: PoisonError::new(()),
        }
    }
}

impl From<PaneError> for TerminalMuxError {
    fn from(error: PaneError) -> Self {
        TerminalMuxError::Internal(error.to_string())
    }
}

impl From<IoHandlerError> for TerminalMuxError {
    fn from(error: IoHandlerError) -> Self {
        TerminalMuxError::Internal(error.to_string())
    }
}
