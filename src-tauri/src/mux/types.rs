//! 核心数据类型定义

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info};

/// 面板唯一标识符
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

/// PTY 终端尺寸
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

/// 面板信息
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

/// 终端配置
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalConfig {
    pub shell_config: ShellConfig,
    pub buffer_size: usize,
    pub batch_size: usize,
    pub flush_interval_ms: u64,
}

impl TerminalConfig {
    /// 创建带有指定shell的配置
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
            buffer_size: 1024 * 10, // 10KB
            batch_size: 512,
            flush_interval_ms: 10,
        }
    }
}

/// Shell 配置
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
            program: ShellManager::get_default_shell().path,
            args: Vec::new(),
            working_directory: None,
            env: None,
        }
    }
}

/// Shell 信息
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellInfo {
    pub name: String,
    pub path: String,
    pub display_name: String,
}

impl ShellInfo {
    pub fn new(name: &str, path: &str, display_name: &str) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_string(),
            display_name: display_name.to_string(),
        }
    }
}

/// Shell管理器统计信息
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellManagerStats {
    pub available_shells: usize,
    pub default_shell: Option<ShellInfo>,
    pub last_detection_time: Option<u64>,
}

/// Shell管理器
#[derive(Debug)]
pub struct ShellManager {
    available_shells: Vec<ShellInfo>,
    stats: ShellManagerStats,
}

impl ShellManager {
    pub fn new() -> Self {
        let mut manager = Self {
            available_shells: Vec::new(),
            stats: ShellManagerStats::default(),
        };
        manager.detect_and_set_shells();
        manager
    }

    /// 检测并设置可用shell
    fn detect_and_set_shells(&mut self) {
        let start_time = std::time::SystemTime::now();
        self.available_shells = Self::detect_available_shells();
        self.stats.available_shells = self.available_shells.len();
        self.stats.default_shell = Some(Self::get_default_shell());
        self.stats.last_detection_time = Some(
            start_time
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }

    pub fn get_stats(&self) -> &ShellManagerStats {
        &self.stats
    }

    /// 检测系统上可用的shell
    pub fn detect_available_shells() -> Vec<ShellInfo> {
        info!("开始检测可用shell");
        let mut shells = Vec::new();

        // 添加常见的shell
        let common_shells = [
            ("zsh", "/bin/zsh", "Zsh"),
            ("bash", "/bin/bash", "Bash"),
            ("fish", "/usr/bin/fish", "Fish"),
            // macOS specific
            ("sh", "/bin/sh", "sh"),
        ];

        for (name, path, display_name) in &common_shells {
            if Self::validate_shell(path) {
                shells.push(ShellInfo::new(name, path, display_name));
            }
        }

        info!("检测到可用shell: {:?}", shells);
        shells
    }

    /// 获取默认shell
    pub fn get_default_shell() -> ShellInfo {
        #[cfg(windows)]
        {
            ShellInfo::new("bash", "C:\\Program Files\\Git\\bin\\bash.exe", "Bash")
        }

        #[cfg(not(windows))]
        {
            // 首先尝试从环境变量获取默认shell
            if let Ok(shell_path) = std::env::var("SHELL") {
                if Self::validate_shell(&shell_path) {
                    // 从路径中提取shell名称
                    if let Some(shell_name) = std::path::Path::new(&shell_path).file_name() {
                        if let Some(name_str) = shell_name.to_str() {
                            let display_name = match name_str {
                                "zsh" => "Zsh",
                                "bash" => "Bash",
                                "fish" => "Fish",
                                "sh" => "sh",
                                _ => name_str,
                            };
                            return ShellInfo::new(name_str, &shell_path, display_name);
                        }
                    }
                }
            }

            // 如果环境变量不可用，尝试检测常见的默认shell
            let preferred_shells = [
                ("zsh", "/bin/zsh", "Zsh"),
                ("bash", "/bin/bash", "Bash"),
                ("sh", "/bin/sh", "sh"),
            ];

            for (name, path, display_name) in &preferred_shells {
                if Self::validate_shell(path) {
                    return ShellInfo::new(name, path, display_name);
                }
            }

            // 最后的备选方案
            ShellInfo::new("bash", "/bin/bash", "Bash")
        }
    }

    /// 验证shell是否可用
    pub fn validate_shell(path: &str) -> bool {
        debug!("验证shell路径: {}", path);
        let exists = std::path::Path::new(path).exists();
        debug!("Shell路径验证结果: {} -> {}", path, exists);
        exists
    }

    /// 根据名称查找shell
    pub fn find_shell_by_name(name: &str) -> Option<ShellInfo> {
        debug!("根据名称查找shell: {}", name);
        let result = Self::detect_available_shells()
            .into_iter()
            .find(|shell| shell.name == name);

        match &result {
            Some(shell) => {
                debug!("找到shell: {} -> {}", shell.name, shell.path);
            }
            None => {
                debug!("未找到shell: {}", name);
            }
        }

        result
    }

    /// 根据路径查找shell
    pub fn find_shell_by_path(path: &str) -> Option<ShellInfo> {
        debug!("根据路径查找shell: {}", path);
        let result = Self::detect_available_shells()
            .into_iter()
            .find(|shell| shell.path == path);

        match &result {
            Some(shell) => {
                debug!("找到shell: {} -> {}", shell.name, shell.path);
            }
            None => {
                debug!("未找到shell: {}", path);
            }
        }

        result
    }
}

impl Default for ShellManager {
    fn default() -> Self {
        Self::new()
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
}

// ===== 前端事件结构体 =====

/// 终端输出事件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputEvent {
    pub pane_id: PaneId,
    pub data: String,
}

/// 终端创建事件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalCreatedEvent {
    pub pane_id: PaneId,
}

/// 终端关闭事件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalClosedEvent {
    pub pane_id: PaneId,
}

/// 终端大小调整事件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalResizedEvent {
    pub pane_id: PaneId,
    pub rows: u16,
    pub cols: u16,
}

/// 终端退出事件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalExitEvent {
    pub pane_id: PaneId,
    pub exit_code: Option<i32>,
}
