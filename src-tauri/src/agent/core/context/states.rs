use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::ipc::Channel;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use crate::agent::core::status::AgentTaskStatus;
use crate::agent::react::runtime::ReactRuntime;
use crate::agent::types::{AgentRunEvent, Message};
use crate::llm::anthropic_types::{MessageParam, SystemPrompt};

use super::chain::Chain;

/// Execution state
pub(crate) struct ExecutionState {
    pub(crate) runtime_status: AgentTaskStatus,
    pub(crate) system_prompt: Option<SystemPrompt>,
    pub(crate) system_prompt_overlay: Option<SystemPrompt>,
    pub(crate) developer_context: Vec<String>,
    /// Per-turn supplemental developer context (e.g. `<subagents>` block).
    /// Rebuilt each iteration by the orchestrator; merged into developer_context
    /// when the LLM request is constructed.
    pub(crate) per_turn_developer_context: Vec<String>,
    /// Used by subagent/lifecycle init only; orchestrator reloads from DB.
    pub(crate) messages: Vec<MessageParam>,
    pub(crate) message_sequence: i64,
    pub(crate) current_iteration: u32,
    pub(crate) error_count: u32,
}

impl ExecutionState {
    pub fn new(runtime_status: AgentTaskStatus) -> Self {
        Self {
            runtime_status,
            system_prompt: None,
            system_prompt_overlay: None,
            developer_context: Vec::new(),
            per_turn_developer_context: Vec::new(),
            messages: Vec::new(),
            message_sequence: 0,
            current_iteration: 0,
            error_count: 0,
        }
    }

    pub fn messages_vec(&self) -> Vec<MessageParam> {
        self.messages.clone()
    }
}

/// UI message state (real-time mirror of message table)
#[derive(Default)]
pub(crate) struct MessageState {
    pub(crate) assistant_message: Option<Message>,
}

pub(crate) struct TaskStates {
    pub execution: RwLock<ExecutionState>,
    pub chain: RwLock<Chain>,
    pub messages: Mutex<MessageState>,
    pub react_runtime: Arc<RwLock<ReactRuntime>>,
    pub progress_channel: Mutex<Option<Channel<AgentRunEvent>>>,
    /// Simplified cancellation flag - use AtomicBool instead of CancellationToken
    pub aborted: Arc<AtomicBool>,
    pub abort_token: CancellationToken,
}

impl TaskStates {
    pub fn new(
        execution: ExecutionState,
        react_runtime: ReactRuntime,
        progress_channel: Option<Channel<AgentRunEvent>>,
    ) -> Self {
        Self {
            execution: RwLock::new(execution),
            chain: RwLock::new(Chain::new()),
            messages: Mutex::new(MessageState::default()),
            react_runtime: Arc::new(RwLock::new(react_runtime)),
            progress_channel: Mutex::new(progress_channel),
            aborted: Arc::new(AtomicBool::new(false)),
            abort_token: CancellationToken::new(),
        }
    }
}
