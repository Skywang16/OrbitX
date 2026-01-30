pub mod chain;
pub mod states;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::ipc::Channel;
use tokio::sync::Notify;
use tokio::sync::RwLock;

use self::chain::Chain;
use self::states::{ExecutionState, TaskStates};
use crate::agent::config::{AgentConfig, TaskExecutionConfig};
use crate::agent::context::FileContextTracker;
use crate::agent::core::executor::ImageAttachment;
use crate::agent::core::status::AgentTaskStatus;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::persistence::AgentPersistence;
use crate::agent::react::runtime::ReactRuntime;
use crate::agent::react::types::ReactRuntimeConfig;
use crate::agent::state::manager::{StateManager, TaskState, TaskStatus, TaskThresholds};
use crate::agent::state::session::SessionContext;
use crate::agent::tools::ToolRegistry;
use crate::agent::types::{
    Block, ErrorBlock, MessageRole as UiMessageRole, MessageStatus, TaskEvent, TokenUsage,
    SubtaskStatus, ToolStatus, UserImageBlock, UserTextBlock,
};
use crate::agent::workspace_changes::WorkspaceChangeJournal;
use crate::checkpoint::CheckpointService;
use crate::llm::anthropic_types::{
    ContentBlock, MessageContent, MessageParam, MessageRole as AnthropicRole, SystemPrompt,
    ToolResultContent,
};
use crate::storage::DatabaseManager;
use tokio_util::sync::CancellationToken;

#[async_trait::async_trait]
pub trait SubtaskRunner: Send + Sync {
    async fn run_subtask(
        &self,
        parent: &TaskContext,
        request: SubtaskRequest,
    ) -> TaskExecutorResult<SubtaskResponse>;
}

#[derive(Debug, Clone)]
pub struct SubtaskRequest {
    pub description: String,
    pub prompt: String,
    pub subagent_type: String,
    pub session_id: Option<i64>,
    pub call_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SubtaskResponse {
    pub session_id: i64,
    pub status: SubtaskStatus,
    pub summary: Option<String>,
}

pub struct TaskContextDeps {
    pub tool_registry: Arc<ToolRegistry>,
    pub repositories: Arc<DatabaseManager>,
    pub agent_persistence: Arc<AgentPersistence>,
    pub checkpoint_service: Option<Arc<CheckpointService>>,
    pub workspace_changes: Arc<WorkspaceChangeJournal>,
    pub subtask_runner: Arc<dyn SubtaskRunner>,
}

pub struct TaskContext {
    pub task_id: Arc<str>,
    pub session_id: i64,
    pub user_prompt: Arc<str>,
    pub agent_type: Arc<str>,
    pub cwd: Arc<str>,
    emit_task_events: bool,
    config: TaskExecutionConfig,

    session: Arc<SessionContext>,
    tool_registry: Arc<ToolRegistry>,
    subtask_runner: Arc<dyn SubtaskRunner>,
    state_manager: Arc<StateManager>,
    checkpoint_service: Option<Arc<CheckpointService>>,
    active_checkpoint: Arc<RwLock<Option<ActiveCheckpoint>>>,
    workspace_changes: Arc<WorkspaceChangeJournal>,
    workspace_key: Arc<str>,

    pub(crate) states: TaskStates,

    pause_status: AtomicU8,
    pause_notify: Arc<Notify>,
}

impl TaskContext {
    /// Construct a fresh context for a new task.
    pub async fn new(
        task_id: String,
        session_id: i64,
        user_prompt: String,
        agent_type: String,
        config: TaskExecutionConfig,
        workspace_path: String,
        emit_task_events: bool,
        progress_channel: Option<Channel<TaskEvent>>,
        deps: TaskContextDeps,
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

        let task_status = AgentTaskStatus::Created;
        let current_iteration = 0u32;
        let error_count = 0u32;

        let mut task_state = TaskState::new(task_id.clone(), thresholds);
        task_state.iterations = current_iteration;
        task_state.consecutive_errors = error_count;
        task_state.task_status = map_status(&task_status);

        let normalized_workspace = workspace_path;
        let workspace_root = PathBuf::from(&normalized_workspace);
        let workspace_key: Arc<str> = Arc::from(normalized_workspace.as_str());

        let session = Arc::new(SessionContext::new(
            task_id.clone(),
            session_id,
            workspace_root.clone(),
            user_prompt.clone(),
            config,
            Arc::clone(&deps.repositories),
            Arc::clone(&deps.agent_persistence),
        ));

        let execution = ExecutionState::new(task_status);
        let react_runtime = ReactRuntime::new(runtime_config);

        let states = TaskStates::new(execution, react_runtime, progress_channel);

        Ok(Self {
            task_id: Arc::from(task_id.as_str()),
            session_id,
            user_prompt: Arc::from(user_prompt.as_str()),
            agent_type: Arc::from(agent_type.as_str()),
            cwd: Arc::from(normalized_workspace.as_str()),
            emit_task_events,
            config,
            session,
            tool_registry: deps.tool_registry,
            subtask_runner: deps.subtask_runner,
            state_manager: Arc::new(StateManager::new(task_state)),
            checkpoint_service: deps.checkpoint_service,
            active_checkpoint: Arc::new(RwLock::new(None)),
            workspace_changes: deps.workspace_changes,
            workspace_key,
            states,
            pause_status: AtomicU8::new(0),
            pause_notify: Arc::new(Notify::new()),
        })
    }

    pub fn emits_task_events(&self) -> bool {
        self.emit_task_events
    }

    pub async fn note_agent_write_intent(&self, path: &Path) {
        self.workspace_changes
            .begin_agent_write(Arc::clone(&self.workspace_key), path.to_path_buf())
            .await;
    }

    pub async fn note_agent_read_snapshot(&self, path: &Path, content: &str) {
        self.workspace_changes
            .update_snapshot_from_read(Arc::clone(&self.workspace_key), path.to_path_buf(), content)
            .await;
    }

    pub async fn set_progress_channel(&self, channel: Option<Channel<TaskEvent>>) {
        *self.states.progress_channel.lock().await = channel;
    }

    pub async fn progress_channel(&self) -> Option<Channel<TaskEvent>> {
        self.states.progress_channel.lock().await.clone()
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

    pub async fn active_checkpoint_handle(&self) -> Option<(i64, PathBuf)> {
        self.active_checkpoint
            .read()
            .await
            .as_ref()
            .map(|cp| (cp.id, cp.workspace_root.clone()))
    }

    pub async fn inherit_checkpoint(&self, checkpoint_id: i64, workspace_root: PathBuf) {
        let mut guard = self.active_checkpoint.write().await;
        *guard = Some(ActiveCheckpoint {
            id: checkpoint_id,
            workspace_root,
        });
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

    pub fn subtask_runner(&self) -> &dyn SubtaskRunner {
        self.subtask_runner.as_ref()
    }

    pub async fn status(&self) -> AgentTaskStatus {
        self.states.execution.read().await.runtime_status
    }

    pub async fn set_status(&self, status: AgentTaskStatus) -> TaskExecutorResult<()> {
        let session_status = {
            let mut exec = self.states.execution.write().await;
            exec.runtime_status = status;
            match status {
                AgentTaskStatus::Created | AgentTaskStatus::Paused => "idle",
                AgentTaskStatus::Running => "running",
                AgentTaskStatus::Completed => "completed",
                AgentTaskStatus::Error => "error",
                AgentTaskStatus::Cancelled => "cancelled",
            }
        };

        self.agent_persistence()
            .sessions()
            .update_status(self.session_id, session_status)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        self.state_manager
            .update_task_status(map_status(&status), None)
            .await;
        Ok(())
    }

    /// Increment iteration counter and sync to storage.
    pub async fn increment_iteration(&self) -> TaskExecutorResult<u32> {
        let current = {
            let mut exec = self.states.execution.write().await;
            exec.current_iteration = exec.current_iteration.saturating_add(1);
            exec.message_sequence = 0;
            exec.current_iteration
        };

        self.state_manager.increment_iteration().await;
        Ok(current)
    }

    /// Current iteration number.
    pub async fn current_iteration(&self) -> u32 {
        self.states.execution.read().await.current_iteration
    }

    /// Increase error counter and persist.
    pub async fn increment_error_count(&self) -> TaskExecutorResult<u32> {
        let count = {
            let mut exec = self.states.execution.write().await;
            exec.error_count = exec.error_count.saturating_add(1);
            exec.error_count
        };
        self.state_manager.increment_error_count().await;
        Ok(count)
    }

    pub async fn reset_error_count(&self) -> TaskExecutorResult<()> {
        {
            let mut exec = self.states.execution.write().await;
            exec.error_count = 0;
        };
        self.state_manager.reset_error_count().await;
        Ok(())
    }

    /// Determine if execution should stop based on status and thresholds.
    pub async fn should_stop(&self) -> bool {
        let (status, iteration, errors) = {
            let exec = self.states.execution.read().await;
            (
                exec.runtime_status,
                exec.current_iteration,
                exec.error_count,
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

    pub async fn with_chain_mut<T>(&self, f: impl FnOnce(&mut Chain) -> T) -> T {
        let mut chain = self.states.chain.write().await;
        f(&mut chain)
    }

    /// 检查任务是否被中止
    /// 简化版：只检查 aborted 标志，无需锁
    pub fn check_aborted(&self, no_check_pause: bool) -> TaskExecutorResult<()> {
        if self.states.aborted.load(Ordering::SeqCst) {
            return Err(TaskExecutorError::TaskInterrupted);
        }
        if no_check_pause {
            return Ok(());
        }
        // 暂停检查保持同步方式
        let status = self.pause_status.load(Ordering::SeqCst);
        if status != 0 {
            // 如果暂停，返回错误让调用者处理
            return Err(TaskExecutorError::TaskInterrupted);
        }
        Ok(())
    }

    /// 异步检查任务是否被中止（带暂停等待）
    pub async fn check_aborted_async(&self, no_check_pause: bool) -> TaskExecutorResult<()> {
        if self.states.aborted.load(Ordering::SeqCst) || self.states.abort_token.is_cancelled() {
            return Err(TaskExecutorError::TaskInterrupted);
        }
        if no_check_pause {
            return Ok(());
        }
        while self.pause_status.load(Ordering::SeqCst) != 0 {
            tokio::select! {
                _ = self.states.abort_token.cancelled() => {
                    return Err(TaskExecutorError::TaskInterrupted);
                }
                _ = self.pause_notify.notified() => {}
            }
        }
        Ok(())
    }

    /// 中止任务执行
    /// 简化版：只需设置 aborted 标志
    pub fn abort(&self) {
        self.states.aborted.store(true, Ordering::SeqCst);
        self.states.abort_token.cancel();

        // 标记 react 运行时为中止状态
        let react_runtime = Arc::clone(&self.states.react_runtime);
        tokio::spawn(async move {
            let mut react = react_runtime.write().await;
            react.mark_abort();
        });
    }

    /// 检查是否已中止
    pub fn is_aborted(&self) -> bool {
        self.states.aborted.load(Ordering::SeqCst) || self.states.abort_token.is_cancelled()
    }

    pub fn set_pause(&self, paused: bool, _abort_current_step: bool) {
        let new_status = if paused { 1 } else { 0 };
        self.pause_status.store(new_status, Ordering::SeqCst);
        self.pause_notify.notify_waiters();
    }

    /// 为 LLM 流创建取消令牌
    /// 这个 token 会在任务 aborted 时自动取消
    pub fn create_stream_cancel_token(&self) -> CancellationToken {
        self.states.abort_token.child_token()
    }

    /// Add assistant message using Anthropic-native types (text and/or tool uses).
    pub async fn add_assistant_message(
        &self,
        text: Option<String>,
        tool_calls: Option<Vec<ContentBlock>>,
    ) -> TaskExecutorResult<()> {
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
        Ok(())
    }

    /// Append tool results as a user message with ToolResult blocks; also persist tool rows.
    pub async fn add_tool_results(&self, results: Vec<ToolCallResult>) -> TaskExecutorResult<()> {
        let blocks: Vec<ContentBlock> = results
            .iter()
            .map(|r| {
                // r.result is now a JSON Value that contains the actual result content
                // It could be a string, number, object, or array - convert to appropriate string representation
                let result_text = if r.result.is_string() {
                    r.result.as_str().unwrap_or("").to_string()
                } else {
                    // For non-string values, serialize to JSON string
                    serde_json::to_string(&r.result).unwrap_or_else(|_| r.result.to_string())
                };

                ContentBlock::ToolResult {
                    tool_use_id: r.call_id.clone(),
                    content: Some(ToolResultContent::Text(result_text)),
                    is_error: Some(r.status != crate::agent::tools::ToolResultStatus::Success),
                }
            })
            .collect();

        {
            let mut exec = self.states.execution.write().await;
            exec.tool_results.extend(results);
            exec.messages.push(MessageParam {
                role: AnthropicRole::User,
                content: MessageContent::Blocks(blocks),
            });
        }
        Ok(())
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
            exec.system_prompt_overlay = None;
            exec.messages.clear();
            exec.message_sequence = 0;
        }
        self.add_user_message(user_prompt).await?;
        Ok(())
    }

    pub async fn get_messages(&self) -> Vec<MessageParam> {
        self.states.execution.read().await.messages_vec()
    }

    pub async fn get_system_prompt(&self) -> Option<SystemPrompt> {
        let exec = self.states.execution.read().await;
        match (&exec.system_prompt, &exec.system_prompt_overlay) {
            (None, None) => None,
            (Some(base), None) => Some(base.clone()),
            (None, Some(overlay)) => Some(overlay.clone()),
            (Some(base), Some(overlay)) => Some(merge_system_prompts(base, overlay)),
        }
    }

    pub async fn add_user_message(&self, text: String) -> TaskExecutorResult<()> {
        self.add_user_message_with_images(text, None).await
    }

    pub async fn add_user_message_with_images(
        &self,
        text: String,
        images: Option<&[ImageAttachment]>,
    ) -> TaskExecutorResult<()> {
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
        Ok(())
    }

    /// Add a user message with optional images and system reminders.
    /// System reminders are wrapped in <system-reminder> tags and prepended to the user message.
    /// They are only sent to the LLM, not persisted to UI messages.
    pub async fn add_user_message_with_reminders(
        &self,
        text: String,
        images: Option<&[ImageAttachment]>,
        system_reminders: &[String],
    ) -> TaskExecutorResult<()> {
        // Build the final text with system reminders prepended
        let final_text = if system_reminders.is_empty() {
            text
        } else {
            let reminder_block = system_reminders
                .iter()
                .map(|r| format!("<system-reminder>\n{}\n</system-reminder>", r.trim()))
                .collect::<Vec<_>>()
                .join("\n\n");
            format!("{}\n\n{}", reminder_block, text)
        };

        self.add_user_message_with_images(final_text, images).await
    }

    pub async fn reset_message_state(&self) -> TaskExecutorResult<()> {
        {
            let mut exec = self.states.execution.write().await;
            exec.messages.clear();
            exec.message_sequence = 0;
        }
        Ok(())
    }

    /// Set system prompt in memory only; do not persist system message to DB.
    pub async fn set_system_prompt(&self, prompt: String) -> TaskExecutorResult<()> {
        let mut exec = self.states.execution.write().await;
        exec.system_prompt = Some(SystemPrompt::Text(prompt));
        exec.system_prompt_overlay = None;
        Ok(())
    }

    // Deprecated: system prompt is stored separately and not part of messages.
    pub async fn update_system_prompt(&self, new_system_prompt: String) -> TaskExecutorResult<()> {
        let mut exec = self.states.execution.write().await;
        exec.system_prompt = Some(SystemPrompt::Text(new_system_prompt));
        Ok(())
    }

    /// A transient overlay appended to the base system prompt (e.g. loop warnings).
    pub async fn set_system_prompt_overlay(
        &self,
        overlay: Option<SystemPrompt>,
    ) -> TaskExecutorResult<()> {
        self.states.execution.write().await.system_prompt_overlay = overlay;
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

    pub async fn emit_event(&self, event: TaskEvent) -> TaskExecutorResult<()> {
        let channel_guard = self.states.progress_channel.lock().await;
        if let Some(channel) = channel_guard.as_ref() {
            channel
                .send(event)
                .map_err(TaskExecutorError::ChannelError)?;
        }
        Ok(())
    }

    pub async fn initialize_message_track(
        &self,
        user_prompt: &str,
        images: Option<&[ImageAttachment]>,
        internal_user_message: bool,
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
                false,
                internal_user_message,
                self.agent_type.as_ref(),
                None,
                None,
                None,
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        self.emit_event(TaskEvent::MessageCreated {
            task_id: self.task_id.to_string(),
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
                false,
                false,
                self.agent_type.as_ref(),
                Some(user_message.id),
                None,
                None,
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        {
            let mut msg_state = self.states.messages.lock().await;
            msg_state.assistant_message = Some(assistant_message.clone());
        }

        self.emit_event(TaskEvent::MessageCreated {
            task_id: self.task_id.to_string(),
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

        self.emit_event(TaskEvent::BlockAppended {
            task_id: self.task_id.to_string(),
            message_id,
            block,
        })
            .await
    }

    pub async fn active_assistant_message_id(&self) -> Option<i64> {
        self.states
            .messages
            .lock()
            .await
            .assistant_message
            .as_ref()
            .map(|m| m.id)
    }

    pub async fn assistant_update_block(
        &self,
        block_id: &str,
        block: Block,
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

        let Some(index) = find_block_index(&message.blocks, block_id) else {
            return Err(TaskExecutorError::StatePersistenceFailed(format!(
                "block {block_id} not found for update"
            )));
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
            task_id: self.task_id.to_string(),
            message_id,
            block_id: block_id.to_string(),
            block,
        })
        .await
    }

    pub async fn assistant_upsert_block(&self, block: Block) -> TaskExecutorResult<()> {
        let block_id = match &block {
            Block::Thinking(b) => b.id.clone(),
            Block::Text(b) => b.id.clone(),
            Block::Tool(b) => b.id.clone(),
            Block::Subtask(b) => b.id.clone(),
            _ => {
                return Err(TaskExecutorError::StatePersistenceFailed(
                    "block type does not support upsert".to_string(),
                ))
            }
        };

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

        let message_id = message.id;
        let index_opt = find_block_index(&message.blocks, &block_id);
        let existed = index_opt.is_some();
        if let Some(index) = index_opt {
            message.blocks[index] = block.clone();
        } else {
            message.blocks.push(block.clone());
        }

        self.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        self.states.messages.lock().await.assistant_message = Some(message);

        if existed {
            self.emit_event(TaskEvent::BlockUpdated {
                task_id: self.task_id.to_string(),
                message_id,
                block_id,
                block,
            })
            .await
        } else {
            self.emit_event(TaskEvent::BlockAppended {
                task_id: self.task_id.to_string(),
                message_id,
                block,
            })
            .await
        }
    }

    /// 计算当前会话的上下文占用
    pub async fn calculate_context_usage(
        &self,
        model_id: &str,
    ) -> Option<crate::agent::types::ContextUsage> {
        use crate::agent::utils::tokenizer::{count_text_tokens, count_message_param_tokens};

        // 获取模型的上下文窗口大小
        let context_window = match model_id {
            // Claude models
            s if s.contains("claude-3-5-sonnet") => 200_000,
            s if s.contains("claude-3-5-haiku") => 200_000,
            s if s.contains("claude-3-opus") => 200_000,
            s if s.contains("claude-3-sonnet") => 200_000,
            s if s.contains("claude-3-haiku") => 200_000,
            // GPT models
            s if s.contains("gpt-4") => 128_000,
            s if s.contains("gpt-3.5-turbo") => 16_384,
            // 默认值
            _ => 128_000,
        };

        let mut tokens_used = 0usize;

        let execution = self.states.execution.read().await;

        // 统计系统提示
        if let Some(system_prompt) = &execution.system_prompt {
            match system_prompt {
                SystemPrompt::Text(t) => {
                    tokens_used += count_text_tokens(t);
                }
                SystemPrompt::Blocks(blocks) => {
                    for block in blocks {
                        tokens_used += count_text_tokens(&block.text);
                    }
                }
            }
        }

        // 统计对话历史
        for msg in &execution.messages {
            tokens_used += count_message_param_tokens(msg);
        }

        // 确保不超过上下文窗口
        let tokens_used = (tokens_used as u32).min(context_window);

        Some(crate::agent::types::ContextUsage {
            tokens_used,
            context_window,
        })
    }

    pub async fn finish_assistant_message(
        &self,
        status: MessageStatus,
        token_usage: Option<TokenUsage>,
        context_usage: Option<crate::agent::types::ContextUsage>,
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
            .max(0);

        message.status = status.clone();
        message.finished_at = Some(finished_at);
        message.duration_ms = Some(duration_ms);
        message.token_usage = token_usage.clone();
        message.context_usage = context_usage.clone();

        self.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let message_id = message.id;
        self.states.messages.lock().await.assistant_message = Some(message);

        self.emit_event(TaskEvent::MessageFinished {
            task_id: self.task_id.to_string(),
            message_id,
            status,
            finished_at,
            duration_ms,
            token_usage,
            context_usage,
        })
        .await
    }

    pub async fn fail_assistant_message(&self, error: ErrorBlock) -> TaskExecutorResult<()> {
        let Some(mut message) = self.states.messages.lock().await.assistant_message.clone() else {
            return Ok(());
        };

        let now = Utc::now();
        let mut changed_blocks: Vec<(String, Block)> = Vec::new();

        let appended = Block::Error(error);
        message.blocks.push(appended.clone());

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
                    if matches!(b.status, ToolStatus::Running | ToolStatus::Pending) {
                        b.status = ToolStatus::Error;
                        b.finished_at = Some(now);
                        b.duration_ms = Some(
                            now.signed_duration_since(b.started_at)
                                .num_milliseconds()
                                .max(0),
                        );
                        changed_blocks.push((b.id.clone(), Block::Tool(b.clone())));
                    }
                }
                Block::Subtask(b) => {
                    if matches!(b.status, SubtaskStatus::Running | SubtaskStatus::Pending) {
                        b.status = SubtaskStatus::Error;
                        if b.summary.is_none() {
                            b.summary = Some("Parent task failed".to_string());
                        }
                        changed_blocks.push((b.id.clone(), Block::Subtask(b.clone())));
                    }
                }
                _ => {}
            }
        }

        message.status = MessageStatus::Error;
        message.finished_at = Some(now);
        message.duration_ms = Some(
            now.signed_duration_since(message.created_at)
                .num_milliseconds()
                .max(0),
        );

        self.agent_persistence()
            .messages()
            .update(&message)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let message_id = message.id;
        self.states.messages.lock().await.assistant_message = Some(message.clone());

        self.emit_event(TaskEvent::BlockAppended {
            task_id: self.task_id.to_string(),
            message_id,
            block: appended,
        })
        .await?;

        for (block_id, block) in changed_blocks {
            let _ = self
                .emit_event(TaskEvent::BlockUpdated {
                    task_id: self.task_id.to_string(),
                    message_id,
                    block_id,
                    block,
                })
                .await;
        }

        self.emit_event(TaskEvent::MessageFinished {
            task_id: self.task_id.to_string(),
            message_id,
            status: MessageStatus::Error,
            finished_at: now,
            duration_ms: message.duration_ms.unwrap_or(0),
            token_usage: None,
            context_usage: None,
        })
        .await?;
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
                    if matches!(b.status, ToolStatus::Running | ToolStatus::Pending) {
                        b.status = ToolStatus::Cancelled;
                        b.finished_at = Some(now);
                        b.duration_ms = Some(
                            now.signed_duration_since(b.started_at)
                                .num_milliseconds()
                                .max(0),
                        );
                        changed_blocks.push((b.id.clone(), Block::Tool(b.clone())));
                    }
                }
                Block::Subtask(b) => {
                    if matches!(b.status, SubtaskStatus::Running | SubtaskStatus::Pending) {
                        b.status = SubtaskStatus::Cancelled;
                        if b.summary.is_none() {
                            b.summary = Some("Parent task cancelled".to_string());
                        }
                        changed_blocks.push((b.id.clone(), Block::Subtask(b.clone())));
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
                .max(0),
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
                    task_id: self.task_id.to_string(),
                    message_id: message.id,
                    block_id,
                    block,
                })
                .await;
        }

        self.emit_event(TaskEvent::MessageFinished {
            task_id: self.task_id.to_string(),
            message_id: message.id,
            status: MessageStatus::Cancelled,
            finished_at: now,
            duration_ms: message.duration_ms.unwrap_or(0),
            token_usage: None,
            context_usage: None,
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

fn map_user_image_blocks(images: &[ImageAttachment]) -> Vec<Block> {
    images
        .iter()
        .enumerate()
        .map(|(index, attachment)| {
            let ext = attachment.mime_type.split('/').nth(1).unwrap_or("image");
            Block::UserImage(UserImageBlock {
                data_url: attachment.data_url.clone(),
                mime_type: attachment.mime_type.clone(),
                file_name: Some(format!("image_{index}.{ext}")),
                file_size: Some(attachment.data_url.len() as i64),
            })
        })
        .collect()
}

fn merge_system_prompts(
    base: &crate::llm::anthropic_types::SystemPrompt,
    overlay: &crate::llm::anthropic_types::SystemPrompt,
) -> crate::llm::anthropic_types::SystemPrompt {
    use crate::llm::anthropic_types::{SystemBlock, SystemPrompt};

    match (base, overlay) {
        (SystemPrompt::Text(a), SystemPrompt::Text(b)) => {
            SystemPrompt::Text(format!("{}\n\n{}", a.trim_end(), b.trim_start()))
        }
        (SystemPrompt::Blocks(a), SystemPrompt::Blocks(b)) => {
            let mut out = a.clone();
            out.extend(b.iter().cloned());
            SystemPrompt::Blocks(out)
        }
        (SystemPrompt::Text(a), SystemPrompt::Blocks(b)) => {
            let mut out = vec![SystemBlock {
                block_type: "text".to_string(),
                text: a.clone(),
                cache_control: None,
            }];
            out.extend(b.iter().cloned());
            SystemPrompt::Blocks(out)
        }
        (SystemPrompt::Blocks(a), SystemPrompt::Text(b)) => {
            let mut out = a.clone();
            out.push(SystemBlock {
                block_type: "text".to_string(),
                text: b.clone(),
                cache_control: None,
            });
            SystemPrompt::Blocks(out)
        }
    }
}

fn find_block_index(blocks: &[Block], block_id: &str) -> Option<usize> {
    blocks.iter().position(|block| match block {
        Block::Thinking(b) => b.id == block_id,
        Block::Text(b) => b.id == block_id,
        Block::Tool(b) => b.id == block_id,
        Block::Subtask(b) => b.id == block_id,
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
