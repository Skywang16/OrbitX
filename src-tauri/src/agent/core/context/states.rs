use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::ipc::Channel;
use tokio::sync::{Mutex, RwLock};

use crate::agent::core::context::ToolCallResult;
use crate::agent::core::status::AgentTaskStatus;
use crate::agent::persistence::AgentExecution;
use crate::agent::react::runtime::ReactRuntime;
use crate::agent::types::{Message, TaskEvent};
use crate::llm::anthropic_types::{MessageParam, SystemPrompt};

use super::chain::Chain;

/// 执行状态
pub(crate) struct ExecutionState {
    pub(crate) record: AgentExecution,
    pub(crate) runtime_status: AgentTaskStatus,
    pub(crate) system_prompt: Option<SystemPrompt>,
    /// 简化：用 Vec 替代 MessageRingBuffer
    pub(crate) messages: Vec<MessageParam>,
    pub(crate) message_sequence: i64,
    pub(crate) tool_results: Vec<ToolCallResult>,
}

impl ExecutionState {
    pub fn new(record: AgentExecution, runtime_status: AgentTaskStatus) -> Self {
        Self {
            record,
            runtime_status,
            system_prompt: None,
            messages: Vec::new(),
            message_sequence: 0,
            tool_results: Vec::new(),
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
    pub execution: Arc<RwLock<ExecutionState>>,
    pub chain: Arc<RwLock<Chain>>,
    pub messages: Arc<Mutex<MessageState>>,
    pub react_runtime: Arc<RwLock<ReactRuntime>>,
    pub progress_channel: Arc<Mutex<Option<Channel<TaskEvent>>>>,
    /// 简化的取消标志 - 用 AtomicBool 替代 CancellationToken
    pub aborted: Arc<AtomicBool>,
}

impl TaskStates {
    pub fn new(
        execution: ExecutionState,
        react_runtime: ReactRuntime,
        progress_channel: Option<Channel<TaskEvent>>,
    ) -> Self {
        Self {
            execution: Arc::new(RwLock::new(execution)),
            chain: Arc::new(RwLock::new(Chain::new())),
            messages: Arc::new(Mutex::new(MessageState::default())),
            react_runtime: Arc::new(RwLock::new(react_runtime)),
            progress_channel: Arc::new(Mutex::new(progress_channel)),
            aborted: Arc::new(AtomicBool::new(false)),
        }
    }
}
