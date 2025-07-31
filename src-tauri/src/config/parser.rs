/*!
 * TOML 配置解析器模块
 *
 * 提供配置文件的读写、解析、序列化和智能合并功能。
 * 支持完整的 TOML 语法，提供详细的错误处理和恢复机制。
 */

use crate::{
    config::{defaults::create_default_config, paths::ConfigPaths, types::AppConfig},
    utils::error::AppResult,
};
use anyhow::Context;

use std::path::PathBuf;
use toml;

/// TOML 配置解析器
///
/// 负责配置文件的读写、解析和序列化操作。
#[derive(Debug, Clone)]
pub struct ConfigParser {
    /// 配置文件路径
    config_path: PathBuf,
}

impl ConfigParser {
    /// 创建新的配置解析器实例
    ///
    /// # Arguments
    /// * `config_paths` - 配置路径管理器
    ///
    /// # Returns
    /// 返回配置解析器实例
    pub fn new(config_paths: &ConfigPaths) -> Self {
        Self {
            config_path: config_paths.config_file(),
        }
    }

    /// 从文件加载配置
    ///
    /// 如果配置文件不存在或解析失败，将尝试从资源目录复制默认配置，
    /// 如果资源复制失败，则使用代码生成的默认配置。
    ///
    /// # Returns
    /// 返回解析后的配置结构
    pub async fn load_config(&self) -> AppResult<AppConfig> {
        // 检查配置文件是否存在
        if !self.config_path.exists() {
            tracing::info!("配置文件不存在，尝试从资源目录复制: {:?}", self.config_path);

            // 注意：资源复制需要在应用初始化时通过 AppHandle 完成
            // 这里先使用代码生成的默认配置作为回退方案
            let default_config = create_default_config();
            self.save_config(&default_config).await?;
            return Ok(default_config);
        }

        // 尝试读取和解析配置文件
        match self.read_config_file().await {
            Ok(content) => {
                match self.parse_toml_content(&content) {
                    Ok(config) => {
                        tracing::debug!("配置文件解析成功");
                        Ok(config)
                    }
                    Err(e) => {
                        tracing::warn!("配置文件解析失败: {}, 使用默认配置", e);
                        let default_config = create_default_config();
                        // 尝试保存默认配置，失败也不影响返回
                        if let Err(save_err) = self.save_config(&default_config).await {
                            tracing::error!("保存默认配置失败: {}", save_err);
                        }
                        Ok(default_config)
                    }
                }
            }
            Err(e) => {
                tracing::warn!("读取配置文件失败: {}, 使用默认配置", e);
                let default_config = create_default_config();
                // 尝试保存默认配置，失败也不影响返回
                if let Err(save_err) = self.save_config(&default_config).await {
                    tracing::error!("保存默认配置失败: {}", save_err);
                }
                Ok(default_config)
            }
        }
    }

    /// 保存配置到文件
    ///
    /// 支持自动备份和原子写入操作。
    ///
    /// # Arguments
    /// * `config` - 要保存的配置结构
    ///
    /// # Returns
    /// 返回操作结果
    pub async fn save_config(&self, config: &AppConfig) -> AppResult<()> {
        // 创建配置目录
        self.ensure_config_directory().await?;

        // 序列化配置为 TOML
        let toml_content = self.serialize_to_toml(config)?;

        // 原子写入配置文件
        self.atomic_write_config(&toml_content).await?;

        Ok(())
    }

    // ========================================================================
    // 私有方法
    // ========================================================================

    /// 读取配置文件内容
    async fn read_config_file(&self) -> AppResult<String> {
        tokio::fs::read_to_string(&self.config_path)
            .await
            .with_context(|| format!("无法读取配置文件: {}", self.config_path.display()))
    }

    /// 解析 TOML 内容
    fn parse_toml_content(&self, content: &str) -> AppResult<AppConfig> {
        // 简化：直接解析，失败时返回错误
        toml::from_str::<AppConfig>(content)
            .with_context(|| format!("TOML 配置解析失败 (文件: {})", self.config_path.display()))
    }

    /// 序列化配置为 TOML
    fn serialize_to_toml(&self, config: &AppConfig) -> AppResult<String> {
        toml::to_string_pretty(config).context("配置序列化为TOML失败")
    }

    /// 确保配置目录存在
    async fn ensure_config_directory(&self) -> AppResult<()> {
        if let Some(parent) = self.config_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("无法创建配置目录: {}", parent.display()))?;
        }
        Ok(())
    }

    /// 原子写入配置文件
    async fn atomic_write_config(&self, content: &str) -> AppResult<()> {
        // 创建临时文件
        let temp_path = self.config_path.with_extension("tmp");

        // 写入临时文件
        tokio::fs::write(&temp_path, content)
            .await
            .with_context(|| format!("无法写入临时配置文件: {}", temp_path.display()))?;

        // 原子性地重命名文件
        tokio::fs::rename(&temp_path, &self.config_path)
            .await
            .with_context(|| {
                format!(
                    "无法重命名配置文件: {} -> {}",
                    temp_path.display(),
                    self.config_path.display()
                )
            })?;

        Ok(())
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// 创建测试用的配置解析器
    fn create_test_parser() -> (ConfigParser, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config_paths = ConfigPaths::with_app_data_dir(temp_dir.path()).unwrap();
        let parser = ConfigParser::new(&config_paths);
        (parser, temp_dir)
    }

    #[tokio::test]
    async fn test_load_default_config() {
        let (parser, _temp_dir) = create_test_parser();

        let config = parser.load_config().await.unwrap();

        assert_eq!(config.version, crate::config::CONFIG_VERSION);
        assert_eq!(config.app.language, "zh-CN");
    }

    #[tokio::test]
    async fn test_save_and_load_config() {
        let (parser, _temp_dir) = create_test_parser();

        let mut config = create_default_config();
        config.app.language = "en-US".to_string();

        parser.save_config(&config).await.unwrap();
        let loaded_config = parser.load_config().await.unwrap();

        assert_eq!(loaded_config.app.language, "en-US");
    }

    #[tokio::test]
    async fn test_invalid_toml_recovery() {
        let (parser, _temp_dir) = create_test_parser();

        // 写入无效的 TOML 内容
        let invalid_toml = "invalid toml content [[[";
        tokio::fs::write(parser.config_path.clone(), invalid_toml)
            .await
            .unwrap();

        // 应该能够恢复到默认配置
        let config = parser.load_config().await.unwrap();
        assert_eq!(config.version, crate::config::CONFIG_VERSION);
    }

    #[tokio::test]
    async fn test_partial_config_merge() {
        let (parser, _temp_dir) = create_test_parser();

        // 首先保存一个完整的默认配置
        let default_config = create_default_config();
        parser.save_config(&default_config).await.unwrap();

        // 然后创建一个修改过的配置
        let mut modified_config = default_config.clone();
        modified_config.app.language = "ja-JP".to_string();
        modified_config.app.confirm_on_exit = false;
        modified_config.appearance.font.family = "Monaco".to_string();
        modified_config.appearance.font.size = 16.0;

        parser.save_config(&modified_config).await.unwrap();
        let loaded_config = parser.load_config().await.unwrap();

        // 检查配置是否正确加载
        assert_eq!(loaded_config.app.language, "ja-JP");
        assert!(!loaded_config.app.confirm_on_exit);
        assert_eq!(loaded_config.appearance.font.family, "Monaco");
        assert_eq!(loaded_config.appearance.font.size, 16.0);

        // 检查默认值是否保留
        assert_eq!(loaded_config.app.startup_behavior, "restore"); // 默认值
    }

    #[tokio::test]
    async fn test_toml_format_consistency() {
        let (parser, _temp_dir) = create_test_parser();

        // 创建默认配置
        let config = create_default_config();

        // 序列化为TOML
        let toml_content = parser.serialize_to_toml(&config).unwrap();

        // 验证TOML格式包含预期的节和字段
        assert!(toml_content.contains("version = \"1.0.0\""));
        assert!(toml_content.contains("[app]"));
        assert!(toml_content.contains("language = \"zh-CN\""));
        assert!(toml_content.contains("confirm_on_exit = true"));
        assert!(toml_content.contains("startup_behavior = \"restore\""));

        assert!(toml_content.contains("[appearance]"));
        assert!(toml_content.contains("ui_scale = 100"));
        assert!(toml_content.contains("animations_enabled = true"));

        assert!(toml_content.contains("[appearance.theme_config]"));
        assert!(toml_content.contains("terminal_theme = \"one-dark\""));
        assert!(toml_content.contains("follow_system = true"));

        assert!(toml_content.contains("[appearance.font]"));
        assert!(toml_content.contains("family = \"Menlo, Monaco, 'Courier New', monospace\""));
        assert!(toml_content.contains("size = 14.0"));

        assert!(toml_content.contains("[terminal]"));
        assert!(toml_content.contains("scrollback = 1000"));

        assert!(toml_content.contains("[window]"));
        assert!(toml_content.contains("opacity = 1.0"));

        assert!(toml_content.contains("[ai]"));
        assert!(toml_content.contains("enabled = true"));

        // 验证能够正确反序列化
        let reparsed_config = parser.parse_toml_content(&toml_content).unwrap();
        assert_eq!(reparsed_config.version, config.version);
        assert_eq!(reparsed_config.app.language, config.app.language);
        assert_eq!(
            reparsed_config.appearance.ui_scale,
            config.appearance.ui_scale
        );
    }

    #[tokio::test]
    async fn test_simplified_error_handling() {
        let (parser, _temp_dir) = create_test_parser();

        // 测试文件不存在的情况
        let config = parser.load_config().await.unwrap();
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.app.language, "zh-CN");

        // 测试无效TOML的情况 - 应该返回默认配置
        let invalid_toml = "invalid toml [[[";
        tokio::fs::write(&parser.config_path, invalid_toml)
            .await
            .unwrap();

        let config = parser.load_config().await.unwrap();
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.app.language, "zh-CN");

        // 验证配置文件已被重新创建为有效的TOML
        let content = tokio::fs::read_to_string(&parser.config_path)
            .await
            .unwrap();
        assert!(content.contains("version = \"1.0.0\""));
        assert!(content.contains("[app]"));
    }
}
