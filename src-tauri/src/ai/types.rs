/*!
 * AI相关的数据类型定义
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// 重新导出Repository中的类型
pub use crate::storage::repositories::ai_models::{AIModelConfig, AIProvider, ModelType};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AIContext {
    pub working_directory: Option<String>,
    pub command_history: Option<Vec<String>>,
    pub environment: Option<HashMap<String, String>>,
    pub current_command: Option<String>,
    pub last_output: Option<String>,
    pub system_info: Option<SystemInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub shell: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIResponse {
    pub content: String,
    pub response_type: AIResponseType,
    pub suggestions: Option<Vec<String>>,
    pub metadata: Option<AIResponseMetadata>,
    pub error: Option<AIErrorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIErrorInfo {
    pub message: String,
    pub code: Option<String>,
    pub details: Option<serde_json::Value>,
    pub provider_response: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AIResponseType {
    Chat,
    Text,
    Code,
    Command,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIResponseMetadata {
    pub model: Option<String>,
    pub tokens_used: Option<u32>,
    pub response_time: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AISettings {
    pub models: Vec<AIModelConfig>,
    pub features: AIFeatureSettings,
    pub performance: AIPerformanceSettings,
}

impl AISettings {
    /// 根据ID查找模型
    pub fn find_model(&self, id: &str) -> Option<&AIModelConfig> {
        self.models.iter().find(|m| m.id == id)
    }

    /// 根据ID查找模型（可变引用）
    pub fn find_model_mut(&mut self, id: &str) -> Option<&mut AIModelConfig> {
        self.models.iter_mut().find(|m| m.id == id)
    }

    /// 添加模型
    pub fn add_model(&mut self, model: AIModelConfig) -> Result<(), String> {
        if self.models.iter().any(|m| m.id == model.id) {
            return Err(format!("Model with ID '{}' already exists", model.id));
        }

        self.models.push(model);
        Ok(())
    }

    /// 移除模型
    pub fn remove_model(&mut self, id: &str) -> Result<(), String> {
        let index = self
            .models
            .iter()
            .position(|m| m.id == id)
            .ok_or_else(|| format!("Model with ID '{id}' not found"))?;

        self.models.remove(index);
        Ok(())
    }

    /// 验证设置
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct AIFeatureSettings {
    pub chat: ChatSettings,
    pub user_rules: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSettings {
    pub enabled: bool,
    pub max_history_length: usize,
    pub auto_save_history: bool,
    pub context_window_size: usize,
}

impl Default for ChatSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_history_length: usize::MAX,
            auto_save_history: true,
            context_window_size: 4000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIPerformanceSettings {
    pub request_timeout: u64,
    pub max_concurrent_requests: usize,
    pub cache_enabled: bool,
    pub cache_ttl: u64,
}

impl Default for AIPerformanceSettings {
    fn default() -> Self {
        Self {
            request_timeout: 30,
            max_concurrent_requests: 5,
            cache_enabled: true,
            cache_ttl: 3600,
        }
    }
}

/// AI配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIConfig {
    pub max_context_tokens: u32,
    pub model_name: String,
    pub enable_semantic_compression: bool,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 4096,
            model_name: "default-model".to_string(),
            enable_semantic_compression: false,
        }
    }
}
