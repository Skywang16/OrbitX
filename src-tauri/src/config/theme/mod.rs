/*!
 * 主题系统模块
 *
 * 统一管理主题相关的所有功能，包括主题管理、命令接口、服务和类型定义。
 */

pub mod commands;
pub mod defaults;
pub mod manager;
pub mod service;
pub mod types;

// 重新导出核心类型和函数
pub use commands::{
    theme_get_available, theme_get_current, theme_get_config_status, handle_system_theme_change,
    theme_set_follow_system, theme_set_terminal, ThemeConfigStatus, ThemeInfo,
};
pub use defaults::create_default_theme_config;
pub use manager::{
    ThemeIndex, ThemeIndexEntry, ThemeManager, ThemeManagerOptions, ThemeValidationResult,
    ThemeValidator,
};
pub use service::{SystemThemeDetector, ThemeService};
pub use types::{AnsiColors, SyntaxHighlight, Theme, ThemeConfig, ThemeType, UIColors};
