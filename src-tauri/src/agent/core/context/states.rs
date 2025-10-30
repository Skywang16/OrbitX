use std::sync::Arc;
use tauri::ipc::Channel;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use crate::agent::core::context::ToolCallResult;
use crate::agent::core::ring_buffer::MessageRingBuffer;
use crate::agent::core::status::AgentTaskStatus;
use crate::agent::events::TaskProgressPayload;
use crate::agent::persistence::AgentExecution;
use crate::agent::react::runtime::ReactRuntime;
use crate::agent::types::TaskDetail;
use crate::agent::ui::UiStep;
use crate::llm::anthropic_types::{MessageParam, SystemPrompt};

use super::chain::Chain;

const MAX_MESSAGE_HISTORY: usize = 100;

/// 执行状态
pub(crate) struct ExecutionState {
    pub(crate) record: AgentExecution,
    pub(crate) runtime_status: AgentTaskStatus,
    pub(crate) system_prompt: Option<SystemPrompt>,
    pub(crate) messages: MessageRingBuffer<MessageParam, MAX_MESSAGE_HISTORY>,
    pub(crate) message_sequence: i64,
    pub(crate) tool_results: Vec<ToolCallResult>,
    pub(crate) current_iteration: Option<Arc<crate::agent::state::iteration::IterationContext>>,
    pub(crate) ui_assistant_message_id: Option<i64>,
}

impl ExecutionState {
    pub fn new(record: AgentExecution, runtime_status: AgentTaskStatus) -> Self {
        Self {
            record,
            runtime_status,
            system_prompt: None,
            messages: MessageRingBuffer::new(),
            message_sequence: 0,
            tool_results: Vec::new(),
            current_iteration: None,
            ui_assistant_message_id: None,
        }
    }

    pub fn messages_vec(&self) -> Vec<MessageParam>
    where
        MessageParam: Clone,
    {
        self.messages.iter().cloned().collect()
    }
}

/// 规划状态
pub(crate) struct PlanningState {
    pub(crate) chain: Chain,
    pub(crate) conversation: Vec<String>,
    pub(crate) current_node_id: Option<String>,
    pub(crate) task_detail: Option<TaskDetail>,
    pub(crate) root_task_id: Option<String>,
    pub(crate) parent_task_id: Option<String>,
    pub(crate) children: Vec<String>,
}

impl PlanningState {
    pub fn new(user_prompt: String) -> Self {
        Self {
            chain: Chain::new(user_prompt),
            conversation: Vec::new(),
            current_node_id: None,
            task_detail: None,
            root_task_id: None,
            parent_task_id: None,
            children: Vec::new(),
        }
    }
}

/// UI状态
#[derive(Default)]
pub(crate) struct UiState {
    pub(crate) steps: Vec<UiStep>,
}

pub(crate) struct CancellationState {
    pub(crate) main_token: CancellationToken,
    pub(crate) step_tokens: Vec<CancellationToken>,
}

impl Default for CancellationState {
    fn default() -> Self {
        Self {
            main_token: CancellationToken::new(),
            step_tokens: Vec::new(),
        }
    }
}

pub(crate) struct TaskStates {
    pub execution: Arc<RwLock<ExecutionState>>,
    pub planning: Arc<RwLock<PlanningState>>,
    pub ui: Arc<Mutex<UiState>>,
    pub react_runtime: Arc<RwLock<ReactRuntime>>,
    pub progress_channel: Arc<Mutex<Option<Channel<TaskProgressPayload>>>>,
    pub cancellation: Arc<Mutex<CancellationState>>,
}

impl TaskStates {
    pub fn new(
        execution: ExecutionState,
        planning: PlanningState,
        react_runtime: ReactRuntime,
        progress_channel: Option<Channel<TaskProgressPayload>>,
    ) -> Self {
        Self {
            execution: Arc::new(RwLock::new(execution)),
            planning: Arc::new(RwLock::new(planning)),
            ui: Arc::new(Mutex::new(UiState::default())),
            react_runtime: Arc::new(RwLock::new(react_runtime)),
            progress_channel: Arc::new(Mutex::new(progress_channel)),
            cancellation: Arc::new(Mutex::new(CancellationState::default())),
        }
    }
}
