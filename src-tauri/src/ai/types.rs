/*!
 * AI相关的数据类型定义
 */

use chrono::{DateTime, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;


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
    // 新增：会话上下文管理系统字段
    pub chat_history: Option<Vec<Message>>,
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
    pub user_prefix_prompt: Option<String>,
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
            max_history_length: usize::MAX, // 无限历史长度
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
            cache_ttl: 3600, // 1小时
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time: f64,
    pub tokens_used: u64,
    pub cache_hit_rate: Option<f64>,
    pub model_usage: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIHealthStatus {
    pub model_id: String,
    pub status: HealthStatus,
    pub last_checked: DateTime<Utc>,
    pub response_time: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheEntry<T> {
    pub key: String,
    pub value: T,
    pub timestamp: DateTime<Utc>,
    pub ttl: u64,
    pub hits: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub total_entries: usize,
    pub hit_rate: f64,
    pub memory_usage: usize,
    pub oldest_entry: Option<DateTime<Utc>>,
    pub newest_entry: Option<DateTime<Utc>>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamChunk {
    pub content: String,
    pub is_complete: bool,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// 流式响应类型别名
pub type AIStreamResponse =
    Pin<Box<dyn Stream<Item = Result<StreamChunk, crate::utils::error::AppError>> + Send>>;


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterCapabilities {
    pub supports_streaming: bool,
    pub supports_batch: bool,
    pub supports_function_calling: bool,
    pub supports_vision: bool,
    pub max_tokens: Option<u32>,
    pub max_batch_size: Option<usize>,
    pub supported_models: Vec<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckRequest {
    pub include_latency: bool,
    pub include_model_info: bool,
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckResponse {
    pub status: HealthStatus,
    pub latency: Option<u64>,
    pub model_info: Option<ModelInfo>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub name: String,
    pub version: Option<String>,
    pub context_length: Option<u32>,
    pub capabilities: Option<AdapterCapabilities>,
}


// 重新导出Repository中的会话和消息类型
pub use crate::storage::repositories::conversations::{Conversation, Message};
/// AI配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIConfig {
    pub max_context_tokens: u32, // 上下文最大token (当前版本暂未强制执行)
    pub model_name: String,      // 使用的模型名称
    pub enable_semantic_compression: bool, // 是否启用语义压缩 (Phase 5功能)
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
/// 上下文统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextStats {
    pub conversation_id: i64,
    pub total_messages: i32,
    pub summary_generated: bool,
    pub last_summary_at: Option<DateTime<Utc>>,
}
