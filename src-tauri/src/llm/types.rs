use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OAuth 配置（轻量级，用于运行时）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthRuntimeConfig {
    pub provider: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Option<i64>,
}

/// Provider 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProviderConfig {
    pub provider_type: String,
    pub api_key: String,
    pub api_url: Option<String>,
    pub options: Option<HashMap<String, serde_json::Value>>,
    /// OAuth 配置（如果使用 OAuth 认证）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_config: Option<OAuthRuntimeConfig>,
}

/// Embedding 请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: Vec<String>,
    pub encoding_format: Option<String>,
    pub dimensions: Option<usize>,
}

/// Embedding 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub data: Vec<EmbeddingData>,
    pub model: String,
    pub usage: Option<LLMUsage>,
}

/// 单个 embedding 数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
    pub index: usize,
    pub object: String,
}

/// 使用统计（仅用于 Embeddings）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LLMUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
