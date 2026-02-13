//! TOML配置管理器

use super::{
    events::{ConfigEvent, ConfigEventSender},
    reader::TomlConfigReader,
    validator::TomlConfigValidator,
    writer::TomlConfigWriter,
};
use crate::config::error::{ConfigError, ConfigResult, TomlConfigError};
use crate::config::{theme::ThemeConfig, types::AppConfig};
use serde::Serialize;
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tokio::sync::broadcast;

/// TOML配置管理器
pub struct TomlConfigManager {
    config_cache: Arc<RwLock<AppConfig>>,
    reader: TomlConfigReader,
    writer: TomlConfigWriter,
    validator: TomlConfigValidator,
    event_sender: ConfigEventSender,
}

impl TomlConfigManager {
    /// 创建新的配置管理器
    pub async fn new() -> ConfigResult<Self> {
        let reader = TomlConfigReader::new()?;
        let config_path = reader.get_config_path().clone();
        let writer = TomlConfigWriter::new(config_path);
        let validator = TomlConfigValidator::new();
        let event_sender = ConfigEventSender::new().0;

        let manager = Self {
            config_cache: Arc::new(RwLock::new(crate::config::defaults::create_default_config())),
            reader,
            writer,
            validator,
            event_sender,
        };

        Ok(manager)
    }

    /// 创建用于测试的配置管理器
    #[cfg(test)]
    pub async fn new_for_test(config_path: std::path::PathBuf) -> ConfigResult<Self> {
        let reader = TomlConfigReader::new_with_config_path(config_path.clone())?;
        let writer = TomlConfigWriter::new(config_path);
        let validator = TomlConfigValidator::new();
        let event_sender = ConfigEventSender::new().0;

        let manager = Self {
            config_cache: Arc::new(RwLock::new(crate::config::defaults::create_default_config())),
            reader,
            writer,
            validator,
            event_sender,
        };

        Ok(manager)
    }

    /// 从文件系统加载TOML配置
    pub async fn load_config(&self) -> ConfigResult<AppConfig> {
        let config = match self.reader.load_config().await {
            Ok(config) => config,
            Err(_) => {
                // 加载失败，发送验证失败事件
                self.event_sender
                    .send_validation_failed(vec!["配置文件读取或解析失败".to_string()]);

                self.writer.create_backup_and_use_default().await?
            }
        };

        // 更新缓存
        {
            let mut cache = self
                .config_cache
                .write()
                .map_err(TomlConfigError::from_poison)
                .map_err(ConfigError::from)?;
            *cache = config.clone();
        }

        // 发送加载事件
        self.event_sender.send_loaded();

        Ok(config)
    }

    /// 保存配置到文件
    pub async fn config_save(&self, config: &AppConfig) -> ConfigResult<()> {
        // 验证配置
        if let Err(e) = self.validator.config_validate(config) {
            let errors = vec![e.to_string()];
            self.event_sender.send_validation_failed(errors);
            return Err(ConfigError::from(e));
        }

        // 保存到文件
        self.writer.config_save(config).await?;

        // 更新缓存
        {
            let mut cache = self
                .config_cache
                .write()
                .map_err(TomlConfigError::from_poison)
                .map_err(ConfigError::from)?;
            *cache = config.clone();
        }

        // 发送保存事件
        self.event_sender.send_saved();

        Ok(())
    }

    /// 更新指定配置节
    pub async fn update_section<T>(&self, section: &str, data: T) -> ConfigResult<()>
    where
        T: Serialize,
    {
        let mut current_config = {
            let cache = self
                .config_cache
                .read()
                .map_err(TomlConfigError::from_poison)
                .map_err(ConfigError::from)?;
            cache.clone()
        };

        // 将数据序列化为JSON值以便操作
        let data_value: Value = serde_json::to_value(data)?;

        // 更新指定节
        self.update_config_section(&mut current_config, section, data_value)?;

        // 验证更新后的配置
        if let Err(e) = self.validator.config_validate(&current_config) {
            let errors = vec![e.to_string()];
            self.event_sender.send_validation_failed(errors);
            return Err(ConfigError::from(e));
        }

        // 保存配置
        self.writer.config_save(&current_config).await?;

        // 更新缓存
        {
            let mut cache = self
                .config_cache
                .write()
                .map_err(TomlConfigError::from_poison)
                .map_err(ConfigError::from)?;
            *cache = current_config;
        }

        // 发送更新事件
        self.event_sender.send_updated(Some(section.to_string()));

        Ok(())
    }

    /// 获取当前配置
    pub async fn config_get(&self) -> ConfigResult<AppConfig> {
        let cache = self
            .config_cache
            .read()
            .map_err(TomlConfigError::from_poison)
            .map_err(ConfigError::from)?;
        Ok(cache.clone())
    }

    /// 获取配置文件路径
    pub async fn get_config_path(&self) -> PathBuf {
        self.reader.get_config_path().clone()
    }

    /// 验证配置
    pub fn config_validate(&self, config: &AppConfig) -> ConfigResult<()> {
        if let Err(e) = self.validator.config_validate(config) {
            let errors = vec![e.to_string()];
            self.event_sender.send_validation_failed(errors);
            return Err(ConfigError::from(e));
        }
        Ok(())
    }

    /// 订阅配置变更事件
    pub fn subscribe_changes(&self) -> broadcast::Receiver<ConfigEvent> {
        self.event_sender.subscribe()
    }

    /// 使用更新函数更新配置
    pub async fn config_update<F>(&self, updater: F) -> ConfigResult<()>
    where
        F: FnOnce(&mut AppConfig) -> ConfigResult<()> + Send,
    {
        let mut current_config = {
            let cache = self
                .config_cache
                .read()
                .map_err(TomlConfigError::from_poison)
                .map_err(ConfigError::from)?;
            cache.clone()
        };

        // 应用更新函数
        updater(&mut current_config)?;

        // 验证更新后的配置
        if let Err(e) = self.validator.config_validate(&current_config) {
            let errors = vec![e.to_string()];
            self.event_sender.send_validation_failed(errors);
            return Err(ConfigError::from(e));
        }

        // 保存配置
        self.config_save(&current_config).await?;

        Ok(())
    }

    /// 合并配置
    pub fn merge_config(
        &self,
        base_config: &AppConfig,
        partial_config: Value,
    ) -> ConfigResult<AppConfig> {
        // 将基础配置转换为JSON值
        let mut base_value = serde_json::to_value(base_config)?;

        // 递归合并
        self.merge_json_values(&mut base_value, partial_config)?;

        // 转换回配置结构
        let merged_config: AppConfig = serde_json::from_value(base_value)?;

        Ok(merged_config)
    }

    /// 更新配置节的具体实现
    pub fn update_config_section(
        &self,
        config: &mut AppConfig,
        section: &str,
        data: Value,
    ) -> ConfigResult<()> {
        match section {
            "app" => {
                let app_config: crate::config::types::AppConfigApp = serde_json::from_value(data)?;
                config.app = app_config;
            }
            "app.language" => {
                if let Some(language) = data.as_str() {
                    config.app.language = language.to_string();
                } else {
                    return Err(TomlConfigError::Validation {
                        reason: "语言设置必须是字符串类型".to_string(),
                    }
                    .into());
                }
            }
            "app.confirm_on_exit" => {
                if let Some(confirm) = data.as_bool() {
                    config.app.confirm_on_exit = confirm;
                } else {
                    return Err(TomlConfigError::Validation {
                        reason: "退出确认设置必须是布尔类型".to_string(),
                    }
                    .into());
                }
            }
            "appearance" => {
                let appearance_config: crate::config::types::AppearanceConfig =
                    serde_json::from_value(data)?;
                config.appearance = appearance_config;
            }
            "appearance.theme_config" => {
                let theme_config: ThemeConfig = serde_json::from_value(data)?;
                config.appearance.theme_config = theme_config;
            }
            "appearance.theme_config.terminal_theme" => {
                if let Some(theme) = data.as_str() {
                    config.appearance.theme_config.terminal_theme = theme.to_string();
                } else {
                    return Err(TomlConfigError::Validation {
                        reason: "终端主题设置必须是字符串类型".to_string(),
                    }
                    .into());
                }
            }
            "appearance.font" => {
                let font_config: crate::config::types::FontConfig = serde_json::from_value(data)?;
                config.appearance.font = font_config;
            }
            "appearance.font.size" => {
                if let Some(size) = data.as_f64() {
                    config.appearance.font.size = size as f32;
                } else {
                    return Err(TomlConfigError::Validation {
                        reason: "字体大小必须是数字类型".to_string(),
                    }
                    .into());
                }
            }
            "terminal" => {
                let terminal_config: crate::config::types::TerminalConfig =
                    serde_json::from_value(data)?;
                config.terminal = terminal_config;
            }
            "terminal.scrollback" => {
                if let Some(scrollback) = data.as_u64() {
                    config.terminal.scrollback = scrollback as u32;
                } else {
                    return Err(TomlConfigError::Validation {
                        reason: "滚动缓冲区设置必须是正整数".to_string(),
                    }
                    .into());
                }
            }
            "ai" => {
                // AI配置已迁移到SQLite，TOML中不再存储AI配置
                return Err(TomlConfigError::Validation {
                    reason: "AI配置已迁移到SQLite，请使用AI API进行配置管理".to_string(),
                }
                .into());
            }
            "shortcuts" => {
                let shortcuts_config: crate::config::types::ShortcutsConfig =
                    serde_json::from_value(data)?;
                config.shortcuts = shortcuts_config;
            }
            _ => {
                return Err(TomlConfigError::Validation {
                    reason: format!("不支持的配置节: {}", section),
                }
                .into());
            }
        }

        Ok(())
    }

    /// 递归合并JSON值
    #[allow(clippy::only_used_in_recursion)]
    fn merge_json_values(&self, base: &mut Value, overlay: Value) -> ConfigResult<()> {
        match (base, overlay) {
            (Value::Object(base_obj), Value::Object(overlay_obj)) => {
                for (key, value) in overlay_obj {
                    if let Some(base_value) = base_obj.get_mut(&key) {
                        self.merge_json_values(base_value, value)?;
                    } else {
                        base_obj.insert(key, value);
                    }
                }
            }
            (base_val, overlay_val) => {
                *base_val = overlay_val;
            }
        }
        Ok(())
    }
}
