/*!
 * 核心数据类型定义
 */

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

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
            // 使用安全的默认值，避免循环依赖
            program: Self::get_safe_default_shell(),
            args: Vec::new(),
            working_directory: None,
            env: None,
        }
    }
}

impl ShellConfig {
    /// 获取安全的默认shell路径，避免循环依赖
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

    /// 使用默认shell创建配置（推荐使用此方法而非Default）
    pub fn with_default_shell() -> Self {
        Self {
            program: ShellManager::get_default_shell().path,
            args: Vec::new(),
            working_directory: None,
            env: None,
        }
    }

    /// 使用指定shell创建配置
    pub fn with_shell(shell_info: &ShellInfo) -> Self {
        Self {
            program: shell_info.path.clone(),
            args: Vec::new(),
            working_directory: None,
            env: None,
        }
    }

    /// 使用指定shell路径创建配置
    pub fn with_shell_path(path: String) -> Self {
        Self {
            program: path,
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
    pub cache_hits: u64,
    pub cache_misses: u64,
}

/// Shell缓存条目
#[derive(Debug, Clone)]
struct ShellCacheEntry {
    shells: Vec<ShellInfo>,
    default_shell: ShellInfo,
    timestamp: SystemTime,
    access_count: u64,
}

impl ShellCacheEntry {
    fn new(shells: Vec<ShellInfo>, default_shell: ShellInfo) -> Self {
        Self {
            shells,
            default_shell,
            timestamp: SystemTime::now(),
            access_count: 0,
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed().unwrap_or(Duration::MAX) > ttl
    }

    fn access(&mut self) {
        self.access_count += 1;
    }
}

/// 全局Shell缓存
static SHELL_CACHE: OnceLock<Arc<Mutex<Option<ShellCacheEntry>>>> = OnceLock::new();

/// Shell管理器
#[derive(Debug)]
pub struct ShellManager {
    stats: ShellManagerStats,
}

use crate::mux::ConfigManager;

impl ShellManager {
    pub fn new() -> Self {
        let mut manager = Self {
            stats: ShellManagerStats::default(),
        };
        manager.update_stats();
        manager
    }

    /// 更新统计信息
    fn update_stats(&mut self) {
        let cache = Self::get_cache();
        let cache_guard = cache.lock().unwrap();

        if let Some(entry) = cache_guard.as_ref() {
            self.stats.available_shells = entry.shells.len();
            self.stats.default_shell = Some(entry.default_shell.clone());
            self.stats.last_detection_time = Some(
                entry.timestamp
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
        } else {
            // 如果缓存为空，触发检测
            drop(cache_guard);
            let _ = Self::get_cached_shells();
            self.update_stats();
        }
    }

    pub fn get_stats(&self) -> &ShellManagerStats {
        &self.stats
    }

    /// 获取缓存实例
    fn get_cache() -> &'static Arc<Mutex<Option<ShellCacheEntry>>> {
        SHELL_CACHE.get_or_init(|| Arc::new(Mutex::new(None)))
    }

    /// 获取缓存的shell列表
    pub fn get_cached_shells() -> Vec<ShellInfo> {
        let cache = Self::get_cache();
        let mut cache_guard = cache.lock().unwrap();

        // 检查缓存是否存在且未过期
        let config = ConfigManager::get_config();
        if let Some(entry) = cache_guard.as_mut() {
            if !entry.is_expired(config.shell_cache_ttl()) {
                entry.access();
                debug!("Shell缓存命中，返回 {} 个shell", entry.shells.len());
                return entry.shells.clone();
            } else {
                debug!("Shell缓存已过期，重新检测");
            }
        } else {
            debug!("Shell缓存为空，首次检测");
        }

        // 缓存过期或不存在，重新检测
        info!("开始检测可用shell");
        let shells = Self::detect_available_shells_internal();
        let default_shell = Self::get_default_shell_internal();

        // 更新缓存
        *cache_guard = Some(ShellCacheEntry::new(shells.clone(), default_shell));

        info!("Shell检测完成，发现 {} 个可用shell", shells.len());
        shells
    }

    /// 获取缓存的默认shell
    pub fn get_cached_default_shell() -> ShellInfo {
        let cache = Self::get_cache();
        let mut cache_guard = cache.lock().unwrap();

        // 检查缓存是否存在且未过期
        let config = ConfigManager::get_config();
        if let Some(entry) = cache_guard.as_mut() {
            if !entry.is_expired(config.shell_cache_ttl()) {
                entry.access();
                debug!("默认shell缓存命中: {}", entry.default_shell.name);
                return entry.default_shell.clone();
            }
        }

        // 缓存过期或不存在，重新检测
        drop(cache_guard);
        let _ = Self::get_cached_shells(); // 这会更新缓存

        // 重新获取
        let cache_guard = cache.lock().unwrap();
        cache_guard.as_ref()
            .map(|entry| entry.default_shell.clone())
            .unwrap_or_else(|| Self::get_default_shell_internal())
    }

    /// 强制刷新缓存
    pub fn refresh_cache() {
        let cache = Self::get_cache();
        let mut cache_guard = cache.lock().unwrap();
        *cache_guard = None;
        drop(cache_guard);

        info!("Shell缓存已清空，下次访问时将重新检测");
    }

    /// 检查缓存状态
    pub fn cache_status() -> (bool, Option<SystemTime>, u64) {
        let cache = Self::get_cache();
        let cache_guard = cache.lock().unwrap();

        if let Some(entry) = cache_guard.as_ref() {
            let config = ConfigManager::get_config();
            (
                !entry.is_expired(config.shell_cache_ttl()),
                Some(entry.timestamp),
                entry.access_count,
            )
        } else {
            (false, None, 0)
        }
    }

    /// 检测系统上可用的shell（公共接口，使用缓存）
    pub fn detect_available_shells() -> Vec<ShellInfo> {
        Self::get_cached_shells()
    }

    /// 内部shell检测实现（不使用缓存）
    fn detect_available_shells_internal() -> Vec<ShellInfo> {
        debug!("执行shell检测");
        let mut shells = Vec::new();
        let config = ConfigManager::get_config();

        // 从配置获取shell路径
        let shell_paths = if cfg!(windows) {
            &config.shell.default_paths.windows
        } else {
            &config.shell.default_paths.unix
        };

        // 检测配置中的shell路径
        for path in shell_paths {
            if Self::validate_shell(path) {
                if let Some(shell_name) = std::path::Path::new(path).file_name() {
                    if let Some(name_str) = shell_name.to_str() {
                        let display_name = Self::get_shell_display_name(name_str);

                        // 避免重复添加相同名称的shell
                        if !shells.iter().any(|s: &ShellInfo| s.name == name_str) {
                            shells.push(ShellInfo::new(name_str, path, display_name));
                        }
                    }
                }
            }
        }

        // 尝试从PATH环境变量中查找其他shell
        if let Ok(path_env) = std::env::var("PATH") {
            for path_dir in path_env.split(':') {
                for shell_name in &["zsh", "bash", "fish", "tcsh", "csh"] {
                    let shell_path = format!("{}/{}", path_dir, shell_name);
                    if Self::validate_shell(&shell_path) && !shells.iter().any(|s| s.path == shell_path) {
                        let display_name = match *shell_name {
                            "zsh" => "Zsh",
                            "bash" => "Bash",
                            "fish" => "Fish",
                            "tcsh" => "Tcsh",
                            "csh" => "Csh",
                            _ => shell_name,
                        };
                        shells.push(ShellInfo::new(shell_name, &shell_path, display_name));
                    }
                }
            }
        }

        debug!("检测到 {} 个可用shell", shells.len());
        shells
    }

    /// 获取默认shell（公共接口，使用缓存）
    pub fn get_default_shell() -> ShellInfo {
        Self::get_cached_default_shell()
    }

    /// 内部默认shell获取实现（不使用缓存）
    fn get_default_shell_internal() -> ShellInfo {
        #[cfg(windows)]
        {
            // Windows平台的默认shell检测
            let windows_shells = [
                ("bash", "C:\\Program Files\\Git\\bin\\bash.exe", "Git Bash"),
                ("powershell", "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe", "PowerShell"),
                ("cmd", "C:\\Windows\\System32\\cmd.exe", "Command Prompt"),
            ];

            for (name, path, display_name) in &windows_shells {
                if Self::validate_shell(path) {
                    return ShellInfo::new(name, path, display_name);
                }
            }

            // 备选方案
            ShellInfo::new("cmd", "C:\\Windows\\System32\\cmd.exe", "Command Prompt")
        }

        #[cfg(not(windows))]
        {
            // 首先尝试从环境变量获取默认shell
            if let Ok(shell_path) = std::env::var("SHELL") {
                if Self::validate_shell(&shell_path) {
                    // 从路径中提取shell名称
                    if let Some(shell_name) = std::path::Path::new(&shell_path).file_name() {
                        if let Some(name_str) = shell_name.to_str() {
                            let display_name = Self::get_shell_display_name(name_str);
                            debug!("从环境变量获取默认shell: {} -> {}", name_str, shell_path);
                            return ShellInfo::new(name_str, &shell_path, display_name);
                        }
                    }
                }
            }

            // 如果环境变量不可用，尝试检测常见的默认shell
            let preferred_shells = [
                ("zsh", "/bin/zsh", "Zsh"),
                ("bash", "/bin/bash", "Bash"),
                ("fish", "/usr/bin/fish", "Fish"),
                ("sh", "/bin/sh", "sh"),
            ];

            for (name, path, display_name) in &preferred_shells {
                if Self::validate_shell(path) {
                    debug!("使用默认shell: {} -> {}", name, path);
                    return ShellInfo::new(name, path, display_name);
                }
            }

            // 最后的备选方案
            warn!("未找到任何可用的shell，使用备选方案");
            ShellInfo::new("bash", "/bin/bash", "Bash")
        }
    }

    /// 获取shell的显示名称
    fn get_shell_display_name(name: &str) -> &'static str {
        match name {
            "zsh" => "Zsh",
            "bash" => "Bash",
            "fish" => "Fish",
            "sh" => "sh",
            "tcsh" => "Tcsh",
            "csh" => "Csh",
            "powershell" => "PowerShell",
            "cmd" => "Command Prompt",
            _ => "Unknown Shell",
        }
    }

    /// 验证shell是否可用
    pub fn validate_shell(path: &str) -> bool {
        if path.trim().is_empty() {
            return false;
        }

        let path_obj = std::path::Path::new(path);
        let exists = path_obj.exists();
        let is_executable = path_obj.is_file();

        debug!("验证shell路径: {} -> exists: {}, executable: {}", path, exists, is_executable);
        exists && is_executable
    }

    /// 根据名称查找shell（使用缓存）
    pub fn find_shell_by_name(name: &str) -> Option<ShellInfo> {
        if name.trim().is_empty() {
            return None;
        }

        debug!("根据名称查找shell: {}", name);
        let shells = Self::get_cached_shells();
        let result = shells.into_iter().find(|shell| shell.name == name);

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

    /// 根据路径查找shell（使用缓存）
    pub fn find_shell_by_path(path: &str) -> Option<ShellInfo> {
        if path.trim().is_empty() {
            return None;
        }

        debug!("根据路径查找shell: {}", path);
        let shells = Self::get_cached_shells();
        let result = shells.into_iter().find(|shell| shell.path == path);

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

    /// 获取shell管理器的详细统计信息
    pub fn get_detailed_stats() -> ShellManagerStats {
        let cache = Self::get_cache();
        let cache_guard = cache.lock().unwrap();

        let mut stats = ShellManagerStats::default();

        if let Some(entry) = cache_guard.as_ref() {
            stats.available_shells = entry.shells.len();
            stats.default_shell = Some(entry.default_shell.clone());
            stats.last_detection_time = Some(
                entry.timestamp
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
            stats.cache_hits = entry.access_count;
        }

        stats
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
