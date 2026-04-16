use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Message - a complete message from user or assistant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: i64,
    #[serde(rename = "threadId")]
    pub thread_id: i64,
    pub role: MessageRole,
    pub agent_type: String,
    pub parent_message_id: Option<i64>,
    pub status: MessageStatus,
    pub blocks: Vec<Block>,
    pub is_summary: bool,
    pub is_internal: bool,
    pub model_id: Option<String>,
    pub provider_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub token_usage: Option<TokenUsage>,
    pub context_usage: Option<ContextUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    Streaming,
    Completed,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: Option<i64>,
    pub cache_write_tokens: Option<i64>,
}

/// Context usage information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextUsage {
    /// Current number of tokens used
    pub tokens_used: u32,
    /// Total context window size
    pub context_window: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SubagentStatus {
    Pending,
    Running,
    Completed,
    Cancelled,
    Error,
}

impl SubagentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Error => "error",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "pending" => Self::Pending,
            "running" => Self::Running,
            "completed" => Self::Completed,
            "cancelled" => Self::Cancelled,
            "error" => Self::Error,
            _ => Self::Error,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SubagentRecord {
    pub id: String,
    pub parent_thread_id: i64,
    pub child_thread_id: i64,
    pub parent_message_id: i64,
    pub name: String,
    pub profile: String,
    pub task_title: String,
    pub status: SubagentStatus,
    pub latest_activity: Option<String>,
    pub final_summary: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

/// Content block - building unit of a message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    UserText(UserTextBlock),
    UserImage(UserImageBlock),
    Thinking(ThinkingBlock),
    Text(TextBlock),
    Tool(ToolBlock),
    AgentSwitch(AgentSwitchBlock),
    Error(ErrorBlock),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserTextBlock {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserImageBlock {
    pub data_url: String,
    pub mime_type: String,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingBlock {
    pub id: String,
    pub content: String,
    pub is_streaming: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<crate::llm::anthropic_types::ReasoningBlockMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextBlock {
    pub id: String,
    pub content: String,
    pub is_streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolBlock {
    pub id: String,
    pub call_id: String,
    pub name: String,
    pub status: ToolStatus,
    pub input: Value,
    pub output: Option<ToolOutput>,
    pub compacted_at: Option<DateTime<Utc>>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolStatus {
    Pending,
    Running,
    Completed,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolOutput {
    pub content: Value,
    pub title: Option<String>,
    pub metadata: Option<Value>,
    pub cancel_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSwitchBlock {
    pub from_agent: String,
    pub to_agent: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBlock {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

/// Agent run progress event streamed to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentRunEvent {
    #[serde(rename_all = "camelCase")]
    #[serde(rename = "agent_run_created")]
    AgentRunCreated {
        run_id: String,
        thread_id: i64,
        workspace_path: String,
    },

    #[serde(rename_all = "camelCase")]
    MessageCreated { run_id: String, message: Message },

    #[serde(rename_all = "camelCase")]
    SubagentCreated {
        run_id: String,
        subagent: SubagentRecord,
    },

    #[serde(rename_all = "camelCase")]
    SubagentUpdated {
        run_id: String,
        subagent: SubagentRecord,
    },

    #[serde(rename_all = "camelCase")]
    BlockAppended {
        run_id: String,
        message_id: i64,
        block: Block,
    },

    #[serde(rename_all = "camelCase")]
    BlockUpdated {
        run_id: String,
        message_id: i64,
        block_id: String,
        block: Block,
    },

    #[serde(rename_all = "camelCase")]
    MessageFinished {
        run_id: String,
        message_id: i64,
        status: MessageStatus,
        finished_at: DateTime<Utc>,
        duration_ms: i64,
        token_usage: Option<TokenUsage>,
        context_usage: Option<ContextUsage>,
    },

    #[serde(rename_all = "camelCase")]
    #[serde(rename = "agent_run_completed")]
    AgentRunCompleted { run_id: String },

    #[serde(rename_all = "camelCase")]
    #[serde(rename = "agent_run_error")]
    AgentRunError { run_id: String, error: ErrorBlock },

    #[serde(rename_all = "camelCase")]
    #[serde(rename = "agent_run_cancelled")]
    AgentRunCancelled { run_id: String },

    /// Tool execution confirmation request (frontend needs to show dialog and return decision)
    #[serde(rename_all = "camelCase")]
    ToolConfirmationRequested {
        run_id: String,
        request_id: String,
        workspace_path: String,
        tool_name: String,
        summary: String,
    },

    /// LLM request is being retried (connection/rate-limit/server error)
    #[serde(rename_all = "camelCase")]
    #[serde(rename = "agent_run_retrying")]
    AgentRunRetrying {
        run_id: String,
        attempt: u32,
        max_attempts: u32,
        reason: String,
        error_message: String,
        retry_in_ms: u64,
    },
}
