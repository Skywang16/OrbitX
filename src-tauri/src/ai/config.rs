/*!
 * AI配置管理模块
 */

use crate::ai::AIModelConfig;
use crate::config::ConfigManager;
use crate::utils::error::AppResult;
use anyhow::anyhow;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// AI配置管理器
///
/// 直接使用统一配置系统管理 AI 配置
pub struct AIConfigManager {
    /// 统一配置管理器的引用
    config_manager: Arc<ConfigManager>,
}

impl AIConfigManager {
    /// 创建新的 AI 配置管理器
    ///
    /// # Arguments
    /// * `config_manager` - 统一配置管理器的引用
    pub fn new(config_manager: Arc<ConfigManager>) -> Self {
        Self { config_manager }
    }

    /// 获取所有模型配置
    pub async fn get_models(&self) -> AppResult<Vec<AIModelConfig>> {
        let config = self.config_manager.get_config().await?;
        // 将简单模型配置转换为完整的AIModelConfig
        let models: Vec<AIModelConfig> = config
            .ai
            .models
            .iter()
            .map(|simple_model| {
                AIModelConfig {
                    id: simple_model.name.clone(),
                    name: simple_model.name.clone(),
                    provider: match simple_model.provider.as_str() {
                        "openai" => crate::ai::AIProvider::OpenAI,
                        "anthropic" => crate::ai::AIProvider::Claude,
                        _ => crate::ai::AIProvider::Custom,
                    },
                    api_url: "".to_string(), // 这些字段在简单配置中不存在
                    api_key: "".to_string(),
                    model: simple_model.name.clone(),
                    is_default: Some(false),
                    options: None,
                }
            })
            .collect();
        Ok(models)
    }

    /// 根据ID获取模型配置
    pub async fn get_model(&self, model_id: &str) -> AppResult<Option<AIModelConfig>> {
        let config = self.config_manager.get_config().await?;

        // 在简单模型配置中查找
        if let Some(simple_model) = config.ai.models.iter().find(|m| m.name == model_id) {
            Ok(Some(AIModelConfig {
                id: simple_model.name.clone(),
                name: simple_model.name.clone(),
                provider: match simple_model.provider.as_str() {
                    "openai" => crate::ai::AIProvider::OpenAI,
                    "anthropic" => crate::ai::AIProvider::Claude,
                    _ => crate::ai::AIProvider::Custom,
                },
                api_url: "".to_string(),
                api_key: "".to_string(),
                model: simple_model.name.clone(),
                is_default: Some(false),
                options: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// 获取默认模型
    pub async fn get_default_model(&self) -> AppResult<Option<AIModelConfig>> {
        let config = self.config_manager.get_config().await?;

        // 获取第一个启用的模型作为默认模型
        if let Some(simple_model) = config.ai.models.iter().find(|m| m.enabled) {
            Ok(Some(AIModelConfig {
                id: simple_model.name.clone(),
                name: simple_model.name.clone(),
                provider: match simple_model.provider.as_str() {
                    "openai" => crate::ai::AIProvider::OpenAI,
                    "anthropic" => crate::ai::AIProvider::Claude,
                    _ => crate::ai::AIProvider::Custom,
                },
                api_url: "".to_string(),
                api_key: "".to_string(),
                model: simple_model.name.clone(),
                is_default: Some(true),
                options: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// 设置默认模型
    pub async fn set_default_model(&self, model_id: &str) -> AppResult<()> {
        self.config_manager
            .update_config(|config| {
                // 检查模型是否存在
                if !config.ai.models.iter().any(|m| m.name == model_id) {
                    return Err(anyhow!(
                        "AI输入验证错误: Model with ID '{model_id}' not found"
                    ));
                }

                // 在简单配置中，我们通过启用状态来表示默认模型
                for model in &mut config.ai.models {
                    model.enabled = model.name == model_id;
                }
                Ok(())
            })
            .await?;

        Ok(())
    }

    /// 保存设置到存储
    pub async fn save_settings(&self) -> AppResult<()> {
        self.config_manager.save_config().await
    }

    /// 重新加载设置
    pub async fn reload_settings(&self) -> AppResult<()> {
        self.config_manager.load_config().await?;
        Ok(())
    }

    /// 添加模型配置
    pub async fn add_model(&self, config: AIModelConfig) -> AppResult<()> {
        // 验证配置
        self.validate_model_config(&config)?;

        self.config_manager
            .update_config(|app_config| {
                // 检查ID是否已存在
                if app_config.ai.models.iter().any(|m| m.name == config.id) {
                    return Err(anyhow!(
                        "AI输入验证错误: Model with ID '{}' already exists",
                        config.id
                    ));
                }

                // 转换为简单模型配置并添加
                let simple_model = crate::config::AISimpleModelConfig {
                    name: config.name,
                    provider: match config.provider {
                        crate::ai::AIProvider::OpenAI => "openai".to_string(),
                        crate::ai::AIProvider::Claude => "anthropic".to_string(),
                        _ => "custom".to_string(),
                    },
                    enabled: config.is_default.unwrap_or(false),
                };
                app_config.ai.models.push(simple_model);
                Ok(())
            })
            .await?;

        Ok(())
    }

    /// 更新模型配置
    pub async fn update_model(&self, model_id: &str, config: AIModelConfig) -> AppResult<()> {
        self.validate_model_config(&config)?;

        self.config_manager
            .update_config(|app_config| {
                // 查找并更新模型
                if let Some(model) = app_config.ai.models.iter_mut().find(|m| m.name == model_id) {
                    model.name = config.name;
                    model.provider = match config.provider {
                        crate::ai::AIProvider::OpenAI => "openai".to_string(),
                        crate::ai::AIProvider::Claude => "anthropic".to_string(),
                        _ => "custom".to_string(),
                    };
                    model.enabled = config.is_default.unwrap_or(false);
                    Ok(())
                } else {
                    Err(anyhow!(
                        "AI输入验证错误: Model with ID '{model_id}' not found"
                    ))
                }
            })
            .await?;

        Ok(())
    }

    /// 删除模型配置
    pub async fn remove_model(&self, model_id: &str) -> AppResult<()> {
        self.config_manager
            .update_config(|app_config| {
                // 查找模型索引
                if let Some(index) = app_config.ai.models.iter().position(|m| m.name == model_id) {
                    // 删除模型
                    app_config.ai.models.remove(index);
                } else {
                    return Err(anyhow!(
                        "AI输入验证错误: Model with ID '{model_id}' not found"
                    ));
                }
                Ok(())
            })
            .await?;

        Ok(())
    }

    /// 验证模型配置
    fn validate_model_config(&self, config: &AIModelConfig) -> AppResult<()> {
        if config.id.is_empty() {
            return Err(anyhow!("AI输入验证错误: Model ID cannot be empty"));
        }

        if config.name.is_empty() {
            return Err(anyhow!("AI输入验证错误: Model name cannot be empty"));
        }

        if config.api_url.is_empty() {
            return Err(anyhow!("AI输入验证错误: API URL cannot be empty"));
        }

        if config.api_key.is_empty() {
            return Err(anyhow!("AI输入验证错误: API key cannot be empty"));
        }

        if config.model.is_empty() {
            return Err(anyhow!("AI输入验证错误: Model name cannot be empty"));
        }

        Ok(())
    }
}
