//! Terminal multiplexer configuration management

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tracing::warn;

use crate::mux::error::{MuxConfigError, MuxConfigResult};

/// Terminal runtime configuration persisted on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSystemConfig {
    /// Buffer configuration
    pub buffer: BufferConfig,
    /// Shell configuration
    pub shell: ShellSystemConfig,
    /// Performance tuning configuration
    pub performance: PerformanceConfig,
    /// Cleanup configuration
    pub cleanup: CleanupConfig,
}

/// Buffer configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BufferConfig {
    /// Maximum buffer size in bytes
    pub max_size: usize,
    /// Preferred buffer size to keep in memory in bytes
    pub keep_size: usize,
    /// Maximum truncation attempts before giving up
    pub max_truncation_attempts: usize,
}

/// Shell integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellSystemConfig {
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    /// Maximum cache age in seconds
    pub max_cache_age_seconds: u64,
    /// Default shell search paths grouped by platform
    pub default_paths: DefaultShellPaths,
}

/// Default shell path list per platform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultShellPaths {
    /// Default shell executable list on Unix-like systems
    pub unix: Vec<String>,
    /// Default shell executable list on Windows systems
    pub windows: Vec<String>,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceConfig {
    /// Worker thread count
    pub worker_threads: Option<usize>,
    /// Maximum concurrent connections
    pub max_concurrent_connections: usize,
    /// Timeout configuration in milliseconds
    pub timeouts: TimeoutConfig,
}

/// Timeout settings for terminal I/O
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeoutConfig {
    /// Command execution timeout
    pub command_execution_ms: u64,
    /// Connection timeout
    pub connection_ms: u64,
    /// Read timeout
    pub read_ms: u64,
    /// Write timeout
    pub write_ms: u64,
}

/// Background cleanup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupConfig {
    /// Cleanup interval in seconds
    pub interval_seconds: u64,
    /// Stale threshold in seconds
    pub stale_threshold_seconds: u64,
    /// Whether automatic cleanup is enabled
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
            max_size: 1_000_000,
            keep_size: 500_000,
            max_truncation_attempts: 1000,
        }
    }
}

impl Default for ShellSystemConfig {
    fn default() -> Self {
        Self {
            cache_ttl_seconds: 300,      // 5 minutes
            max_cache_age_seconds: 3600, // 1 hour
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
                "/usr/local/bin/zsh".to_string(),
                "/usr/local/bin/bash".to_string(),
                "/usr/local/bin/fish".to_string(),
            ],
            windows: vec![
                "C:\\Program Files\\Git\\bin\\bash.exe".to_string(),
                "C:\\Program Files\\Git\\usr\\bin\\bash.exe".to_string(),
            ],
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            worker_threads: None, // use system default
            max_concurrent_connections: 100,
            timeouts: TimeoutConfig::default(),
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            command_execution_ms: 30_000, // 30 seconds
            connection_ms: 5_000,         // 5 seconds
            read_ms: 10_000,              // 10 seconds
            write_ms: 5_000,              // 5 seconds
        }
    }
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            interval_seconds: 300,         // 5 minutes
            stale_threshold_seconds: 1800, // 30 minutes
            auto_cleanup_enabled: true,
        }
    }
}

impl TerminalSystemConfig {
    /// Apply environment variable overrides
    pub fn override_from_env(&mut self) {
        if let Ok(val) = std::env::var("TERMINAL_BUFFER_MAX_SIZE") {
            if let Ok(size) = val.parse::<usize>() {
                self.buffer.max_size = size;
            }
        }

        if let Ok(val) = std::env::var("TERMINAL_BUFFER_KEEP_SIZE") {
            if let Ok(size) = val.parse::<usize>() {
                self.buffer.keep_size = size;
            }
        }

        // Shell configuration overrides
        if let Ok(val) = std::env::var("TERMINAL_SHELL_CACHE_TTL") {
            if let Ok(ttl) = val.parse::<u64>() {
                self.shell.cache_ttl_seconds = ttl;
            }
        }

        // Cleanup configuration overrides
        if let Ok(val) = std::env::var("TERMINAL_CLEANUP_INTERVAL") {
            if let Ok(interval) = val.parse::<u64>() {
                self.cleanup.interval_seconds = interval;
            }
        }

        if let Ok(val) = std::env::var("TERMINAL_AUTO_CLEANUP") {
            if let Ok(enabled) = val.parse::<bool>() {
                self.cleanup.auto_cleanup_enabled = enabled;
            }
        }
    }

    /// Validate configuration invariants
    pub fn validate(&self) -> MuxConfigResult<()> {
        // Validate buffer configuration
        if self.buffer.max_size == 0 {
            return Err(MuxConfigError::Validation {
                reason: "buffer.max_size must be greater than zero".to_string(),
            });
        }

        if self.buffer.keep_size >= self.buffer.max_size {
            return Err(MuxConfigError::Validation {
                reason: "buffer.keep_size must be smaller than buffer.max_size".to_string(),
            });
        }

        if self.buffer.max_truncation_attempts == 0 {
            return Err(MuxConfigError::Validation {
                reason: "buffer.max_truncation_attempts must be greater than zero".to_string(),
            });
        }

        // Validate shell configuration
        if self.shell.cache_ttl_seconds == 0 {
            return Err(MuxConfigError::Validation {
                reason: "shell.cache_ttl_seconds must be greater than zero".to_string(),
            });
        }

        // Validate performance configuration
        if self.performance.max_concurrent_connections == 0 {
            return Err(MuxConfigError::Validation {
                reason: "performance.max_concurrent_connections must be greater than zero"
                    .to_string(),
            });
        }

        // Validate cleanup configuration
        if self.cleanup.interval_seconds == 0 {
            return Err(MuxConfigError::Validation {
                reason: "cleanup.interval_seconds must be greater than zero".to_string(),
            });
        }

        if self.cleanup.stale_threshold_seconds == 0 {
            return Err(MuxConfigError::Validation {
                reason: "cleanup.stale_threshold_seconds must be greater than zero".to_string(),
            });
        }

        Ok(())
    }

    /// Cleanup interval helper
    pub fn cleanup_interval(&self) -> Duration {
        Duration::from_secs(self.cleanup.interval_seconds)
    }

    /// Stale threshold helper
    pub fn stale_threshold(&self) -> Duration {
        Duration::from_secs(self.cleanup.stale_threshold_seconds)
    }

    /// Shell cache TTL helper
    pub fn shell_cache_ttl(&self) -> Duration {
        Duration::from_secs(self.shell.cache_ttl_seconds)
    }

    /// Shell cache maximum age helper
    pub fn shell_max_cache_age(&self) -> Duration {
        Duration::from_secs(self.shell.max_cache_age_seconds)
    }
}

/// Global configuration storage
static GLOBAL_CONFIG: OnceLock<Arc<Mutex<TerminalSystemConfig>>> = OnceLock::new();

/// Configuration manager facade
pub struct ConfigManager;

impl ConfigManager {
    /// Initialise global configuration
    pub fn init() -> MuxConfigResult<()> {
        let config = Self::load_config()?;
        GLOBAL_CONFIG
            .set(Arc::new(Mutex::new(config)))
            .map_err(|_| MuxConfigError::Internal("Config manager already initialized".into()))?;
        Ok(())
    }

    /// Acquire the shared configuration handle
    pub fn get() -> Arc<Mutex<TerminalSystemConfig>> {
        GLOBAL_CONFIG
            .get_or_init(|| {
                warn!("Terminal mux config manager was not initialised; falling back to defaults");
                Arc::new(Mutex::new(TerminalSystemConfig::default()))
            })
            .clone()
    }

    /// Load a configuration snapshot (file -> environment -> defaults)
    fn load_config() -> MuxConfigResult<TerminalSystemConfig> {
        let mut config = TerminalSystemConfig::default();

        // Apply environment overrides
        config.override_from_env();

        // Validate the resulting configuration
        config.validate()?;

        Ok(config)
    }

    /// Reload the in-memory configuration
    pub fn reload() -> MuxConfigResult<()> {
        let new_config = Self::load_config()?;
        let config_guard = Self::get();
        let mut config = config_guard
            .lock()
            .map_err(|_| MuxConfigError::Internal("Failed to acquire config lock".into()))?;
        *config = new_config;
        Ok(())
    }

    /// Return a snapshot of the configuration
    pub fn config_get() -> TerminalSystemConfig {
        let config_guard = Self::get();
        let config = config_guard
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        config.clone()
    }

    /// Mutate the configuration under a lock
    pub fn config_update<F>(updater: F) -> MuxConfigResult<()>
    where
        F: FnOnce(&mut TerminalSystemConfig),
    {
        let config_guard = Self::get();
        let mut config = config_guard
            .lock()
            .map_err(|_| MuxConfigError::Internal("Failed to acquire config lock".into()))?;

        updater(&mut config);
        config.validate()?;

        Ok(())
    }
}
