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
