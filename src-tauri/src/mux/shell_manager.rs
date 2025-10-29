//! Shell detection and management

use serde::Serialize;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

use crate::mux::ConfigManager;

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

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellManagerStats {
    pub available_shells: usize,
    pub default_shell: Option<ShellInfo>,
    pub last_detection_time: Option<u64>,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

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

static SHELL_CACHE: OnceLock<Arc<Mutex<Option<ShellCacheEntry>>>> = OnceLock::new();

#[derive(Debug)]
pub struct ShellManager {
    stats: ShellManagerStats,
}

impl ShellManager {
    pub fn new() -> Self {
        let mut manager = Self {
            stats: ShellManagerStats::default(),
        };
        manager.update_stats();
        manager
    }

    fn update_stats(&mut self) {
        let cache = Self::get_cache();
        let cache_guard = cache.lock().unwrap();

        if let Some(entry) = cache_guard.as_ref() {
            self.stats.available_shells = entry.shells.len();
            self.stats.default_shell = Some(entry.default_shell.clone());
            self.stats.last_detection_time = Some(
                entry
                    .timestamp
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
        } else {
            drop(cache_guard);
            let _ = Self::get_cached_shells();
            self.update_stats();
        }
    }

    pub fn get_stats(&self) -> &ShellManagerStats {
        &self.stats
    }

    fn get_cache() -> &'static Arc<Mutex<Option<ShellCacheEntry>>> {
        SHELL_CACHE.get_or_init(|| Arc::new(Mutex::new(None)))
    }

    pub fn get_cached_shells() -> Vec<ShellInfo> {
        let cache = Self::get_cache();
        let mut cache_guard = cache.lock().unwrap();

        let config = ConfigManager::config_get();
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

    pub fn get_cached_default_shell() -> ShellInfo {
        let cache = Self::get_cache();
        let mut cache_guard = cache.lock().unwrap();

        let config = ConfigManager::config_get();
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
        cache_guard
            .as_ref()
            .map(|entry| entry.default_shell.clone())
            .unwrap_or_else(|| Self::get_default_shell_internal())
    }

    pub fn refresh_cache() {
        let cache = Self::get_cache();
        let mut cache_guard = cache.lock().unwrap();
        *cache_guard = None;
        drop(cache_guard);

        debug!("Shell缓存已清空，下次访问时将重新检测");
    }

    /// 检查缓存状态
    pub fn cache_status() -> (bool, Option<SystemTime>, u64) {
        let cache = Self::get_cache();
        let cache_guard = cache.lock().unwrap();

        if let Some(entry) = cache_guard.as_ref() {
            let config = ConfigManager::config_get();
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
        let config = ConfigManager::config_get();

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
            // 使用平台特定的PATH分隔符
            let path_separator = if cfg!(windows) { ';' } else { ':' };
            for path_dir in path_env.split(path_separator) {
                // 根据平台选择要搜索的shell
                let shell_names = if cfg!(windows) {
                    &["bash.exe", "zsh.exe", "fish.exe"][..]
                } else {
                    &["zsh", "bash", "fish"][..]
                };

                for shell_name in shell_names {
                    // 使用PathBuf来正确处理路径连接
                    let shell_path = std::path::PathBuf::from(path_dir)
                        .join(shell_name)
                        .to_string_lossy()
                        .to_string();

                    if Self::validate_shell(&shell_path)
                        && !shells.iter().any(|s| s.path == shell_path)
                    {
                        let base_name = if cfg!(windows) {
                            shell_name.strip_suffix(".exe").unwrap_or(shell_name)
                        } else {
                            shell_name
                        };

                        let display_name = match base_name {
                            "zsh" => "Zsh",
                            "bash" => "Bash",
                            "fish" => "Fish",
                            _ => shell_name,
                        };
                        shells.push(ShellInfo::new(base_name, &shell_path, display_name));
                    }
                }
            }
        }

        debug!("检测到 {} 个可用shell", shells.len());
        shells
    }

    /// 获取默认shell（公共接口，使用缓存）
    pub fn terminal_get_default_shell() -> ShellInfo {
        Self::get_cached_default_shell()
    }

    /// 内部默认shell获取实现（不使用缓存）
    fn get_default_shell_internal() -> ShellInfo {
        #[cfg(windows)]
        {
            // Windows平台的默认shell检测
            let windows_shells = [
                ("bash", "C:\\Program Files\\Git\\bin\\bash.exe", "Git Bash"),
                (
                    "bash",
                    "C:\\Program Files\\Git\\usr\\bin\\bash.exe",
                    "Git Bash",
                ),
                ("zsh", "C:\\Program Files\\Git\\usr\\bin\\zsh.exe", "Zsh"),
                ("fish", "C:\\Program Files\\Git\\usr\\bin\\fish.exe", "Fish"),
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

        exists && is_executable
    }

    /// 根据名称查找shell（使用缓存）
    pub fn terminal_find_shell_by_name(name: &str) -> Option<ShellInfo> {
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
    pub fn terminal_find_shell_by_path(path: &str) -> Option<ShellInfo> {
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
                entry
                    .timestamp
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
