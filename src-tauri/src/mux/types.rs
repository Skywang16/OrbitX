//! 核心数据类型定义

use bytes::Bytes;
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
pub struct TerminalConfig {
    pub shell_config: ShellConfig,
}

impl TerminalConfig {
    pub fn with_shell(shell_config: ShellConfig) -> Self {
        Self {
            shell_config,
            ..Default::default()
        }
    }
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            shell_config: ShellConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellConfig {
    pub program: String,
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub env: Option<HashMap<String, String>>,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            // 使用安全的默认值，避免循环依赖
            program: Self::get_safe_default_shell(),
            args: Vec::new(),
            working_directory: None,
            env: None,
        }
    }
}

impl ShellConfig {
    fn get_safe_default_shell() -> String {
        #[cfg(windows)]
        {
            "C:\\Program Files\\Git\\bin\\bash.exe".to_string()
        }
        #[cfg(not(windows))]
        {
            // 优先使用环境变量，然后是常见路径
            std::env::var("SHELL")
                .ok()
                .filter(|path| std::path::Path::new(path).exists())
                .unwrap_or_else(|| {
                    // 按优先级检查常见shell
                    let shells = ["/bin/zsh", "/bin/bash", "/bin/sh"];
                    shells
                        .iter()
                        .find(|&&path| std::path::Path::new(path).exists())
                        .map(|&path| path.to_string())
                        .unwrap_or_else(|| "/bin/bash".to_string())
                })
        }
    }

    pub fn with_default_shell() -> Self {
        Self {
            program: ShellManager::terminal_get_default_shell().path,
            args: Vec::new(),
            working_directory: None,
            env: None,
        }
    }

    pub fn with_shell(shell_info: &ShellInfo) -> Self {
        Self {
            program: shell_info.path.clone(),
            args: Vec::new(),
            working_directory: None,
            env: None,
        }
    }

    pub fn with_shell_path(path: String) -> Self {
        Self {
            program: path,
            args: Vec::new(),
            working_directory: None,
            env: None,
        }
    }
}

/// Mux通知事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::enum_variant_names)]
pub enum MuxNotification {
    /// 面板输出数据
    PaneOutput { pane_id: PaneId, data: Bytes },
    /// 面板已添加
    PaneAdded(PaneId),
    /// 面板已移除
    PaneRemoved(PaneId),
    /// 面板大小已调整
    PaneResized { pane_id: PaneId, size: PtySize },
    /// 面板进程已退出
    PaneExited {
        pane_id: PaneId,
        exit_code: Option<i32>,
    },
    /// 面板工作目录已变化
    PaneCwdChanged { pane_id: PaneId, cwd: String },
}
