/*!
 * Storage system type definitions module
 *
 * Defines core data types and interfaces used in the storage system
 */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Storage layer type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageLayer {
    Config,
    State,
    Data,
}

impl StorageLayer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::State => "state",
            Self::Data => "data",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 1200,
            height: 800,
            maximized: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerminalRuntimeKind {
    Workspace,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentTerminalMode {
    Blocking,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentTerminalStatus {
    Initializing,
    Running,
    Completed,
    Failed,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalRuntimeState {
    pub id: u32,
    pub cwd: String,
    pub shell: String,
    /// Pre-computed display title for the tab, calculated by the backend.
    pub display_title: String,
    pub kind: TerminalRuntimeKind,
    pub thread_id: Option<i64>,
    pub agent_terminal_id: Option<String>,
    pub source_label: Option<String>,
    pub agent_mode: Option<AgentTerminalMode>,
    pub agent_status: Option<AgentTerminalStatus>,
}

/// Configuration section type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigSection {
    /// Application configuration
    App,
    /// Appearance configuration
    Appearance,
    /// Terminal configuration
    Terminal,
    /// Shortcuts configuration
    Shortcuts,
    /// AI configuration
    Ai,
    /// Custom section
    Custom(String),
}

impl ConfigSection {
    pub fn as_str(&self) -> &str {
        match self {
            Self::App => "app",
            Self::Appearance => "appearance",
            Self::Terminal => "terminal",
            Self::Shortcuts => "shortcuts",
            Self::Ai => "ai",
            Self::Custom(name) => name,
        }
    }
}

impl From<&str> for ConfigSection {
    fn from(s: &str) -> Self {
        match s {
            "app" => Self::App,
            "appearance" => Self::Appearance,
            "terminal" => Self::Terminal,
            "shortcuts" => Self::Shortcuts,
            "ai" => Self::Ai,
            custom => Self::Custom(custom.to_string()),
        }
    }
}

impl From<String> for ConfigSection {
    fn from(s: String) -> Self {
        ConfigSection::from(s.as_str())
    }
}

/// Event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageEvent {
    /// Configuration changed event
    ConfigChanged {
        section: ConfigSection,
        old_value: Option<serde_json::Value>,
        new_value: serde_json::Value,
    },
    /// Data updated event
    DataUpdated {
        table: String,
        operation: String,
        affected_rows: usize,
    },
    /// Cache event
    CacheEvent { operation: String, key: String },
    /// Error event
    Error {
        layer: StorageLayer,
        error: String,
        timestamp: DateTime<Utc>,
    },
}

/// Storage event listener
pub trait StorageEventListener: Send + Sync {
    fn on_event(&self, event: StorageEvent);
}

/// Simple functional event listener
pub struct FunctionListener<F>
where
    F: Fn(StorageEvent) + Send + Sync,
{
    func: F,
}

impl<F> FunctionListener<F>
where
    F: Fn(StorageEvent) + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

impl<F> StorageEventListener for FunctionListener<F>
where
    F: Fn(StorageEvent) + Send + Sync,
{
    fn on_event(&self, event: StorageEvent) {
        (self.func)(event);
    }
}
