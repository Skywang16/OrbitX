use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::ipc::Channel;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use crate::agent::core::context::ToolCallResult;
use crate::agent::core::status::AgentTaskStatus;
use crate::agent::react::runtime::ReactRuntime;
use crate::agent::types::{Message, TaskEvent};
use crate::llm::anthropic_types::{MessageParam, SystemPrompt};

use super::chain::Chain;

/// 执行状态
pub(crate) struct ExecutionState {
    pub(crate) runtime_status: AgentTaskStatus,
    pub(crate) system_prompt: Option<SystemPrompt>,
    pub(crate) system_prompt_overlay: Option<SystemPrompt>,
    /// 简化：用 Vec 替代 MessageRingBuffer
    pub(crate) messages: Vec<MessageParam>,
    pub(crate) message_sequence: i64,
    pub(crate) tool_results: Vec<ToolCallResult>,
    pub(crate) current_iteration: u32,
    pub(crate) error_count: u32,
}

impl ExecutionState {
    pub fn new(runtime_status: AgentTaskStatus) -> Self {
        Self {
            runtime_status,
            system_prompt: None,
            system_prompt_overlay: None,
            messages: Vec::new(),
            message_sequence: 0,
            tool_results: Vec::new(),
            current_iteration: 0,
            error_count: 0,
        }
    }

    pub fn messages_vec(&self) -> Vec<MessageParam>
    where
        MessageParam: Clone,
    {
        self.messages.clone()
    }
}

/// UI 消息状态（消息表的实时镜像）
#[derive(Default)]
pub(crate) struct MessageState {
    pub(crate) assistant_message: Option<Message>,
}

pub(crate) struct TaskStates {
    pub execution: RwLock<ExecutionState>,
    pub chain: RwLock<Chain>,
    pub messages: Mutex<MessageState>,
    pub react_runtime: Arc<RwLock<ReactRuntime>>,
    pub progress_channel: Mutex<Option<Channel<TaskEvent>>>,
    /// 简化的取消标志 - 用 AtomicBool 替代 CancellationToken
    pub aborted: Arc<AtomicBool>,
    pub abort_token: CancellationToken,
}

impl TaskStates {
    pub fn new(
        execution: ExecutionState,
        react_runtime: ReactRuntime,
        progress_channel: Option<Channel<TaskEvent>>,
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
