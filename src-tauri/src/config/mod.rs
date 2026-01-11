// 配置系统模块

pub mod commands;
pub mod defaults;
pub mod error;
pub mod paths;
pub mod shortcuts;
pub mod terminal_commands;
pub mod theme;
pub mod toml;
pub mod types;

pub use commands::{
    config_get, config_open_folder, config_reset_to_defaults, config_set, ConfigManagerState,
};
pub use defaults::*;
pub use error::{
    ConfigCommandError, ConfigCommandResult, ConfigError, ConfigPathsError, ConfigPathsResult,
    ConfigResult, ShortcutsActionError, ShortcutsActionResult, ShortcutsError, ShortcutsResult,
    TerminalConfigError, TerminalConfigResult, ThemeConfigError, ThemeConfigResult,
    TomlConfigError, TomlConfigResult,
};
pub use paths::ConfigPaths;
pub use shortcuts::{
    shortcuts_add, shortcuts_detect_conflicts, shortcuts_execute_action, shortcuts_get_config,
    shortcuts_get_current_platform, shortcuts_get_statistics, shortcuts_remove,
    shortcuts_reset_to_defaults, shortcuts_update, shortcuts_update_config,
    shortcuts_validate_config, ShortcutManagerState,
};
pub use terminal_commands::{
    terminal_config_get, terminal_config_reset_to_defaults, terminal_config_set,
    terminal_config_validate,
};
pub use theme::{
    handle_system_theme_change, theme_get_available, theme_get_config_status, theme_get_current,
    theme_set_follow_system, theme_set_terminal, SystemThemeDetector, ThemeConfigStatus,
    ThemeIndex, ThemeIndexEntry, ThemeInfo, ThemeManager, ThemeManagerOptions, ThemeService,
    ThemeValidationResult, ThemeValidator,
};
pub use toml::{ConfigEvent, TomlConfigManager};
pub use types::*;

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
