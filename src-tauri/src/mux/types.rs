//! Core data type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::shell_manager::{ShellInfo, ShellManager};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneId(pub u32);

impl PaneId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl From<u32> for PaneId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<PaneId> for u32 {
    fn from(pane_id: PaneId) -> Self {
        pane_id.0
    }
}

impl std::fmt::Display for PaneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl PtySize {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

impl Default for PtySize {
    fn default() -> Self {
        Self::new(24, 80)
    }
}

#[derive(Debug, Clone)]
pub struct PaneInfo {
    pub pane_id: PaneId,
    pub size: PtySize,
    pub title: String,
    pub working_directory: Option<PathBuf>,
    pub exit_code: Option<i32>,
}

impl PaneInfo {
    pub fn new(pane_id: PaneId, size: PtySize) -> Self {
        Self {
            pane_id,
            size,
            title: String::new(),
            working_directory: None,
            exit_code: None,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PaneRuntimeMetadata {
    pub thread_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MuxSessionConfig {
    pub shell_config: MuxShellConfig,
    #[serde(default)]
    pub runtime_metadata: PaneRuntimeMetadata,
}

impl MuxSessionConfig {
    pub fn with_default_shell() -> Result<Self, String> {
        Ok(Self {
            shell_config: MuxShellConfig::with_default_shell()?,
            runtime_metadata: PaneRuntimeMetadata::default(),
        })
    }

    pub fn with_shell(shell_config: MuxShellConfig) -> Self {
        Self {
            shell_config,
            runtime_metadata: PaneRuntimeMetadata::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MuxShellConfig {
    pub shell_info: ShellInfo,
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub env: Option<HashMap<String, String>>,
}

impl MuxShellConfig {
    pub fn with_default_shell() -> Result<Self, String> {
        Ok(Self {
            shell_info: ShellManager::terminal_get_default_shell()?,
            args: Vec::new(),
            working_directory: None,
            env: None,
        })
    }

    pub fn with_shell(shell_info: ShellInfo) -> Self {
        Self {
            shell_info,
            args: Vec::new(),
            working_directory: None,
            env: None,
        }
    }
}

// MuxNotification has been moved to crate::events::mux module
