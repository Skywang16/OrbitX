//! Anthropic Messages API 类型定义
//!
//! 直接映射 Anthropic 官方 API 结构，无中间抽象层
//! 参考: https://docs.claude.com/en/api/messages
//!
//! ## 设计原则
//!
//! 1. **零抽象**：直接翻译官方API文档，字段名、类型完全一致
//! 2. **Serde优先**：用 `#[serde]` 属性处理所有序列化需求
//! 3. **可选字段**：用 `Option<T>` + `skip_serializing_if` 处理可选参数
//! 4. **扩展性**：新字段默认不会破坏旧代码（serde的优势）
//!
//! ## 与 TypeScript SDK 对应关系
//!
//! | Rust 类型 | TypeScript (@anthropic-ai/sdk) |
//! |-----------|--------------------------------|
//! | `MessageParam` | `Anthropic.Messages.MessageParam` |
//! | `ContentBlock` | `Anthropic.TextBlockParam \| Anthropic.ImageBlockParam \| ...` |
//! | `ToolUseBlock` | `Anthropic.ToolUseBlock` |
//! | `ToolResultBlockParam` | `Anthropic.ToolResultBlockParam` |

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ============================================================
// 消息结构 (Messages)
// ============================================================

/// 消息参数 - 对应 `Anthropic.Messages.MessageParam`
///
/// 用于构建发送给API的消息历史
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageParam {
    /// 消息角色：user 或 assistant
    pub role: MessageRole,
    /// 消息内容：可以是纯文本或结构化内容块数组
    pub content: MessageContent,
}

/// 消息角色
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// 用户消息
    User,
    /// 助手消息
    Assistant,
}

/// 消息内容 - 可以是字符串或内容块数组
///
/// 对应 TypeScript: `string | ContentBlock[]`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessageContent {
    /// 纯文本内容
    Text(String),
    /// 结构化内容块数组（支持文本、图片、工具调用等）
    Blocks(Vec<ContentBlock>),
}

impl MessageContent {
    /// 创建文本内容
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// 创建单个文本块
    pub fn text_block(text: impl Into<String>) -> Self {
        Self::Blocks(vec![ContentBlock::Text {
            text: text.into(),
            cache_control: None,
        }])
    }
}

// ============================================================
// 内容块 (Content Blocks)
// ============================================================

/// 内容块 - 对应 Anthropic 的各种 BlockParam 类型
///
/// 使用 `#[serde(tag = "type")]` 实现 tagged union
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// 文本块 - 对应 `TextBlockParam`
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },

    /// 图片块 - 对应 `ImageBlockParam`
    Image {
        source: ImageSource,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },

    /// 工具调用（assistant发出）- 对应 `ToolUseBlock`
    ToolUse {
        /// 工具调用的唯一ID（格式：toolu_xxx）
        id: String,
        /// 工具名称
        name: String,
        /// 工具输入参数（JSON对象）
        input: JsonValue,
    },

    /// 工具结果（user返回）- 对应 `ToolResultBlockParam`
    ToolResult {
        /// 对应的工具调用ID
        tool_use_id: String,
        /// 工具执行结果（可以是字符串或包含图片的块数组）
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<ToolResultContent>,
        /// 是否为错误结果
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },

    /// Thinking 块（Extended Thinking 特性）
    ///
    /// 当模型使用 Extended Thinking 时返回
    #[serde(rename = "thinking")]
    Thinking {
        thinking: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
}

/// 图片源 - 支持三种方式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Base64 编码的图片
    Base64 {
        /// MIME类型（image/jpeg, image/png, image/gif, image/webp）
        media_type: String,
        /// Base64编码的图片数据
        data: String,
    },
    /// URL 链接的图片
    Url {
        /// 图片URL
        url: String,
    },
    /// Files API 上传的文件
    #[serde(rename = "file")]
    FileId {
        /// 文件ID（通过 Files API 获取）
        file_id: String,
    },
}

/// 工具结果内容 - 可以是字符串或包含图片的块数组
///
/// 对应 TypeScript: `string | ToolResultBlock[]`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// 纯文本结果
    Text(String),
    /// 富内容结果（可包含文本和图片）
    Blocks(Vec<ToolResultBlock>),
}

/// 工具结果块 - 只支持文本和图片
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultBlock {
    /// 文本块
    Text { text: String },
    /// 图片块
    Image { source: ImageSource },
}

/// Prompt Cache 控制
///
/// 参考: https://docs.claude.com/en/docs/build-with-claude/prompt-caching
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheControl {
    /// 缓存类型（目前只支持 "ephemeral"）
    #[serde(rename = "type")]
    pub cache_type: String,
    /// 缓存TTL（可选：5m 或 1h）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
}

impl CacheControl {
    /// 创建临时缓存控制（默认5分钟）
    pub fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: None,
        }
    }

    /// 创建带TTL的临时缓存
    pub fn ephemeral_with_ttl(ttl: impl Into<String>) -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: Some(ttl.into()),
        }
    }
}

// ============================================================
// 工具定义 (Tools)
// ============================================================

/// 工具定义 - 对应 `Anthropic.Tool`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tool {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 输入参数的 JSON Schema
    pub input_schema: JsonValue,
}

impl Tool {
    /// 创建新工具定义
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: JsonValue,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

// ============================================================
// API 请求/响应
// ============================================================

/// CreateMessageRequest - Anthropic 消息创建请求
///
/// 对应 API: `POST /v1/messages`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    /// 模型ID（如 "claude-3-5-sonnet-20241022"）
    pub model: String,

    /// 消息历史
    pub messages: Vec<MessageParam>,

    /// 最大生成token数
    pub max_tokens: u32,

    /// System prompt（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemPrompt>,

    /// 工具定义列表（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// 温度参数 (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// 自定义停止序列
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// 是否流式返回
    #[serde(default)]
    pub stream: bool,

    /// Top-p 采样参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Top-k 采样参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    /// 元数据（用于追踪）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
}

/// System Prompt - 可以是字符串或包含 cache control 的块
///
/// 对应 TypeScript: `string | SystemBlock[]`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SystemPrompt {
    /// 纯文本
    Text(String),
    /// 支持缓存的块数组
    Blocks(Vec<SystemBlock>),
}

impl SystemPrompt {
    /// 创建带缓存控制的system prompt
    pub fn with_cache(text: impl Into<String>) -> Self {
        Self::Blocks(vec![SystemBlock {
            block_type: "text".to_string(),
            text: text.into(),
            cache_control: Some(CacheControl::ephemeral()),
        }])
    }
}

/// System 块
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemBlock {
    #[serde(rename = "type")]
    pub block_type: String, // "text"
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

/// 元数据
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
    /// 用户ID（用于追踪和统计）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// API 响应消息
///
/// 对应 `Anthropic.Messages.Message`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 消息ID
    pub id: String,

    /// 类型（固定为 "message"）
    #[serde(rename = "type")]
    pub message_type: String,

    /// 角色（固定为 "assistant"）
    pub role: MessageRole,

    /// 内容块数组
    pub content: Vec<ContentBlock>,

    /// 使用的模型
    pub model: String,

    /// 停止原因
    pub stop_reason: Option<StopReason>,

    /// 停止序列（如果是自定义stop_sequence触发）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,

    /// Token使用统计
    pub usage: Usage,
}

/// 停止原因
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// 模型自然结束
    EndTurn,
    /// 达到max_tokens限制
    MaxTokens,
    /// 遇到自定义stop_sequence
    StopSequence,
    /// 模型想要使用工具
    ToolUse,
}

/// Token 使用统计
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Usage {
    /// 输入token数
    pub input_tokens: u32,
    /// 输出token数
    pub output_tokens: u32,
    /// 缓存创建的输入token数（Prompt Caching）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    /// 缓存读取的输入token数（Prompt Caching）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

impl Usage {
    /// 总token数
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }

    /// 缓存节省的token数
    pub fn cache_savings(&self) -> u32 {
        self.cache_read_input_tokens.unwrap_or(0)
    }
}

// ============================================================
// 流式事件 (Streaming)
// ============================================================

/// SSE 流式事件
///
/// 参考: https://docs.claude.com/en/api/messages-streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// 消息开始
    MessageStart { message: MessageStartData },

    /// 内容块开始
    ContentBlockStart {
        index: usize,
        content_block: ContentBlockStart,
    },

    /// 内容块增量
    ContentBlockDelta { index: usize, delta: ContentDelta },

    /// 内容块结束
    ContentBlockStop { index: usize },

    /// 消息级别的变化（如stop_reason）
    MessageDelta {
        delta: MessageDeltaData,
        usage: Usage,
    },

    /// 消息结束
    MessageStop,

    /// 连接保活
    Ping,

    /// 错误
    Error { error: ErrorData },
}

/// 消息开始数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStartData {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String, // "message"
    pub role: MessageRole,
    pub model: String,
    pub usage: Usage,
}

/// 内容块开始数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockStart {
    /// 文本块开始
    Text { text: String },
    /// 工具调用开始
    ToolUse { id: String, name: String },
    /// Thinking块开始
    Thinking { thinking: String },
}

/// 内容增量
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentDelta {
    /// 文本增量
    #[serde(rename = "text_delta")]
    Text { text: String },
    /// 工具输入JSON增量（可能是不完整的JSON片段）
    #[serde(rename = "input_json_delta")]
    InputJson { partial_json: String },
    /// Thinking增量
    #[serde(rename = "thinking_delta")]
    Thinking { thinking: String },
}

/// 消息Delta数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeltaData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
}

/// 错误数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorData {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

// ============================================================
// 便捷构造函数
// ============================================================

impl MessageParam {
    /// 创建用户文本消息
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text(content.into()),
        }
    }

    /// 创建用户消息（支持多个内容块）
    pub fn user_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Blocks(blocks),
        }
    }

    /// 创建助手文本消息
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text(content.into()),
        }
    }

    /// 创建助手消息（支持多个内容块）
    pub fn assistant_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Blocks(blocks),
        }
    }
}

impl ContentBlock {
    /// 创建文本块
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            cache_control: None,
        }
    }

    /// 创建带缓存控制的文本块
    pub fn text_with_cache(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            cache_control: Some(CacheControl::ephemeral()),
        }
    }

    /// 创建Base64图片块
    pub fn image_base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource::Base64 {
                media_type: media_type.into(),
                data: data.into(),
            },
            cache_control: None,
        }
    }

    /// 创建URL图片块
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource::Url { url: url.into() },
            cache_control: None,
        }
    }

    /// 创建工具结果块
    pub fn tool_result(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: Some(ToolResultContent::Text(content.into())),
            is_error: None,
        }
    }

    /// 创建工具错误结果块
    pub fn tool_error(tool_use_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: Some(ToolResultContent::Text(error.into())),
            is_error: Some(true),
        }
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_message_param_serialization() {
        let msg = MessageParam::user("Hello, Claude!");
        let json = serde_json::to_value(&msg).unwrap();

        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "Hello, Claude!");
    }

    #[test]
    fn test_content_blocks() {
        let blocks = vec![
            ContentBlock::text("What's in this image?"),
            ContentBlock::image_url("https://example.com/image.jpg"),
        ];

        let msg = MessageParam::user_blocks(blocks);
        let json = serde_json::to_value(&msg).unwrap();

        assert_eq!(json["role"], "user");
        assert_eq!(json["content"][0]["type"], "text");
        assert_eq!(json["content"][1]["type"], "image");
    }

    #[test]
    fn test_tool_use_serialization() {
        let tool_use = ContentBlock::ToolUse {
            id: "toolu_123".to_string(),
            name: "get_weather".to_string(),
            input: json!({"location": "San Francisco"}),
        };

        let json = serde_json::to_value(&tool_use).unwrap();
        assert_eq!(json["type"], "tool_use");
        assert_eq!(json["id"], "toolu_123");
        assert_eq!(json["name"], "get_weather");
    }

    #[test]
    fn test_tool_result_serialization() {
        let tool_result = ContentBlock::tool_result("toolu_123", "Temperature: 72°F");

        let json = serde_json::to_value(&tool_result).unwrap();
        assert_eq!(json["type"], "tool_result");
        assert_eq!(json["tool_use_id"], "toolu_123");
        assert_eq!(json["content"], "Temperature: 72°F");
    }

    #[test]
    fn test_cache_control() {
        let block = ContentBlock::text_with_cache("Cached content");
        let json = serde_json::to_value(&block).unwrap();

        assert_eq!(json["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_system_prompt_with_cache() {
        let system = SystemPrompt::with_cache("You are a helpful assistant.");
        let json = serde_json::to_value(&system).unwrap();

        assert_eq!(json[0]["type"], "text");
        assert_eq!(json[0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_usage_calculations() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: Some(20),
            cache_read_input_tokens: Some(80),
        };

        assert_eq!(usage.total_tokens(), 150);
        assert_eq!(usage.cache_savings(), 80);
    }

    #[test]
    fn test_deserialization_from_api() {
        // 模拟 API 返回的 JSON
        let json_str = r#"{
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "Hello!"
                }
            ],
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 20
            }
        }"#;

        let message: Message = serde_json::from_str(json_str).unwrap();
        assert_eq!(message.id, "msg_123");
        assert_eq!(message.role, MessageRole::Assistant);
        assert_eq!(message.usage.total_tokens(), 30);
    }
}
