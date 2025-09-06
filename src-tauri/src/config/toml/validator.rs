/*!
 * TOML配置验证器
 * 
 * 负责验证配置的有效性和完整性
 */

use crate::{
    config::{theme::ThemeConfig, types::AppConfig},
    utils::error::AppResult,
};
use anyhow::bail;
use tracing::{debug, info};

/// TOML配置验证器
pub struct TomlConfigValidator;

impl TomlConfigValidator {
    /// 创建新的配置验证器
    pub fn new() -> Self {
        Self
    }

    /// 验证完整配置
    pub fn validate_config(&self, config: &AppConfig) -> AppResult<()> {
        debug!("开始验证配置");

        let mut errors = Vec::new();

        // 验证版本
        if config.version.is_empty() {
            errors.push("配置版本不能为空".to_string());
        }

        // 验证应用配置
        if let Err(e) = self.validate_app_config(&config.app) {
            errors.push(format!("应用配置验证失败: {}", e));
        }

        // 验证外观配置
        if let Err(e) = self.validate_appearance_config(&config.appearance) {
            errors.push(format!("外观配置验证失败: {}", e));
        }

        // 验证终端配置
        if let Err(e) = self.validate_terminal_config(&config.terminal) {
            errors.push(format!("终端配置验证失败: {}", e));
        }

        if !errors.is_empty() {
            bail!("配置验证失败: {}", errors.join(", "));
        }

        info!("配置验证通过");
        Ok(())
    }

    /// 验证应用配置
    fn validate_app_config(
        &self,
        app_config: &crate::config::types::AppConfigApp,
    ) -> AppResult<()> {
        // 验证语言设置
        let supported_languages = [
            "zh-CN", "en-US", "ja-JP", "ko-KR", "fr-FR", "de-DE", "es-ES",
        ];
        if !supported_languages.contains(&app_config.language.as_str()) {
            bail!("不支持的语言: {}", app_config.language);
        }

        // 验证启动行为
        let supported_behaviors = ["restore", "new", "last"];
        if !supported_behaviors.contains(&app_config.startup_behavior.as_str()) {
            bail!("不支持的启动行为: {}", app_config.startup_behavior);
        }

        Ok(())
    }

    /// 验证外观配置
    fn validate_appearance_config(
        &self,
        appearance_config: &crate::config::types::AppearanceConfig,
    ) -> AppResult<()> {
        // 验证UI缩放比例
        if !(50..=200).contains(&appearance_config.ui_scale) {
            bail!(
                "UI缩放比例必须在50-200之间，当前值: {}",
                appearance_config.ui_scale
            );
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
    ) -> AppResult<()> {
        // 验证字体大小
        if !(8.0..=72.0).contains(&font_config.size) {
            bail!("字体大小必须在8.0-72.0之间，当前值: {}", font_config.size);
        }

        // 验证行高
        if !(0.5..=3.0).contains(&font_config.line_height) {
            bail!("行高必须在0.5-3.0之间，当前值: {}", font_config.line_height);
        }

        // 验证字符间距
        if !(-5.0..=5.0).contains(&font_config.letter_spacing) {
            bail!(
                "字符间距必须在-5.0-5.0之间，当前值: {}",
                font_config.letter_spacing
            );
        }

        Ok(())
    }

    /// 验证主题配置
    fn validate_theme_config(&self, theme_config: &ThemeConfig) -> AppResult<()> {
        // 验证自动切换时间格式
        if !theme_config.auto_switch_time.contains(':') {
            bail!("自动切换时间格式无效: {}", theme_config.auto_switch_time);
        }

        // 验证主题名称不为空
        if theme_config.terminal_theme.is_empty() {
            bail!("终端主题名称不能为空");
        }

        if theme_config.light_theme.is_empty() {
            bail!("浅色主题名称不能为空");
        }

        if theme_config.dark_theme.is_empty() {
            bail!("深色主题名称不能为空");
        }

        Ok(())
    }

    /// 验证终端配置
    fn validate_terminal_config(
        &self,
        terminal_config: &crate::config::types::TerminalConfig,
    ) -> AppResult<()> {
        // 验证滚动缓冲区
        if !(100..=100000).contains(&terminal_config.scrollback) {
            bail!(
                "滚动缓冲区行数必须在100-100000之间，当前值: {}",
                terminal_config.scrollback
            );
        }

        // 验证光标配置
        self.validate_cursor_config(&terminal_config.cursor)?;

        Ok(())
    }

    /// 验证光标配置
    fn validate_cursor_config(
        &self,
        cursor_config: &crate::config::types::CursorConfig,
    ) -> AppResult<()> {
        // 验证光标粗细
        if !(0.1..=5.0).contains(&cursor_config.thickness) {
            bail!(
                "光标粗细必须在0.1-5.0之间，当前值: {}",
                cursor_config.thickness
            );
        }

        // 验证颜色格式（简单的十六进制颜色检查）
        if !cursor_config.color.starts_with('#') || cursor_config.color.len() != 7 {
            bail!("光标颜色格式无效: {}", cursor_config.color);
        }

        Ok(())
    }
}

impl Default for TomlConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}
