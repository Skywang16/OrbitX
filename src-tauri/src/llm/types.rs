use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LLM 消息内容部分
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LLMMessagePart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "file")]
    File {
        #[serde(rename = "mimeType")]
        mime_type: String,
        data: String, // base64 encoded
    },
    #[serde(rename = "tool-call")]
    ToolCall {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        tool_name: String,
        args: serde_json::Value,
    },
    #[serde(rename = "tool-result")]
    ToolResult {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        tool_name: String,
        result: serde_json::Value,
    },
}

/// LLM 请求消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMessage {
    pub role: String, // "system", "user", "assistant", "tool"
    pub content: LLMMessageContent,
}

/// 消息内容，支持字符串或结构化内容
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LLMMessageContent {
    Text(String),
    Parts(Vec<LLMMessagePart>),
}

/// LLM 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMTool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// LLM 工具调用
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LLMToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// LLM 请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub model: String, // 改为标准的 model 字段
    pub messages: Vec<LLMMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<LLMTool>>,
    pub tool_choice: Option<String>,
    pub stream: bool,
}

/// LLM 使用统计
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LLMUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// LLM 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub finish_reason: String,
    pub tool_calls: Option<Vec<LLMToolCall>>,
    pub usage: Option<LLMUsage>,
}

/// LLM 流式数据块类型 - 简化设计，符合 OpenAI 标准
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LLMStreamChunk {
    /// 内容增量 - 对应 OpenAI 的 choices[0].delta
    Delta {
        /// 文本内容增量
        content: Option<String>,
        /// 工具调用增量
        tool_calls: Option<Vec<LLMToolCall>>,
    },
    /// 流式完成 - 对应 OpenAI 的完成信号
    Finish {
        /// 完成原因
        finish_reason: String,
        /// 使用统计
        usage: Option<LLMUsage>,
    },
    /// 错误
    Error {
        /// 错误信息
        error: String,
    },
}

/// LLM 提供商类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LLMProviderType {
    OpenAI,
    Anthropic,
    Gemini,
    Qwen,
    Custom,
}

/// LLM 提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProviderConfig {
    pub provider_type: LLMProviderType,
    pub api_key: String,
    pub api_url: Option<String>,
    pub model: String,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

/// LLM 错误类型
#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
}

/// LLM 结果类型
pub type LLMResult<T> = Result<T, LLMError>;
