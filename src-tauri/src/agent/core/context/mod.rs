pub mod chain;
pub mod states;

use std::convert::TryFrom;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::ipc::Channel;
use tokio::sync::RwLock;
use tokio::time::sleep;

use self::chain::Chain;
use self::states::{ExecutionState, PlanningState, TaskStates};
use crate::agent::config::{AgentConfig, TaskExecutionConfig};
use crate::agent::context::FileContextTracker;
use crate::agent::core::executor::ImageAttachment;
use crate::agent::core::status::AgentTaskStatus;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::persistence::{
    AgentExecution, AgentPersistence, ExecutionStatus, MessageRole,
};
use crate::agent::react::runtime::ReactRuntime;
use crate::agent::react::types::ReactRuntimeConfig;
use crate::agent::state::manager::{
    StateEventEmitter, StateManager, TaskState, TaskStatus, TaskThresholds,
};
use crate::agent::state::session::SessionContext;
use crate::agent::tools::ToolRegistry;
use crate::agent::types::{
    Block, ErrorBlock, MessageRole as UiMessageRole, MessageStatus, TaskDetail, TaskEvent,
    TokenUsage, ToolStatus, UserImageBlock, UserTextBlock,
};
use crate::agent::utils::tokenizer::count_text_tokens;
use crate::checkpoint::CheckpointService;
use crate::llm::anthropic_types::{
    ContentBlock, MessageContent, MessageParam, MessageRole as AnthropicRole, SystemPrompt,
    ToolResultContent,
};
use crate::storage::DatabaseManager;
use tokio_util::sync::CancellationToken;

pub struct TaskContext {
    pub task_id: Arc<str>,
    pub session_id: i64,
    pub user_prompt: Arc<str>,
    pub cwd: Arc<str>,
    config: TaskExecutionConfig,

    session: Arc<SessionContext>,
    tool_registry: Arc<ToolRegistry>,
    state_manager: Arc<StateManager>,
    checkpoint_service: Option<Arc<CheckpointService>>,
    active_checkpoint: Arc<RwLock<Option<ActiveCheckpoint>>>,

    pub(crate) states: TaskStates,

    pause_status: AtomicU8,
}

impl TaskContext {
    /// Construct a fresh context for a new task.
    pub async fn new(
        execution: AgentExecution,
        config: TaskExecutionConfig,
        workspace_path: String,
        tool_registry: Arc<ToolRegistry>,
        progress_channel: Option<Channel<TaskEvent>>,
        repositories: Arc<DatabaseManager>,
        agent_persistence: Arc<AgentPersistence>,
        checkpoint_service: Option<Arc<CheckpointService>>,
    ) -> TaskExecutorResult<Self> {
        let agent_config = AgentConfig::default();
        let runtime_config = ReactRuntimeConfig {
            max_iterations: agent_config.max_react_num,
            max_consecutive_errors: agent_config.max_react_error_streak,
        };

        let thresholds = TaskThresholds {
            max_consecutive_errors: agent_config.max_react_error_streak,
            max_iterations: agent_config.max_react_num,
        };

        let record = execution;
        let task_id = record.execution_id.clone();
        let session_id = record.session_id;
        let user_prompt = record.user_request.clone();
        let task_status = AgentTaskStatus::from(record.status);
        let current_iteration = record.current_iteration as u32;
        let error_count = record.error_count as u32;

        let mut task_state = TaskState::new(task_id.clone(), thresholds);
        task_state.iterations = current_iteration;
        task_state.consecutive_errors = error_count;
        task_state.task_status = map_status(&task_status);

        let normalized_workspace = workspace_path;

        let session = Arc::new(SessionContext::new(
            task_id.clone(),
            session_id,
            PathBuf::from(&normalized_workspace),
            user_prompt.clone(),
            config,
            Arc::clone(&repositories),
            Arc::clone(&agent_persistence),
        ));

        let execution = ExecutionState::new(record, task_status);
        let planning = PlanningState::new(user_prompt.clone());
        let react_runtime = ReactRuntime::new(runtime_config);

        let states = TaskStates::new(execution, planning, react_runtime, progress_channel);

        Ok(Self {
            task_id: Arc::from(task_id.as_str()),
            session_id,
            user_prompt: Arc::from(user_prompt.as_str()),
            cwd: Arc::from(normalized_workspace.as_str()),
            config,
            session,
            tool_registry,
            state_manager: Arc::new(StateManager::new(task_state, StateEventEmitter::new())),
            checkpoint_service,
            active_checkpoint: Arc::new(RwLock::new(None)),
            states,
            pause_status: AtomicU8::new(0),
        })
    }

    pub async fn set_progress_channel(&self, channel: Option<Channel<TaskEvent>>) {
        *self.states.progress_channel.lock().await = channel;
    }

    pub fn checkpointing_enabled(&self) -> bool {
        self.checkpoint_service.is_some()
    }

    pub async fn init_checkpoint(&self, message_id: i64) -> TaskExecutorResult<()> {
        let service = match &self.checkpoint_service {
            Some(service) => Arc::clone(service),
            None => return Ok(()),
        };

        let checkpoint = service
            .create_empty(self.session_id, message_id, Path::new(self.cwd.as_ref()))
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        {
            let mut guard = self.active_checkpoint.write().await;
            *guard = Some(ActiveCheckpoint {
                id: checkpoint.id,
                workspace_root: PathBuf::from(&checkpoint.workspace_path),
            });
        }

        Ok(())
    }

    pub async fn snapshot_file_before_edit(&self, path: &Path) -> TaskExecutorResult<()> {
        let service = match &self.checkpoint_service {
            Some(service) => Arc::clone(service),
            None => return Ok(()),
        };

        let handle = { self.active_checkpoint.read().await.clone() };

        if let Some(checkpoint) = handle {
            service
                .snapshot_file_before_edit(checkpoint.id, path, &checkpoint.workspace_root)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        }

        Ok(())
    }

    pub fn session(&self) -> Arc<SessionContext> {
        Arc::clone(&self.session)
    }

    pub fn file_tracker(&self) -> Arc<FileContextTracker> {
        self.session.file_tracker()
    }

    pub fn agent_persistence(&self) -> Arc<AgentPersistence> {
        self.session.agent_persistence()
    }

    pub fn tool_registry(&self) -> Arc<ToolRegistry> {
        Arc::clone(&self.tool_registry)
    }

    pub async fn status(&self) -> AgentTaskStatus {
        self.states.execution.read().await.runtime_status
    }

    pub async fn set_status(&self, status: AgentTaskStatus) -> TaskExecutorResult<()> {
        let (execution_status, current_iteration, error_count) = {
            let mut exec = self.states.execution.write().await;
            exec.runtime_status = status;
            exec.record.status = ExecutionStatus::from(&status);
            (
                exec.record.status,
                exec.record.current_iteration,
                exec.record.error_count,
            )
        };

        if matches!(
            status,
            AgentTaskStatus::Completed | AgentTaskStatus::Cancelled | AgentTaskStatus::Error
        ) {
            self.agent_persistence()
                .agent_executions()
                .mark_finished(&self.task_id, execution_status)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        } else {
            self.agent_persistence()
                .agent_executions()
                .update_status(
                    &self.task_id,
                    execution_status,
                    current_iteration,
                    error_count,
                )
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        }

        self.state_manager
            .update_task_status(map_status(&status), None)
            .await;
        Ok(())
    }

    /// Increment iteration counter and sync to storage.
    pub async fn increment_iteration(&self) -> TaskExecutorResult<u32> {
        let (current, current_raw, status, errors) = {
            let mut exec = self.states.execution.write().await;
            exec.record.current_iteration = exec.record.current_iteration.saturating_add(1);
            exec.message_sequence = 0;
            (
                exec.record.current_iteration as u32,
                exec.record.current_iteration,
                exec.record.status,
                exec.record.error_count,
            )
        };

        self.agent_persistence()
            .agent_executions()
            .update_status(&self.task_id, status, current_raw, errors)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        self.state_manager.increment_iteration().await;
        Ok(current)
    }

    /// Current iteration number.
    pub async fn current_iteration(&self) -> u32 {
        self.states.execution.read().await.record.current_iteration as u32
    }

    /// Increase error counter and persist.
    pub async fn increment_error_count(&self) -> TaskExecutorResult<u32> {
        let (count, status, iteration, errors) = {
            let mut exec = self.states.execution.write().await;
            exec.record.error_count = exec.record.error_count.saturating_add(1);
            (
                exec.record.error_count as u32,
                exec.record.status,
                exec.record.current_iteration,
                exec.record.error_count,
            )
        };
        self.agent_persistence()
            .agent_executions()
            .update_status(&self.task_id, status, iteration, errors)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        self.state_manager.increment_error_count().await;
        Ok(count)
    }

    pub async fn reset_error_count(&self) -> TaskExecutorResult<()> {
        let (status, iteration) = {
            let mut exec = self.states.execution.write().await;
            exec.record.error_count = 0;
            (exec.record.status, exec.record.current_iteration)
        };
        self.agent_persistence()
            .agent_executions()
            .update_status(&self.task_id, status, iteration, 0)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        self.state_manager.reset_error_count().await;
        Ok(())
    }

    /// Determine if execution should stop based on status and thresholds.
    pub async fn should_stop(&self) -> bool {
        let (status, iteration, errors) = {
            let exec = self.states.execution.read().await;
            (
                exec.runtime_status,
                exec.record.current_iteration as u32,
                exec.record.error_count as u32,
            )
        };
        if matches!(
            status,
            AgentTaskStatus::Cancelled | AgentTaskStatus::Completed | AgentTaskStatus::Error
        ) {
            return true;
        }
        self.state_manager.should_halt().await
            || iteration >= self.config.max_iterations
            || errors >= self.config.max_errors
    }

    /// Access the execution configuration (零成本访问).
    pub fn config(&self) -> &TaskExecutionConfig {
        &self.config
    }

    /// Access repositories (used by LLM/tool bridges).
    pub fn repositories(&self) -> Arc<DatabaseManager> {
        self.session.repositories()
    }

    pub fn state_manager(&self) -> Arc<StateManager> {
        Arc::clone(&self.state_manager)
    }

    /// 批量读取状态 - 减少锁争用，一次锁获取所有需要的数据
    /// 性能优化：避免多次 read().await
    pub(crate) async fn batch_read_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&ExecutionState) -> R,
    {
        let exec = self.states.execution.read().await;
        f(&exec)
    }

    #[allow(dead_code)]
    pub(crate) async fn batch_update_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut ExecutionState) -> R,
    {
        let mut exec = self.states.execution.write().await;
        f(&mut exec)
    }

    pub async fn with_chain<T>(&self, f: impl FnOnce(&Chain) -> T) -> T {
        let planning = self.states.planning.read().await;
        f(&planning.chain)
    }

    pub async fn with_chain_mut<T>(&self, f: impl FnOnce(&mut Chain) -> T) -> T {
        let mut planning = self.states.planning.write().await;
        f(&mut planning.chain)
    }

    /// Push a user intervention (manual conversation entry).
    pub async fn push_conversation_message(&self, message: String) {
        self.states
            .planning
            .write()
            .await
            .conversation
            .push(message);
    }

    pub async fn drain_conversation(&self) -> Vec<String> {
        let mut planning = self.states.planning.write().await;
        std::mem::take(&mut planning.conversation)
    }

    pub async fn set_task_detail(&self, task: Option<TaskDetail>) {
        self.states.planning.write().await.task_detail = task;
    }

    pub async fn task_detail(&self) -> Option<TaskDetail> {
        self.states.planning.read().await.task_detail.clone()
    }

    pub async fn attach_parent(&self, parent_task_id: String, root_task_id: Option<String>) {
        let mut planning = self.states.planning.write().await;
        planning.parent_task_id = Some(parent_task_id.clone());
        planning.root_task_id = Some(root_task_id.unwrap_or(parent_task_id));
    }

    pub async fn add_child(&self, child_task_id: String) {
        let mut planning = self.states.planning.write().await;
        if !planning.children.contains(&child_task_id) {
            planning.children.push(child_task_id);
        }
    }

    /// Read current node identifier.
    pub async fn current_node_id(&self) -> Option<String> {
        self.states.planning.read().await.current_node_id.clone()
    }

    pub async fn set_planning_node(&self, node_id: Option<String>) {
        self.states.planning.write().await.current_node_id = node_id;
    }

    /// 检查任务是否被中止
    /// 简化版：只检查 aborted 标志，无需锁
    pub fn check_aborted(&self, no_check_pause: bool) -> TaskExecutorResult<()> {
        if self.states.aborted.load(Ordering::SeqCst) {
            return Err(TaskExecutorError::TaskInterrupted.into());
        }
        if no_check_pause {
            return Ok(());
        }
        // 暂停检查保持同步方式
        let status = self.pause_status.load(Ordering::SeqCst);
        if status != 0 {
            // 如果暂停，返回错误让调用者处理
            return Err(TaskExecutorError::TaskInterrupted.into());
        }
        Ok(())
    }

    /// 异步检查任务是否被中止（带暂停等待）
    pub async fn check_aborted_async(&self, no_check_pause: bool) -> TaskExecutorResult<()> {
        if self.states.aborted.load(Ordering::SeqCst) {
            return Err(TaskExecutorError::TaskInterrupted.into());
        }
        if no_check_pause {
            return Ok(());
        }
        loop {
            if self.states.aborted.load(Ordering::SeqCst) {
                return Err(TaskExecutorError::TaskInterrupted.into());
            }
            let status = self.pause_status.load(Ordering::SeqCst);
            if status == 0 {
                break;
            }
            sleep(Duration::from_millis(500)).await;
        }
        Ok(())
    }

    /// 中止任务执行
    /// 简化版：只需设置 aborted 标志
    pub fn abort(&self) {
        self.states.aborted.store(true, Ordering::SeqCst);

        // 标记 react 运行时为中止状态
        let react_runtime = Arc::clone(&self.states.react_runtime);
        tokio::spawn(async move {
            let mut react = react_runtime.write().await;
            react.mark_abort();
        });
    }

    /// 检查是否已中止
    pub fn is_aborted(&self) -> bool {
        self.states.aborted.load(Ordering::SeqCst)
    }

    pub fn set_pause(&self, paused: bool, _abort_current_step: bool) {
        let new_status = if paused { 1 } else { 0 };
        self.pause_status.store(new_status, Ordering::SeqCst);
    }

    /// 为 LLM 流创建取消令牌
    /// 这个 token 会在任务 aborted 时自动取消
    pub fn create_stream_cancel_token(&self) -> CancellationToken {
        let token = CancellationToken::new();
        let aborted = Arc::clone(&self.states.aborted);
        let child_token = token.clone();

        // 监控 aborted 标志，如果被设置则取消 token
        tokio::spawn(async move {
            loop {
                if aborted.load(Ordering::SeqCst) {
                    child_token.cancel();
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
        });

        token
    }

    /// Add assistant message using Anthropic-native types (text and/or tool uses).
    pub async fn add_assistant_message(
        &self,
        text: Option<String>,
        tool_calls: Option<Vec<ContentBlock>>,
    ) {
        let content: MessageContent = match (text, tool_calls) {
            (Some(t), Some(mut calls)) => {
                // Put text as a Text block, then append tool_use blocks
                calls.insert(
                    0,
                    ContentBlock::Text {
                        text: t,
                        cache_control: None,
                    },
                );
                MessageContent::Blocks(calls)
            }
            (Some(t), None) => MessageContent::Text(t),
            (None, Some(calls)) => MessageContent::Blocks(calls),
            (None, None) => MessageContent::Text(String::new()),
        };

        {
            let mut exec = self.states.execution.write().await;
            exec.messages.push(MessageParam {
                role: AnthropicRole::Assistant,
                content: content.clone(),
            });
        }

        // Persist assistant visible content only as a string; do not modify DB schema.
        let rendered = render_message_content(&content);
        let _ = self
            .append_message(MessageRole::Assistant, &rendered, false)
            .await;
    }

    /// Append tool results as a user message with ToolResult blocks; also persist tool rows.
pub async fn add_tool_results(&self, results: Vec<ToolCallResult>) {
        let blocks: Vec<ContentBlock> = results
            .iter()
            .map(|r| ContentBlock::ToolResult {
                tool_use_id: r.call_id.clone(),
                content: Some(ToolResultContent::Text(
                    serde_json::to_string(&r.result).unwrap_or_else(|_| "{}".to_string()),
                )),
                is_error: Some(r.status != crate::agent::tools::ToolResultStatus::Success),
            })
            .collect();

        // Persist each tool result as its own Tool message entry
        for result in &results {
            if let Ok(serialized) = serde_json::to_string(result) {
                let _ = self
                    .append_message(MessageRole::Tool, &serialized, false)
                    .await;
            }
        }

        {
            let mut exec = self.states.execution.write().await;
            exec.tool_results.extend(results);
            exec.messages.push(MessageParam {
                role: AnthropicRole::User,
                content: MessageContent::Blocks(blocks),
            });
        }
    }

    // Deprecated in zero-abstraction model: initial prompts are handled explicitly by caller.
    // Retained signature temporarily, but now implemented using set_system_prompt + add_user_message semantics without DB writes for system.
    pub async fn set_initial_prompts(
        &self,
        system_prompt: String,
        user_prompt: String,
    ) -> TaskExecutorResult<()> {
        {
            let mut exec = self.states.execution.write().await;
            exec.system_prompt = Some(SystemPrompt::Text(system_prompt));
            exec.messages.clear();
            exec.message_sequence = 0;
        }
        self.add_user_message(user_prompt).await;
        Ok(())
    }

    pub async fn get_messages(&self) -> Vec<MessageParam> {
        self.states.execution.read().await.messages_vec()
    }

    pub async fn get_system_prompt(&self) -> Option<SystemPrompt> {
        self.states.execution.read().await.system_prompt.clone()
    }

    pub async fn add_user_message(&self, text: String) {
        self.add_user_message_with_images(text, None).await;
    }

    pub async fn add_user_message_with_images(
        &self,
        text: String,
        images: Option<&[ImageAttachment]>,
    ) {
        let content = if let Some(imgs) = images {
            // 构建包含图片和文本的内容块
            let mut blocks: Vec<ContentBlock> = imgs
                .iter()
                .filter_map(|img| {
                    // 从 data URL 提取 base64 数据
                    // 格式: data:image/jpeg;base64,/9j/4AAQ...
                    let parts: Vec<&str> = img.data_url.splitn(2, ',').collect();
                    if parts.len() == 2 {
                        Some(ContentBlock::Image {
                            source: crate::llm::anthropic_types::ImageSource::Base64 {
                                media_type: img.mime_type.clone(),
                                data: parts[1].to_string(),
                            },
                            cache_control: None,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            // 添加文本块
            if !text.is_empty() {
                blocks.push(ContentBlock::Text {
                    text: text.clone(),
                    cache_control: None,
                });
            }

            MessageContent::Blocks(blocks)
        } else {
            MessageContent::Text(text.clone())
        };

        {
            let mut exec = self.states.execution.write().await;
            exec.messages.push(MessageParam {
                role: AnthropicRole::User,
                content,
            });
        }
        let _ = self.append_message(MessageRole::User, &text, false).await;
    }

    pub async fn reset_message_state(&self) -> TaskExecutorResult<()> {
        {
            let mut exec = self.states.execution.write().await;
            exec.messages.clear();
            exec.message_sequence = 0;
        }

        self.agent_persistence()
            .execution_messages()
            .delete_for_execution(&self.task_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        Ok(())
    }

    /// Set system prompt in memory only; do not persist system message to DB.
    pub async fn set_system_prompt(&self, prompt: String) -> TaskExecutorResult<()> {
        self.states.execution.write().await.system_prompt = Some(SystemPrompt::Text(prompt));
        Ok(())
    }

    // Deprecated: system prompt is stored separately and not part of messages.
    pub async fn update_system_prompt(&self, new_system_prompt: String) -> TaskExecutorResult<()> {
        self.states.execution.write().await.system_prompt =
            Some(SystemPrompt::Text(new_system_prompt));
        Ok(())
    }

    pub async fn restore_messages(&self, messages: Vec<MessageParam>) -> TaskExecutorResult<()> {
        let mut exec = self.states.execution.write().await;
        exec.messages.clear();
        for msg in messages {
            exec.messages.push(msg);
        }
        // 不修改 runtime_status，保持当前状态
        Ok(())
    }

    async fn append_message(
        &self,
        role: MessageRole,
        content: &str,
        is_summary: bool,
    ) -> TaskExecutorResult<()> {
        let (iteration, seq) = {
            let mut exec = self.states.execution.write().await;
            let iteration = exec.record.current_iteration;
            let seq = exec.message_sequence;
            exec.message_sequence = seq.saturating_add(1);
            (iteration, seq)
        };

        self.agent_persistence()
            .execution_messages()
            .append_message(
                &self.task_id,
                role,
                content,
                i64::try_from(count_text_tokens(content)).unwrap_or(i64::MAX),
                is_summary,
                iteration,
                seq,
            )
            .await
            .map(|_| ())
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()).into())
    }

    pub async fn emit_event(&self, event: TaskEvent) -> TaskExecutorResult<()> {
        let channel_guard = self.states.progress_channel.lock().await;
        if let Some(channel) = channel_guard.as_ref() {
            channel.send(event).map_err(TaskExecutorError::ChannelError)?;
        }
        Ok(())
    }

    pub async fn initialize_message_track(
        &self,
        user_prompt: &str,
        images: Option<&[ImageAttachment]>,
    ) -> TaskExecutorResult<i64> {
        let mut user_blocks = Vec::new();

        if let Some(images) = images {
            user_blocks.extend(map_user_image_blocks(images));
        }
        user_blocks.push(Block::UserText(UserTextBlock {
            content: user_prompt.to_string(),
        }));

        let user_message = self
            .agent_persistence()
            .messages()
            .create(
                self.session_id,
                UiMessageRole::User,
                MessageStatus::Completed,
                user_blocks,
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        self.emit_event(TaskEvent::MessageCreated {
            message: user_message.clone(),
        })
        .await?;

        let assistant_message = self
            .agent_persistence()
            .messages()
            .create(
                self.session_id,
                UiMessageRole::Assistant,
                MessageStatus::Streaming,
                Vec::new(),
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        {
            let mut msg_state = self.states.messages.lock().await;
            msg_state.assistant_message = Some(assistant_message.clone());
        }

        self.emit_event(TaskEvent::MessageCreated {
            message: assistant_message,
        })
        .await?;

        Ok(user_message.id)
    }

    pub async fn assistant_append_block(&self, block: Block) -> TaskExecutorResult<()> {
        let mut message = self
            .states
            .messages
            .lock()
            .await
            .assistant_message
            .clone()
            .ok_or_else(|| {
                TaskExecutorError::StatePersistenceFailed(
                    "assistant message not initialized".to_string(),
                )
            })?;

        message.blocks.push(block.clone());
        let message_id = message.id;

        self.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        self.states.messages.lock().await.assistant_message = Some(message);

        self.emit_event(TaskEvent::BlockAppended { message_id, block })
            .await
    }

    pub async fn assistant_update_block(&self, block_id: &str, block: Block) -> TaskExecutorResult<()> {
        let mut message = self
            .states
            .messages
            .lock()
            .await
            .assistant_message
            .clone()
            .ok_or_else(|| {
                TaskExecutorError::StatePersistenceFailed(
                    "assistant message not initialized".to_string(),
                )
            })?;

        let Some(index) = find_block_index(&message.blocks, block_id) else {
            return Err(TaskExecutorError::StatePersistenceFailed(format!(
                "block {} not found for update",
                block_id
            ))
            .into());
        };

        message.blocks[index] = block.clone();
        let message_id = message.id;

        self.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        self.states.messages.lock().await.assistant_message = Some(message);

        self.emit_event(TaskEvent::BlockUpdated {
            message_id,
            block_id: block_id.to_string(),
            block,
        })
        .await
    }

    pub async fn finish_assistant_message(
        &self,
        status: MessageStatus,
        token_usage: Option<TokenUsage>,
    ) -> TaskExecutorResult<()> {
        let mut message = self
            .states
            .messages
            .lock()
            .await
            .assistant_message
            .clone()
            .ok_or_else(|| {
                TaskExecutorError::StatePersistenceFailed(
                    "assistant message not initialized".to_string(),
                )
            })?;

        let finished_at = Utc::now();
        let duration_ms = finished_at
            .signed_duration_since(message.created_at)
            .num_milliseconds()
            .max(0) as i64;

        message.status = status.clone();
        message.finished_at = Some(finished_at);
        message.duration_ms = Some(duration_ms);
        message.token_usage = token_usage.clone();

        self.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let message_id = message.id;
        self.states.messages.lock().await.assistant_message = Some(message);

        self.emit_event(TaskEvent::MessageFinished {
            message_id,
            status,
            finished_at,
            duration_ms,
            token_usage,
        })
        .await
    }

    pub async fn fail_assistant_message(&self, error: ErrorBlock) -> TaskExecutorResult<()> {
        self.assistant_append_block(Block::Error(error.clone()))
            .await?;
        self.finish_assistant_message(MessageStatus::Error, None).await?;
        Ok(())
    }

    pub async fn cancel_assistant_message(&self) -> TaskExecutorResult<()> {
        let Some(mut message) = self.states.messages.lock().await.assistant_message.clone() else {
            return Ok(());
        };

        let now = Utc::now();
        let mut changed_blocks: Vec<(String, Block)> = Vec::new();

        for block in &mut message.blocks {
            match block {
                Block::Thinking(b) => {
                    if b.is_streaming {
                        b.is_streaming = false;
                        changed_blocks.push((b.id.clone(), Block::Thinking(b.clone())));
                    }
                }
                Block::Text(b) => {
                    if b.is_streaming {
                        b.is_streaming = false;
                        changed_blocks.push((b.id.clone(), Block::Text(b.clone())));
                    }
                }
                Block::Tool(b) => {
                    if matches!(b.status, ToolStatus::Running) {
                        b.status = ToolStatus::Cancelled;
                        b.finished_at = Some(now);
                        b.duration_ms = Some(
                            now.signed_duration_since(b.started_at)
                                .num_milliseconds()
                                .max(0) as i64,
                        );
                        changed_blocks.push((b.id.clone(), Block::Tool(b.clone())));
                    }
                }
                _ => {}
            }
        }

        message.status = MessageStatus::Cancelled;
        message.finished_at = Some(now);
        message.duration_ms = Some(
            now.signed_duration_since(message.created_at)
                .num_milliseconds()
                .max(0) as i64,
        );

        self.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        self.states.messages.lock().await.assistant_message = Some(message.clone());

        for (block_id, block) in changed_blocks {
            let _ = self
                .emit_event(TaskEvent::BlockUpdated {
                    message_id: message.id,
                    block_id,
                    block,
                })
                .await;
        }

        self.emit_event(TaskEvent::MessageFinished {
            message_id: message.id,
            status: MessageStatus::Cancelled,
            finished_at: now,
            duration_ms: message.duration_ms.unwrap_or(0),
            token_usage: None,
        })
        .await?;
        Ok(())
    }
}

#[derive(Clone)]
struct ActiveCheckpoint {
    id: i64,
    workspace_root: PathBuf,
}

fn map_status(status: &AgentTaskStatus) -> TaskStatus {
    match status {
        AgentTaskStatus::Created => TaskStatus::Init,
        AgentTaskStatus::Running => TaskStatus::Running,
        AgentTaskStatus::Paused => TaskStatus::Paused,
        AgentTaskStatus::Completed => TaskStatus::Done,
        AgentTaskStatus::Error => TaskStatus::Error,
        AgentTaskStatus::Cancelled => TaskStatus::Aborted,
    }
}

fn render_message_content(content: &MessageContent) -> String {
    match content {
        MessageContent::Text(text) => text.clone(),
        MessageContent::Blocks(blocks) => serde_json::to_string(blocks).unwrap_or_default(),
    }
}

fn map_user_image_blocks(images: &[ImageAttachment]) -> Vec<Block> {
    images
        .iter()
        .enumerate()
        .map(|(index, attachment)| {
            let ext = attachment.mime_type.split('/').nth(1).unwrap_or("image");
            Block::UserImage(UserImageBlock {
                data_url: attachment.data_url.clone(),
                mime_type: attachment.mime_type.clone(),
                file_name: Some(format!("image_{}.{}", index, ext)),
                file_size: Some(attachment.data_url.len() as i64),
            })
        })
        .collect()
}

fn find_block_index(blocks: &[Block], block_id: &str) -> Option<usize> {
    blocks.iter().position(|block| match block {
        Block::Thinking(b) => b.id == block_id,
        Block::Text(b) => b.id == block_id,
        Block::Tool(b) => b.id == block_id,
        _ => false,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub call_id: String,
    pub tool_name: String,
    pub result: Value,
    pub status: crate::agent::tools::ToolResultStatus,
    pub execution_time_ms: u64,
}
