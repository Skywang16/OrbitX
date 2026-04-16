use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agent::types::{Block, ErrorBlock, Message, MessageRole, MessageStatus, TokenUsage};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadMeta {
    pub thread_id: i64,
    pub workspace_path: String,
    pub title: String,
    pub agent_type: String,
    pub parent_thread_id: Option<i64>,
    pub spawned_by_tool_call_id: Option<String>,
    pub model_id: Option<String>,
    pub provider_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RolloutLine {
    pub timestamp: DateTime<Utc>,
    pub item: RolloutItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RolloutItem {
    ThreadMeta(ThreadMeta),
    AgentRunCreated {
        run_id: String,
        thread_id: i64,
        workspace_path: String,
    },
    UserMessageCreated {
        run_id: String,
        message: Message,
    },
    AssistantMessageCreated {
        run_id: String,
        message: Message,
    },
    BlockAppended {
        run_id: String,
        message_id: i64,
        block: Block,
    },
    BlockUpdated {
        run_id: String,
        message_id: i64,
        block_id: String,
        block: Block,
    },
    MessageFinished {
        run_id: String,
        message_id: i64,
        status: MessageStatus,
        finished_at: DateTime<Utc>,
        duration_ms: i64,
        token_usage: Option<TokenUsage>,
    },
    AgentRunCompleted {
        run_id: String,
    },
    AgentRunCancelled {
        run_id: String,
    },
    AgentRunError {
        run_id: String,
        error: ErrorBlock,
    },
    SpawnAgent {
        call_id: String,
        parent_thread_id: i64,
        child_thread_id: i64,
        agent_type: String,
        description: String,
    },
    AgentInput {
        call_id: String,
        thread_id: i64,
        input: String,
    },
    AgentWaiting {
        thread_id: i64,
        timeout_ms: Option<u64>,
    },
    AgentResumed {
        thread_id: i64,
    },
    AgentClosed {
        thread_id: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayedThread {
    pub meta: Option<ThreadMeta>,
    pub messages: Vec<Message>,
}

impl ReplayedThread {
    pub fn empty() -> Self {
        Self {
            meta: None,
            messages: Vec::new(),
        }
    }
}

pub fn empty_message(
    id: i64,
    thread_id: i64,
    role: MessageRole,
    agent_type: String,
    parent_message_id: Option<i64>,
    created_at: DateTime<Utc>,
) -> Message {
    Message {
        id,
        thread_id,
        role,
        agent_type,
        parent_message_id,
        status: MessageStatus::Streaming,
        blocks: Vec::new(),
        is_summary: false,
        is_internal: false,
        model_id: None,
        provider_id: None,
        created_at,
        finished_at: None,
        duration_ms: None,
        token_usage: None,
        context_usage: None,
    }
}
