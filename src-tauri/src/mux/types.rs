//! 核心数据类型定义

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

    pub fn with_pixels(rows: u16, cols: u16, pixel_width: u16, pixel_height: u16) -> Self {
        Self {
            rows,
            cols,
            pixel_width,
            pixel_height,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct TerminalConfig {
    pub shell_config: ShellConfig,
}

impl TerminalConfig {
    pub fn with_shell(shell_config: ShellConfig) -> Self {
        Self { shell_config }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellConfig {
    pub shell_info: ShellInfo,
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub env: Option<HashMap<String, String>>,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            shell_info: ShellManager::terminal_get_default_shell(),
            args: Vec::new(),
            working_directory: None,
            env: None,
        }
    }
}

impl ShellConfig {
    pub fn with_default_shell() -> Self {
        Self {
            shell_info: ShellManager::terminal_get_default_shell(),
            args: Vec::new(),
            working_directory: None,
            env: None,
        }
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

// MuxNotification 已移至 crate::events::mux 模块
