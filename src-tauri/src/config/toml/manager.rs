/*!
 * TOML配置管理器
 *
 * 整合读取、写入、验证和事件功能的核心管理器
 */

use super::{
    events::{ConfigEvent, ConfigEventSender},
    reader::TomlConfigReader,
    validator::TomlConfigValidator,
    writer::TomlConfigWriter,
};
use crate::{
    config::{theme::ThemeConfig, types::AppConfig},
    utils::error::AppResult,
};
use anyhow::{anyhow, bail, Context};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tokio::sync::broadcast;
use tracing::debug;

/// TOML配置管理器
///
/// 负责TOML配置文件的完整生命周期管理，包括读写、验证和事件通知。
pub struct TomlConfigManager {
    config_cache: Arc<RwLock<AppConfig>>,
    reader: TomlConfigReader,
    writer: TomlConfigWriter,
    validator: TomlConfigValidator,
    event_sender: ConfigEventSender,
}

impl TomlConfigManager {
    /// 创建新的配置管理器
    pub async fn new() -> AppResult<Self> {
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

        tracing::info!("TOML配置管理器初始化完成");
        Ok(manager)
    }

    /// 从文件系统加载TOML配置
    pub async fn load_config(&self) -> AppResult<AppConfig> {
        let config = match self.reader.load_config().await {
            Ok(config) => config,
            Err(_) => {
                // 加载失败，发送验证失败事件
                self.event_sender
                    .send_validation_failed(vec!["配置文件读取或解析失败".to_string()]);

                // 创建备份并使用默认配置
                self.writer.create_backup_and_use_default().await?
            }
        };

        // 更新缓存
        {
            let mut cache = self
                .config_cache
                .write()
                .map_err(|e| anyhow!("无法获取配置缓存写锁: {}", e))?;
            *cache = config.clone();
        }

        // 发送加载事件
        self.event_sender.send_loaded();

        Ok(config)
    }

    /// 保存配置到文件
    pub async fn save_config(&self, config: &AppConfig) -> AppResult<()> {
        // 验证配置
        if let Err(e) = self.validator.validate_config(config) {
            let errors = vec![e.to_string()];
            self.event_sender.send_validation_failed(errors);
            return Err(e);
        }

        // 保存到文件
        self.writer.save_config(config).await?;

        // 更新缓存
        {
            let mut cache = self
                .config_cache
                .write()
                .map_err(|e| anyhow!("无法获取配置缓存写锁: {}", e))?;
            *cache = config.clone();
        }

        // 发送保存事件
        self.event_sender.send_saved();

        Ok(())
    }

    /// 更新指定配置节
    pub async fn update_section<T>(&self, section: &str, data: T) -> AppResult<()>
    where
        T: Serialize,
    {
        debug!("更新配置节: {}", section);

        // 获取当前配置
        let mut current_config = {
            let cache = self
                .config_cache
                .read()
                .map_err(|e| anyhow!("无法获取配置缓存读锁: {}", e))?;
            cache.clone()
        };

        // 将数据序列化为JSON值以便操作
        let data_value = serde_json::to_value(data).context("无法序列化更新数据")?;

        // 更新指定节
        self.update_config_section(&mut current_config, section, data_value)?;

        // 验证更新后的配置
        if let Err(e) = self.validator.validate_config(&current_config) {
            let errors = vec![e.to_string()];
            self.event_sender.send_validation_failed(errors);
            return Err(e);
        }

        // 保存配置
        self.writer.save_config(&current_config).await?;

        // 更新缓存
        {
            let mut cache = self
                .config_cache
                .write()
                .map_err(|e| anyhow!("无法获取配置缓存写锁: {}", e))?;
            *cache = current_config;
        }

        // 发送更新事件
        self.event_sender.send_updated(Some(section.to_string()));

        Ok(())
    }

    /// 获取当前配置
    pub async fn get_config(&self) -> AppResult<AppConfig> {
        let cache = self
            .config_cache
            .read()
            .map_err(|e| anyhow!("无法获取配置缓存读锁: {}", e))?;
        Ok(cache.clone())
    }

    /// 获取配置文件路径
    pub async fn get_config_path(&self) -> PathBuf {
        self.reader.get_config_path().clone()
    }

    /// 验证配置
    pub fn validate_config(&self, config: &AppConfig) -> AppResult<()> {
        match self.validator.validate_config(config) {
            Ok(()) => Ok(()),
            Err(e) => {
                // 发送验证失败事件
                let errors = vec![e.to_string()];
                self.event_sender.send_validation_failed(errors);
                Err(e)
            }
        }
    }

    /// 订阅配置变更事件
    pub fn subscribe_changes(&self) -> broadcast::Receiver<ConfigEvent> {
        self.event_sender.subscribe()
    }

    /// 使用更新函数更新配置
    pub async fn update_config<F>(&self, updater: F) -> AppResult<()>
    where
        F: FnOnce(&mut AppConfig) -> AppResult<()> + Send,
    {
        // 获取当前配置
        let mut current_config = {
            let cache = self
                .config_cache
                .read()
                .map_err(|e| anyhow!("无法获取配置缓存读锁: {}", e))?;
            cache.clone()
        };

        // 应用更新函数
        updater(&mut current_config)?;

        // 验证更新后的配置
        if let Err(e) = self.validator.validate_config(&current_config) {
            let errors = vec![e.to_string()];
            self.event_sender.send_validation_failed(errors);
            return Err(e);
        }

        // 保存配置
        self.save_config(&current_config).await?;

        Ok(())
    }

    /// 合并配置
    pub fn merge_config(
        &self,
        base_config: &AppConfig,
        partial_config: serde_json::Value,
    ) -> AppResult<AppConfig> {
        debug!("开始合并配置");

        // 将基础配置转换为JSON值
        let mut base_value = serde_json::to_value(base_config).context("无法序列化基础配置")?;

        // 递归合并
        self.merge_json_values(&mut base_value, partial_config)?;

        // 转换回配置结构
        let merged_config: AppConfig =
            serde_json::from_value(base_value).context("无法反序列化合并后的配置")?;

        Ok(merged_config)
    }

    /// 更新配置节的具体实现
    pub fn update_config_section(
        &self,
        config: &mut AppConfig,
        section: &str,
        data: serde_json::Value,
    ) -> AppResult<()> {
        match section {
            "app" => {
                let app_config: crate::config::types::AppConfigApp =
                    serde_json::from_value(data).context("无法反序列化应用配置")?;
                config.app = app_config;
            }
            "app.language" => {
                if let Some(language) = data.as_str() {
                    config.app.language = language.to_string();
                } else {
                    bail!("语言设置必须是字符串类型");
                }
            }
            "app.confirm_on_exit" => {
                if let Some(confirm) = data.as_bool() {
                    config.app.confirm_on_exit = confirm;
                } else {
                    bail!("退出确认设置必须是布尔类型");
                }
            }
            "appearance" => {
                let appearance_config: crate::config::types::AppearanceConfig =
                    serde_json::from_value(data).context("无法反序列化外观配置")?;
                config.appearance = appearance_config;
            }
            "appearance.theme_config" => {
                let theme_config: ThemeConfig =
                    serde_json::from_value(data).context("无法反序列化主题配置")?;
                config.appearance.theme_config = theme_config;
            }
            "appearance.theme_config.terminal_theme" => {
                if let Some(theme) = data.as_str() {
                    config.appearance.theme_config.terminal_theme = theme.to_string();
                } else {
                    bail!("终端主题设置必须是字符串类型");
                }
            }
            "appearance.font" => {
                let font_config: crate::config::types::FontConfig =
                    serde_json::from_value(data).context("无法反序列化字体配置")?;
                config.appearance.font = font_config;
            }
            "appearance.font.size" => {
                if let Some(size) = data.as_f64() {
                    config.appearance.font.size = size as f32;
                } else {
                    bail!("字体大小必须是数字类型");
                }
            }
            "terminal" => {
                let terminal_config: crate::config::types::TerminalConfig =
                    serde_json::from_value(data).context("无法反序列化终端配置")?;
                config.terminal = terminal_config;
            }
            "terminal.scrollback" => {
                if let Some(scrollback) = data.as_u64() {
                    config.terminal.scrollback = scrollback as u32;
                } else {
                    bail!("滚动缓冲区设置必须是正整数");
                }
            }
            "ai" => {
                // AI配置已迁移到SQLite，TOML中不再存储AI配置
                bail!("AI配置已迁移到SQLite，请使用AI API进行配置管理");
            }
            "shortcuts" => {
                let shortcuts_config: crate::config::types::ShortcutsConfig =
                    serde_json::from_value(data).context("无法反序列化快捷键配置")?;
                config.shortcuts = shortcuts_config;
            }
            _ => {
                bail!("不支持的配置节: {}", section);
            }
        }

        Ok(())
    }

    /// 递归合并JSON值
    #[allow(clippy::only_used_in_recursion)]
    fn merge_json_values(
        &self,
        base: &mut serde_json::Value,
        overlay: serde_json::Value,
    ) -> AppResult<()> {
        match (base, overlay) {
            (serde_json::Value::Object(base_obj), serde_json::Value::Object(overlay_obj)) => {
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
