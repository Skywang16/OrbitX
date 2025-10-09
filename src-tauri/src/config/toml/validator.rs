//! TOML配置验证器

use crate::config::error::{TomlConfigError, TomlConfigResult};
use crate::config::{theme::ThemeConfig, types::AppConfig};
use tracing::{debug, info};

/// TOML配置验证器
pub struct TomlConfigValidator;

impl TomlConfigValidator {
    /// 创建新的配置验证器
    pub fn new() -> Self {
        Self
    }

    /// 验证完整配置
    pub fn config_validate(&self, config: &AppConfig) -> TomlConfigResult<()> {
        debug!("开始验证配置");

        let mut errors = Vec::new();

        // 验证版本
        if config.version.is_empty() {
            errors.push("Config version cannot be empty".to_string());
        }

        // 验证应用配置
        if let Err(e) = self.validate_app_config(&config.app) {
            errors.push(format!("Application config validation failed: {}", e));
        }

        // 验证外观配置
        if let Err(e) = self.validate_appearance_config(&config.appearance) {
            errors.push(format!("Appearance config validation failed: {}", e));
        }

        // 验证终端配置
        if let Err(e) = self.config_terminal_validate(&config.terminal) {
            errors.push(format!("Terminal config validation failed: {}", e));
        }

        if !errors.is_empty() {
            return Err(TomlConfigError::Validation {
                reason: format!("Configuration validation failed: {}", errors.join(", ")),
            });
        }

        info!("配置验证通过");
        Ok(())
    }

    /// 验证应用配置
    fn validate_app_config(
        &self,
        app_config: &crate::config::types::AppConfigApp,
    ) -> TomlConfigResult<()> {
        // 验证语言设置
        let supported_languages = [
            "zh-CN", "en-US", "ja-JP", "ko-KR", "fr-FR", "de-DE", "es-ES",
        ];
        if !supported_languages.contains(&app_config.language.as_str()) {
            return Err(TomlConfigError::Validation {
                reason: format!("Unsupported language: {}", app_config.language),
            });
        }

        // 验证启动行为
        let supported_behaviors = ["restore", "new", "last"];
        if !supported_behaviors.contains(&app_config.startup_behavior.as_str()) {
            return Err(TomlConfigError::Validation {
                reason: format!(
                    "Unsupported startup behavior: {}",
                    app_config.startup_behavior
                ),
            });
        }

        Ok(())
    }

    /// 验证外观配置
    fn validate_appearance_config(
        &self,
        appearance_config: &crate::config::types::AppearanceConfig,
    ) -> TomlConfigResult<()> {
        // 验证UI缩放比例
        if !(50..=200).contains(&appearance_config.ui_scale) {
            return Err(TomlConfigError::Validation {
                reason: format!(
                    "UI scale must be between 50 and 200, current: {}",
                    appearance_config.ui_scale
                ),
            });
        }

        // 验证字体配置
        self.validate_font_config(&appearance_config.font)?;

        // 验证主题配置
        self.validate_theme_config(&appearance_config.theme_config)?;

        Ok(())
    }

    /// 验证字体配置
    fn validate_font_config(
        &self,
        font_config: &crate::config::types::FontConfig,
    ) -> TomlConfigResult<()> {
        // 验证字体大小
        if !(8.0..=72.0).contains(&font_config.size) {
            return Err(TomlConfigError::Validation {
                reason: format!("Font size must be between 8.0 and 72.0, current: {}", font_config.size),
            });
        }

        // 验证行高
        if !(0.5..=3.0).contains(&font_config.line_height) {
            return Err(TomlConfigError::Validation {
                reason: format!("Line height must be between 0.5 and 3.0, current: {}", font_config.line_height),
            });
        }

        // 验证字符间距
        if !(-5.0..=5.0).contains(&font_config.letter_spacing) {
            return Err(TomlConfigError::Validation {
                reason: format!(
                    "Letter spacing must be between -5.0 and 5.0, current: {}",
                    font_config.letter_spacing
                ),
            });
        }

        Ok(())
    }

    /// 验证主题配置
    fn validate_theme_config(&self, theme_config: &ThemeConfig) -> TomlConfigResult<()> {
        // 验证自动切换时间格式
        if !theme_config.auto_switch_time.contains(':') {
            return Err(TomlConfigError::Validation {
                reason: format!(
                    "Invalid auto-switch time format: {}",
                    theme_config.auto_switch_time
                ),
            });
        }

        // 验证主题名称不为空
        if theme_config.terminal_theme.is_empty() {
            return Err(TomlConfigError::Validation {
                reason: "Terminal theme name cannot be empty".to_string(),
            });
        }

        if theme_config.light_theme.is_empty() {
            return Err(TomlConfigError::Validation {
                reason: "Light theme name cannot be empty".to_string(),
            });
        }

        if theme_config.dark_theme.is_empty() {
            return Err(TomlConfigError::Validation {
                reason: "Dark theme name cannot be empty".to_string(),
            });
        }

        Ok(())
    }

    /// 验证终端配置
    fn config_terminal_validate(
        &self,
        terminal_config: &crate::config::types::TerminalConfig,
    ) -> TomlConfigResult<()> {
        // 验证滚动缓冲区
        if !(100..=100000).contains(&terminal_config.scrollback) {
            return Err(TomlConfigError::Validation {
                reason: format!(
                    "Scrollback lines must be between 100 and 100000, current: {}",
                    terminal_config.scrollback
                ),
            });
        }

        // 验证光标配置
        self.validate_cursor_config(&terminal_config.cursor)?;

        Ok(())
    }

    /// 验证光标配置
    fn validate_cursor_config(
        &self,
        cursor_config: &crate::config::types::CursorConfig,
    ) -> TomlConfigResult<()> {
        // 验证光标粗细
        if !(0.1..=5.0).contains(&cursor_config.thickness) {
            return Err(TomlConfigError::Validation {
                reason: format!(
                    "光标粗细必须在0.1-5.0之间，当前值: {}",
                    cursor_config.thickness
                ),
            });
        }

        // 验证颜色格式（简单的十六进制颜色检查）
        if !cursor_config.color.starts_with('#') || cursor_config.color.len() != 7 {
            return Err(TomlConfigError::Validation {
                reason: format!("光标颜色格式无效: {}", cursor_config.color),
            });
        }

        Ok(())
    }
}

impl Default for TomlConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}
