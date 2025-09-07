//! 终端系统配置管理
//!
//! 提供灵活的配置管理机制，支持文件、环境变量等多种配置源

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tracing::{debug, info, warn};

/// 终端系统配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSystemConfig {
    /// 缓冲区配置
    pub buffer: BufferConfig,
    /// Shell配置
    pub shell: ShellSystemConfig,
    /// 性能配置
    pub performance: PerformanceConfig,
    /// 清理配置
    pub cleanup: CleanupConfig,
}

/// 缓冲区配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BufferConfig {
    /// 最大缓冲区大小（字节）
    pub max_size: usize,
    /// 保留缓冲区大小（字节）
    pub keep_size: usize,
    /// 最大截断尝试次数
    pub max_truncation_attempts: usize,
    /// 批处理大小
    pub batch_size: usize,
    /// 刷新间隔（毫秒）
    pub flush_interval_ms: u64,
}

/// Shell系统配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellSystemConfig {
    /// 缓存TTL（秒）
    pub cache_ttl_seconds: u64,
    /// 最大缓存年龄（秒）
    pub max_cache_age_seconds: u64,
    /// 默认shell路径（按平台）
    pub default_paths: DefaultShellPaths,
}

/// 默认Shell路径配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultShellPaths {
    /// Unix系统的默认shell路径
    pub unix: Vec<String>,
    /// Windows系统的默认shell路径
    pub windows: Vec<String>,
}

/// 性能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceConfig {
    /// 工作线程数
    pub worker_threads: Option<usize>,
    /// 最大并发连接数
    pub max_concurrent_connections: usize,
    /// 超时配置（毫秒）
    pub timeouts: TimeoutConfig,
}

/// 超时配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeoutConfig {
    /// 命令执行超时
    pub command_execution_ms: u64,
    /// 连接超时
    pub connection_ms: u64,
    /// 读取超时
    pub read_ms: u64,
    /// 写入超时
    pub write_ms: u64,
}

/// 清理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupConfig {
    /// 清理间隔（秒）
    pub interval_seconds: u64,
    /// 过期阈值（秒）
    pub stale_threshold_seconds: u64,
    /// 是否启用自动清理
    pub auto_cleanup_enabled: bool,
}

impl Default for TerminalSystemConfig {
    fn default() -> Self {
        Self {
            buffer: BufferConfig::default(),
            shell: ShellSystemConfig::default(),
            performance: PerformanceConfig::default(),
            cleanup: CleanupConfig::default(),
        }
    }
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            max_size: 50_000,
            keep_size: 25_000,
            max_truncation_attempts: 1000,
            batch_size: 512,
            flush_interval_ms: 10,
        }
    }
}

impl Default for ShellSystemConfig {
    fn default() -> Self {
        Self {
            cache_ttl_seconds: 300,      // 5分钟
            max_cache_age_seconds: 3600, // 1小时
            default_paths: DefaultShellPaths::default(),
        }
    }
}

impl Default for DefaultShellPaths {
    fn default() -> Self {
        Self {
            unix: vec![
                "/bin/zsh".to_string(),
                "/bin/bash".to_string(),
                "/usr/bin/fish".to_string(),
                "/opt/homebrew/bin/fish".to_string(),
                "/bin/sh".to_string(),
                "/usr/local/bin/zsh".to_string(),
                "/usr/local/bin/bash".to_string(),
            ],
            windows: vec![
                "C:\\Program Files\\Git\\bin\\bash.exe".to_string(),
                "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe".to_string(),
                "C:\\Windows\\System32\\cmd.exe".to_string(),
            ],
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            worker_threads: None, // 使用系统默认
            max_concurrent_connections: 100,
            timeouts: TimeoutConfig::default(),
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            command_execution_ms: 30_000, // 30秒
            connection_ms: 5_000,         // 5秒
            read_ms: 10_000,              // 10秒
            write_ms: 5_000,              // 5秒
        }
    }
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            interval_seconds: 300,         // 5分钟
            stale_threshold_seconds: 1800, // 30分钟
            auto_cleanup_enabled: true,
        }
    }
}

impl TerminalSystemConfig {
    /// 从文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.as_ref();
        debug!("从文件加载配置: {:?}", path);

        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;

        info!("配置加载成功: {:?}", path);
        Ok(config)
    }

    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.as_ref();
        debug!("保存配置到文件: {:?}", path);

        let content = toml::to_string_pretty(self)?;

        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, content)?;
        info!("配置保存成功: {:?}", path);
        Ok(())
    }

    /// 从环境变量覆盖配置
    pub fn override_from_env(&mut self) {
        debug!("从环境变量覆盖配置");

        // 缓冲区配置
        if let Ok(val) = std::env::var("TERMINAL_BUFFER_MAX_SIZE") {
            if let Ok(size) = val.parse::<usize>() {
                self.buffer.max_size = size;
                debug!("从环境变量设置 buffer.max_size = {}", size);
            }
        }

        if let Ok(val) = std::env::var("TERMINAL_BUFFER_KEEP_SIZE") {
            if let Ok(size) = val.parse::<usize>() {
                self.buffer.keep_size = size;
                debug!("从环境变量设置 buffer.keep_size = {}", size);
            }
        }

        // Shell配置
        if let Ok(val) = std::env::var("TERMINAL_SHELL_CACHE_TTL") {
            if let Ok(ttl) = val.parse::<u64>() {
                self.shell.cache_ttl_seconds = ttl;
                debug!("从环境变量设置 shell.cache_ttl_seconds = {}", ttl);
            }
        }

        // 清理配置
        if let Ok(val) = std::env::var("TERMINAL_CLEANUP_INTERVAL") {
            if let Ok(interval) = val.parse::<u64>() {
                self.cleanup.interval_seconds = interval;
                debug!("从环境变量设置 cleanup.interval_seconds = {}", interval);
            }
        }

        if let Ok(val) = std::env::var("TERMINAL_AUTO_CLEANUP") {
            if let Ok(enabled) = val.parse::<bool>() {
                self.cleanup.auto_cleanup_enabled = enabled;
                debug!("从环境变量设置 cleanup.auto_cleanup_enabled = {}", enabled);
            }
        }
    }

    /// 验证配置的有效性
    pub fn validate(&self) -> Result<(), String> {
        // 验证缓冲区配置
        if self.buffer.max_size == 0 {
            return Err("buffer.max_size 不能为0".to_string());
        }

        if self.buffer.keep_size >= self.buffer.max_size {
            return Err("buffer.keep_size 必须小于 buffer.max_size".to_string());
        }

        if self.buffer.max_truncation_attempts == 0 {
            return Err("buffer.max_truncation_attempts 不能为0".to_string());
        }

        // 验证Shell配置
        if self.shell.cache_ttl_seconds == 0 {
            return Err("shell.cache_ttl_seconds 不能为0".to_string());
        }

        // 验证性能配置
        if self.performance.max_concurrent_connections == 0 {
            return Err("performance.max_concurrent_connections 不能为0".to_string());
        }

        // 验证清理配置
        if self.cleanup.interval_seconds == 0 {
            return Err("cleanup.interval_seconds 不能为0".to_string());
        }

        if self.cleanup.stale_threshold_seconds == 0 {
            return Err("cleanup.stale_threshold_seconds 不能为0".to_string());
        }

        debug!("配置验证通过");
        Ok(())
    }

    /// 获取缓冲区清理间隔
    pub fn cleanup_interval(&self) -> Duration {
        Duration::from_secs(self.cleanup.interval_seconds)
    }

    /// 获取过期阈值
    pub fn stale_threshold(&self) -> Duration {
        Duration::from_secs(self.cleanup.stale_threshold_seconds)
    }

    /// 获取Shell缓存TTL
    pub fn shell_cache_ttl(&self) -> Duration {
        Duration::from_secs(self.shell.cache_ttl_seconds)
    }

    /// 获取Shell最大缓存年龄
    pub fn shell_max_cache_age(&self) -> Duration {
        Duration::from_secs(self.shell.max_cache_age_seconds)
    }
}

/// 全局配置管理器
static GLOBAL_CONFIG: OnceLock<Arc<Mutex<TerminalSystemConfig>>> = OnceLock::new();

/// 配置管理器
pub struct ConfigManager;

impl ConfigManager {
    /// 初始化全局配置
    pub fn init() -> Result<(), Box<dyn std::error::Error>> {
        let config = Self::load_config()?;
        GLOBAL_CONFIG
            .set(Arc::new(Mutex::new(config)))
            .map_err(|_| "配置管理器已经初始化")?;
        info!("配置管理器初始化成功");
        Ok(())
    }

    /// 获取全局配置
    pub fn get() -> Arc<Mutex<TerminalSystemConfig>> {
        GLOBAL_CONFIG
            .get_or_init(|| {
                warn!("配置管理器未初始化，使用默认配置");
                Arc::new(Mutex::new(TerminalSystemConfig::default()))
            })
            .clone()
    }

    /// 加载配置（按优先级：文件 -> 环境变量 -> 默认值）
    fn load_config() -> Result<TerminalSystemConfig, Box<dyn std::error::Error>> {
        let mut config = TerminalSystemConfig::default();

        // 尝试从配置文件加载
        let config_paths = [
            "terminal-config.toml",
            "config/terminal.toml",
            "~/.config/terminal/config.toml",
            "/etc/terminal/config.toml",
        ];

        for path in &config_paths {
            if Path::new(path).exists() {
                match TerminalSystemConfig::from_file(path) {
                    Ok(file_config) => {
                        config = file_config;
                        info!("从文件加载配置: {}", path);
                        break;
                    }
                    Err(e) => {
                        warn!("加载配置文件失败 {}: {}", path, e);
                    }
                }
            }
        }

        // 从环境变量覆盖
        config.override_from_env();

        // 验证配置
        config.validate()?;

        Ok(config)
    }

    /// 重新加载配置
    pub fn reload() -> Result<(), Box<dyn std::error::Error>> {
        let new_config = Self::load_config()?;
        let config_guard = Self::get();
        let mut config = config_guard.lock().map_err(|_| "获取配置锁失败")?;
        *config = new_config;
        info!("配置重新加载成功");
        Ok(())
    }

    /// 保存当前配置到文件
    pub fn save_to_file<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
        let config_guard = Self::get();
        let config = config_guard.lock().map_err(|_| "获取配置锁失败")?;
        config.save_to_file(path)?;
        Ok(())
    }

    /// 获取配置的只读副本
    pub fn get_config() -> TerminalSystemConfig {
        let config_guard = Self::get();
        let config = config_guard
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        config.clone()
    }

    /// 更新配置
    pub fn update_config<F>(updater: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce(&mut TerminalSystemConfig),
    {
        let config_guard = Self::get();
        let mut config = config_guard.lock().map_err(|_| "获取配置锁失败")?;

        updater(&mut config);
        config.validate()?;

        info!("配置更新成功");
        Ok(())
    }
}
