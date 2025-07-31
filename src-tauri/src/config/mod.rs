/*!
 * 统一配置系统模块
 *
 * 提供基于 TOML 格式的统一配置管理功能，包括配置解析、验证、
 * 缓存和文件监听等核心功能。
 */

pub mod commands;
pub mod defaults;
pub mod paths;
pub mod shortcuts;
pub mod terminal_commands;
pub mod theme;
pub mod toml_manager;
pub mod types;

// 重新导出核心类型和函数
pub use commands::{
    create_builtin_themes, get_config, get_config_file_info, get_config_file_path, get_theme_index,
    get_theme_list, load_theme, open_config_file, refresh_theme_index, reset_config_to_defaults,
    save_config, subscribe_config_events, switch_theme, update_config, validate_config,
    validate_theme, ConfigManagerState,
};
pub use defaults::*;
pub use paths::ConfigPaths;
pub use shortcuts::commands::{
    adapt_shortcuts_for_platform, add_shortcut, detect_shortcut_conflicts, get_current_platform,
    get_shortcuts_config, get_shortcuts_statistics, remove_shortcut, reset_shortcuts_to_defaults,
    update_shortcut, update_shortcuts_config, validate_shortcut_binding, validate_shortcuts_config,
};
pub use terminal_commands::{
    detect_system_shells, get_shell_info, get_terminal_config, reset_terminal_config_to_defaults,
    update_cursor_config, update_terminal_behavior_config, update_terminal_config,
    validate_terminal_config, validate_terminal_shell_path,
};
pub use theme::{
    get_available_themes, get_current_theme, get_theme_config_status, handle_system_theme_change,
    set_follow_system_theme, set_terminal_theme, SystemThemeDetector, ThemeConfigStatus,
    ThemeIndex, ThemeIndexEntry, ThemeInfo, ThemeManager, ThemeManagerOptions, ThemeService,
    ThemeValidationResult, ThemeValidator,
};
pub use toml_manager::{ConfigEvent, TomlConfigManager};
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
