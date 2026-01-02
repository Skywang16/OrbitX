use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 消息 - 用户或助手的一条完整消息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: i64,
    pub session_id: i64,
    pub role: MessageRole,
    pub status: MessageStatus,
    pub blocks: Vec<Block>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub token_usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    Streaming,
    Completed,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: Option<i64>,
    pub cache_write_tokens: Option<i64>,
}

/// 内容块 - 消息的组成单元
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    UserText(UserTextBlock),
    UserImage(UserImageBlock),
    Thinking(ThinkingBlock),
    Text(TextBlock),
    Tool(ToolBlock),
    Error(ErrorBlock),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTextBlock {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserImageBlock {
    pub data_url: String,
    pub mime_type: String,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingBlock {
    pub id: String,
    pub content: String,
    pub is_streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextBlock {
    pub id: String,
    pub content: String,
    pub is_streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolBlock {
    pub id: String,
    pub name: String,
    pub status: ToolStatus,
    pub input: Value,
    pub output: Option<ToolOutput>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolStatus {
    Running,
    Completed,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolOutput {
    pub content: Value,
    pub cancel_reason: Option<String>,
    pub ext: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBlock {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

/// 任务进度事件（前端唯一输入）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskEvent {
    #[serde(rename_all = "camelCase")]
    TaskCreated {
        task_id: String,
        session_id: i64,
        workspace_path: String,
    },

    #[serde(rename_all = "camelCase")]
    MessageCreated { message: Message },

    #[serde(rename_all = "camelCase")]
    BlockAppended { message_id: i64, block: Block },

    #[serde(rename_all = "camelCase")]
    BlockUpdated {
        message_id: i64,
        block_id: String,
        block: Block,
    },

    #[serde(rename_all = "camelCase")]
    MessageFinished {
        message_id: i64,
        status: MessageStatus,
        finished_at: DateTime<Utc>,
        duration_ms: i64,
        token_usage: Option<TokenUsage>,
    },

    #[serde(rename_all = "camelCase")]
    TaskCompleted { task_id: String },

    #[serde(rename_all = "camelCase")]
    TaskError { task_id: String, error: ErrorBlock },

    #[serde(rename_all = "camelCase")]
    TaskCancelled { task_id: String },

    /// 工具执行确认请求（前端需要弹窗并回传 decision）
    #[serde(rename_all = "camelCase")]
    ToolConfirmationRequested {
        task_id: String,
        request_id: String,
        workspace_path: String,
        tool_name: String,
        summary: String,
    },
}
