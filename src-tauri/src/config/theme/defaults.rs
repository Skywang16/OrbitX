/*!
 * 主题系统默认配置
 *
 * 提供主题相关配置项的默认值。
 */

use super::types::ThemeConfig;

/// 创建默认主题配置
pub fn create_default_theme_config() -> ThemeConfig {
    ThemeConfig {
        auto_switch_time: "18:00".to_string(),
        terminal_theme: "dark".to_string(),
        light_theme: "light".to_string(),
        dark_theme: "dark".to_string(),
        follow_system: true,
    }
}
