/*!
 * 统一配置系统模块
 *
 * 提供基于 TOML 格式的统一配置管理功能，包括配置解析、验证、
 * 缓存和文件监听等核心功能。
 */

pub mod cache;
pub mod commands;
pub mod css_parser;
pub mod defaults;
pub mod manager;
pub mod parser;
pub mod paths;
pub mod shortcuts;
pub mod terminal_commands;
pub mod theme;
pub mod theme_commands;
pub mod theme_service;
pub mod types;
pub mod validator;

// 重新导出核心类型和函数
pub use cache::{
    CacheConfig, CacheEntry, CacheManager, CacheStats, ConfigCache, InvalidationStrategy,
};
pub use commands::*;
pub use defaults::*;
pub use manager::{ConfigEvent, ConfigManager, ConfigManagerOptions};

pub use parser::ConfigParser;
pub use paths::ConfigPaths;
pub use shortcuts::*;
pub use terminal_commands::*;
pub use theme::{
    ThemeIndex, ThemeIndexEntry, ThemeManager, ThemeManagerOptions, ThemeValidationResult,
    ThemeValidator,
};
pub use theme_commands::{
    get_available_themes, get_current_theme, get_theme_config_status, handle_system_theme_change,
    set_follow_system_theme, set_terminal_theme, ThemeConfigStatus, ThemeInfo,
};
pub use theme_service::{SystemThemeDetector, ThemeService};
pub use types::*;
pub use validator::{ConfigValidator, ValidationError, ValidationResult};

/// 配置系统版本
pub const CONFIG_VERSION: &str = "1.0.0";

/// 配置文件名
pub const CONFIG_DIR_NAME: &str = "config";
pub const THEMES_DIR_NAME: &str = "themes";
pub const CONFIG_FILE_NAME: &str = "config.toml";

/// 备份目录名
pub const BACKUPS_DIR_NAME: &str = "backups";

/// 缓存目录名
pub const CACHE_DIR_NAME: &str = "cache";

/// 日志目录名
pub const LOGS_DIR_NAME: &str = "logs";
