// 配置系统模块

pub mod commands;
pub mod defaults;
pub mod paths;
pub mod shortcuts;
pub mod terminal_commands;
pub mod theme;
pub mod toml;
pub mod types;

pub use commands::{
    config_get, config_get_file_info, config_get_file_path, config_get_folder_path,
    config_open_file, config_open_folder, config_reset_to_defaults, config_save,
    config_subscribe_events, config_update, config_validate, ConfigManagerState,
};
pub use defaults::*;
pub use paths::ConfigPaths;
pub use shortcuts::{
    shortcuts_add, shortcuts_detect_conflicts, shortcuts_execute_action, shortcuts_export_config,
    shortcuts_get_action_metadata, shortcuts_get_config, shortcuts_get_current_platform,
    shortcuts_get_registered_actions, shortcuts_get_statistics, shortcuts_import_config,
    shortcuts_remove, shortcuts_reset_to_defaults, shortcuts_search, shortcuts_update,
    shortcuts_update_config, shortcuts_validate_config, shortcuts_validate_key_combination,
    ShortcutManagerState,
};
pub use terminal_commands::{
    config_terminal_detect_system_shells, config_terminal_get, config_terminal_get_shell_info,
    config_terminal_reset_to_defaults, config_terminal_update, config_terminal_update_behavior,
    config_terminal_update_cursor, config_terminal_validate, config_terminal_validate_shell_path,
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
