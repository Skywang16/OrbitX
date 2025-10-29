/*!
 * TaskExecutor - 核心任务执行器（已迁移至 agent/core）
 *
 * 负责Agent任务的完整执行流程：
 * - ReAct循环管理
 * - LLM调用和响应解析
 * - 工具执行调度
 * - 状态持久化
 * - 并发任务管理
 */

use crate::agent::config::CompactionConfig;
use crate::agent::config::TaskExecutionConfig;
use crate::agent::context::{
    ContextBuilder, ConversationSummarizer, ProjectContextLoader, SummaryResult,
};
use crate::agent::core::context::{TaskContext, ToolCallResult};
use crate::agent::core::iteration_outcome::IterationOutcome;
use crate::agent::core::status::AgentTaskStatus;
use crate::agent::error::AgentError;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::events::{
    TaskCancelledPayload, TaskCompletedPayload, TaskCreatedPayload, TaskErrorPayload,
    TaskPausedPayload, TaskProgressPayload, TaskStartedPayload, TextPayload, ThinkingPayload,
};
use crate::agent::memory::compactor::{CompactionResult, MessageCompactor};
use crate::agent::persistence::{
    AgentExecution, AgentPersistence, Conversation, ConversationSummary, ExecutionEvent,
    ExecutionEventType, ExecutionMessage, FileContextEntry, ToolExecution,
};
use crate::agent::prompt::{build_agent_system_prompt, build_agent_user_prompt};
use crate::agent::state::iteration::IterationSnapshot;
use crate::agent::state::session::CompressedMemory;
use crate::agent::tools::{
    logger::ToolExecutionLogger, ToolDescriptionContext, ToolRegistry, ToolResult as ToolOutcome,
    ToolResultContent,
};
use crate::agent::types::{Agent, Context as AgentContext, Task, ToolSchema};
use crate::agent::ui::AgentUiPersistence;
use crate::llm::anthropic_types::{
    ContentBlock, ContentBlockStart, ContentDelta, MessageContent, MessageParam, StreamEvent,
    SystemPrompt, ToolResultContent as AnthropicToolResultContent,
};
use crate::storage::DatabaseManager;
use crate::terminal::TerminalContextService;
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::ipc::Channel;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// 任务执行参数（与前端风格统一 camelCase）
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteTaskParams {
    pub conversation_id: i64,
    pub user_prompt: String,
    pub chat_mode: String,
    pub model_id: String,
    pub config_overrides: Option<serde_json::Value>,
}

/// 内容块累积器（用于流式组装）
enum BlockAccumulator {
    Text(String),
    ToolUse {
        id: String,
        name: String,
        input_json: String,
    },
    Thinking(String),
}

/// 任务摘要信息
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    pub task_id: String,
    pub conversation_id: i64,
    pub status: String,
    pub current_iteration: u32,
    pub error_count: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

const CONTEXT_EXECUTION_LIMIT: usize = 5;
const CONTEXT_MESSAGE_LIMIT: usize = 50;
const CONTEXT_TOOL_LIMIT: usize = 20;
const CONTEXT_EVENT_LIMIT: usize = 100;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionSnapshot {
    pub execution: AgentExecution,
    pub messages: Vec<ExecutionMessage>,
    pub tool_calls: Vec<ToolExecution>,
    pub events: Vec<ExecutionEvent>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationContextSnapshot {
    pub conversation: Conversation,
    pub summary: Option<ConversationSummary>,
    pub active_task_ids: Vec<String>,
    pub executions: Vec<ExecutionSnapshot>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContextStatus {
    pub conversation_id: i64,
    pub active_files: Vec<FileContextEntry>,
    pub stale_files: Vec<FileContextEntry>,
    pub total_active: usize,
    pub total_stale: usize,
}

/// TaskExecutor核心结构体
struct TaskExecutorInner {
    database: Arc<DatabaseManager>,
    agent_persistence: Arc<AgentPersistence>,
    ui_persistence: Arc<AgentUiPersistence>,
    tool_logger: Arc<ToolExecutionLogger>,
    context_builder_cache: Arc<DashMap<i64, Arc<ContextBuilder>>>,
    active_tasks: Arc<DashMap<String, Arc<TaskContext>>>,
    conversation_contexts: Arc<DashMap<i64, Arc<TaskContext>>>,
    default_config: TaskExecutionConfig,
    terminal_context_service: Arc<TerminalContextService>,
    cache: Arc<crate::storage::UnifiedCache>,
}

#[derive(Clone)]
pub struct TaskExecutor(Arc<TaskExecutorInner>);

impl TaskExecutor {
    pub fn new(
        database: Arc<DatabaseManager>,
        agent_persistence: Arc<AgentPersistence>,
        ui_persistence: Arc<AgentUiPersistence>,
        terminal_context_service: Arc<TerminalContextService>,
        cache: Arc<crate::storage::UnifiedCache>,
    ) -> Self {
        let tool_logger = Arc::new(ToolExecutionLogger::new(
            database.clone(),
            agent_persistence.clone(),
            true,
        ));

        let inner = TaskExecutorInner {
            database,
            agent_persistence,
            ui_persistence,
            tool_logger,
            context_builder_cache: Arc::new(DashMap::new()),
            active_tasks: Arc::new(DashMap::new()),
            conversation_contexts: Arc::new(DashMap::new()),
            default_config: TaskExecutionConfig::default(),
            terminal_context_service,
            cache,
        };

        Self(Arc::new(inner))
    }

    fn inner(&self) -> &TaskExecutorInner {
        self.0.as_ref()
    }

    pub(crate) fn database(&self) -> Arc<DatabaseManager> {
        Arc::clone(&self.inner().database)
    }

    pub(crate) fn tool_logger(&self) -> Arc<ToolExecutionLogger> {
        Arc::clone(&self.inner().tool_logger)
    }

    fn context_builder_cache(&self) -> &DashMap<i64, Arc<ContextBuilder>> {
        &self.inner().context_builder_cache
    }

    fn active_tasks(&self) -> &DashMap<String, Arc<TaskContext>> {
        &self.inner().active_tasks
    }

    fn conversation_contexts(&self) -> &DashMap<i64, Arc<TaskContext>> {
        &self.inner().conversation_contexts
    }

    fn default_config(&self) -> TaskExecutionConfig {
        self.inner().default_config.clone()
    }

    fn terminal_context_service(&self) -> Arc<TerminalContextService> {
        Arc::clone(&self.inner().terminal_context_service)
    }

    /// 执行任务（主入口）
    pub async fn execute_task(
        &self,
        params: ExecuteTaskParams,
        progress_channel: Channel<TaskProgressPayload>,
    ) -> TaskExecutorResult<()> {
        info!(
            "[MODEL_SELECTION] execute_task called with conversation_id: {}, model_id: {:?}",
            params.conversation_id, params.model_id
        );

        // 1. Check memory cache first
        if let Some(existing_ctx) = self.conversation_context(params.conversation_id).await {
            info!(
                "[MODEL_SELECTION] Found existing context for conversation {}, using model_id: {}",
                params.conversation_id, params.model_id
            );

            let status = existing_ctx.status().await;
            let is_actually_running =
                matches!(status, AgentTaskStatus::Running | AgentTaskStatus::Paused);

            if is_actually_running {
                if self.active_tasks().contains_key(&existing_ctx.task_id) {
                    return Err(TaskExecutorError::InternalError(format!(
                        "Conversation {} still has active task, cannot start new task",
                        params.conversation_id
                    ))
                    .into());
                }
            }

            existing_ctx
                .set_progress_channel(Some(progress_channel))
                .await;
            existing_ctx.reset_cancellation().await;

            // Update SystemPrompt with latest environment info
            let (system_prompt, _) = self
                .build_task_prompts(
                    params.conversation_id,
                    existing_ctx.task_id.clone(),
                    &params.user_prompt,
                    Some(&existing_ctx.cwd),
                    &*existing_ctx.tool_registry(),
                )
                .await?;
            existing_ctx.update_system_prompt(system_prompt).await?;

            existing_ctx
                .begin_followup_turn(&params.user_prompt)
                .await?;
            existing_ctx
                .add_user_message(params.user_prompt.clone())
                .await;

            existing_ctx
                .agent_persistence()
                .conversations()
                .touch(existing_ctx.conversation_id)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

            existing_ctx.set_status(AgentTaskStatus::Running).await?;

            self.active_tasks().insert(
                existing_ctx.task_id.clone(),
                existing_ctx.clone(),
            );

            existing_ctx
                .send_progress(TaskProgressPayload::TaskStarted(TaskStartedPayload {
                    task_id: existing_ctx.task_id.clone(),
                    iteration: existing_ctx.current_iteration().await,
                }))
                .await?;

            self.spawn_react_execution(existing_ctx, params.model_id);
            return Ok(());
        }

        // 2. Try to restore from database (after app restart)
        if let Ok(Some(restored_ctx)) = self.restore_from_db(params.conversation_id).await {
            self.register_conversation_context(restored_ctx.clone())
                .await;

            restored_ctx
                .set_progress_channel(Some(progress_channel))
                .await;
            restored_ctx.reset_cancellation().await;

            restored_ctx
                .begin_followup_turn(&params.user_prompt)
                .await?;
            restored_ctx
                .add_user_message(params.user_prompt.clone())
                .await;

            restored_ctx
                .agent_persistence()
                .conversations()
                .touch(restored_ctx.conversation_id)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

            restored_ctx.set_status(AgentTaskStatus::Running).await?;

            self.active_tasks().insert(
                restored_ctx.task_id.clone(),
                restored_ctx.clone(),
            );

            restored_ctx
                .send_progress(TaskProgressPayload::TaskStarted(TaskStartedPayload {
                    task_id: restored_ctx.task_id.clone(),
                    iteration: restored_ctx.current_iteration().await,
                }))
                .await?;

            self.spawn_react_execution(restored_ctx, params.model_id);
            return Ok(());
        }

        // 3. Create new task
        let model_id = params.model_id.clone();
        let context = Arc::new(
            self.create_task_context(params, Some(progress_channel), None)
                .await?,
        );
        self.register_conversation_context(context.clone()).await;

        let persistence = self.agent_persistence();
        persistence
            .agent_executions()
            .mark_started(&context.task_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        context.set_status(AgentTaskStatus::Running).await?;

        self.active_tasks().insert(
            context.task_id.clone(),
            context.clone(),
        );

        context
            .send_progress(TaskProgressPayload::TaskStarted(TaskStartedPayload {
                task_id: context.task_id.clone(),
                iteration: context.current_iteration().await,
            }))
            .await?;

        self.spawn_react_execution(context, model_id);
        Ok(())
    }

    /// 暂停任务
    pub async fn pause_task(&self, task_id: &str) -> TaskExecutorResult<()> {
        if let Some(context) = self.active_tasks().get(task_id) {
            context.set_status(AgentTaskStatus::Paused).await?;
            context.set_pause(true, false);

            context
                .send_progress(TaskProgressPayload::TaskPaused(TaskPausedPayload {
                    task_id: task_id.to_string(),
                    reason: "User requested pause".to_string(),
                    timestamp: Utc::now(),
                }))
                .await?;
        } else {
            return Err(TaskExecutorError::TaskNotFound(task_id.to_string()).into());
        }

        Ok(())
    }

    /// 取消任务
    pub async fn cancel_task(
        &self,
        task_id: &str,
        reason: Option<String>,
    ) -> TaskExecutorResult<()> {
        let context = self.active_tasks()
            .get(task_id)
            .ok_or_else(|| TaskExecutorError::TaskNotFound(task_id.to_string()))?;
        let context = context.value().clone();

        context.set_pause(false, true);
        context.abort();
        context.set_status(AgentTaskStatus::Cancelled).await?;

        context
            .send_progress(TaskProgressPayload::TaskCancelled(TaskCancelledPayload {
                task_id: task_id.to_string(),
                reason: reason.unwrap_or_else(|| "User cancelled".to_string()),
                timestamp: Utc::now(),
            }))
            .await?;

        // ✅ 只删除 active_tasks，保留 conversation_contexts
        self.active_tasks().remove(task_id);

        // ❌ 不清理 context_builder，保留对话上下文用于后续继续
        // self.clear_context_builder(conversation_id).await;

        info!(
            "Task {} cancelled, conversation context preserved for continuation",
            task_id
        );
        Ok(())
    }

    /// 列出任务
    pub async fn list_tasks(
        &self,
        conversation_id: Option<i64>,
        status_filter: Option<String>,
    ) -> TaskExecutorResult<Vec<TaskSummary>> {
        let persistence = self.agent_persistence();
        let executions = if let Some(conv_id) = conversation_id {
            persistence
                .agent_executions()
                .list_recent_by_conversation(conv_id, 50)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?
        } else {
            persistence
                .agent_executions()
                .list_recent(50)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?
        };

        let mut summaries = Vec::new();
        for execution in executions {
            let status = AgentTaskStatus::from(execution.status.clone());
            if let Some(filter) = &status_filter {
                if status.as_str() != filter {
                    continue;
                }
            }

            summaries.push(TaskSummary {
                task_id: execution.execution_id,
                conversation_id: execution.conversation_id,
                status: status.as_str().to_string(),
                current_iteration: execution.current_iteration as u32,
                error_count: execution.error_count as u32,
                created_at: execution.created_at,
                updated_at: execution.updated_at,
            });
        }

        Ok(summaries)
    }

    pub async fn fetch_conversation_context(
        &self,
        conversation_id: i64,
    ) -> TaskExecutorResult<ConversationContextSnapshot> {
        let persistence = self.agent_persistence();

        let conversation = persistence
            .conversations()
            .get(conversation_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?
            .ok_or_else(|| {
                TaskExecutorError::InternalError(format!(
                    "Conversation {} not found",
                    conversation_id
                ))
            })?;

        let summary = persistence
            .conversation_summaries()
            .get(conversation_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let mut active_task_ids: Vec<String> = self.active_tasks()
            .iter()
            .filter_map(|entry| {
                let (task_id, ctx) = entry.pair();
                (ctx.conversation_id == conversation_id).then(|| task_id.clone())
            })
            .collect();
        active_task_ids.sort();

        let executions = persistence
            .agent_executions()
            .list_recent_by_conversation(conversation_id, CONTEXT_EXECUTION_LIMIT as i64)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let mut snapshots = Vec::new();
        for execution in executions {
            let messages = persistence
                .execution_messages()
                .list_by_execution(&execution.execution_id)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
            let tool_calls = persistence
                .tool_executions()
                .list_by_execution(&execution.execution_id)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
            let events = persistence
                .execution_events()
                .list_by_execution(&execution.execution_id)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

            snapshots.push(ExecutionSnapshot {
                execution,
                messages: tail_vec(messages, CONTEXT_MESSAGE_LIMIT),
                tool_calls: tail_vec(tool_calls, CONTEXT_TOOL_LIMIT),
                events: tail_vec(events, CONTEXT_EVENT_LIMIT),
            });
        }

        Ok(ConversationContextSnapshot {
            conversation,
            summary,
            active_task_ids,
            executions: snapshots,
        })
    }

    pub async fn fetch_file_context_status(
        &self,
        conversation_id: i64,
    ) -> TaskExecutorResult<FileContextStatus> {
        let persistence = self.agent_persistence();

        let active_files = persistence
            .file_context()
            .get_active_files(conversation_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
        let stale_files = persistence
            .file_context()
            .get_stale_files(conversation_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        Ok(FileContextStatus {
            conversation_id,
            total_active: active_files.len(),
            total_stale: stale_files.len(),
            active_files,
            stale_files,
        })
    }

    async fn register_conversation_context(&self, context: Arc<TaskContext>) {
        self.conversation_contexts().insert(context.conversation_id, context);
    }

    async fn get_context_builder(&self, context: &Arc<TaskContext>) -> Arc<ContextBuilder> {
        self.context_builder_cache()
            .entry(context.conversation_id)
            .or_insert_with(|| Arc::new(ContextBuilder::new(context.file_tracker())))
            .clone()
    }

    async fn conversation_context(&self, conversation_id: i64) -> Option<Arc<TaskContext>> {
        self.conversation_contexts()
            .get(&conversation_id)
            .map(|entry| entry.value().clone())
    }

    /// Restore TaskContext from database (used after app restart).
    async fn restore_from_db(
        &self,
        conversation_id: i64,
    ) -> TaskExecutorResult<Option<Arc<TaskContext>>> {
        let persistence = self.agent_persistence();

        let executions = persistence
            .agent_executions()
            .list_recent_by_conversation(conversation_id, 1)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let execution = match executions.first() {
            Some(e) => e,
            None => return Ok(None),
        };

        let db_messages = persistence
            .execution_messages()
            .list_by_execution(&execution.execution_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let context = self
            .rebuild_task_context(conversation_id, execution.clone(), db_messages)
            .await?;

        Ok(Some(Arc::new(context)))
    }

    /// Rebuild TaskContext from database records.
    async fn rebuild_task_context(
        &self,
        conversation_id: i64,
        execution: AgentExecution,
        db_messages: Vec<crate::agent::persistence::ExecutionMessage>,
    ) -> TaskExecutorResult<TaskContext> {
        let task_id = execution.execution_id.clone();
        let user_prompt = execution.user_request.clone();
        let cwd = self.resolve_task_cwd().await;

        let chat_mode = "agent".to_string();
        let tool_registry = crate::agent::tools::create_tool_registry(&chat_mode).await;

        let mut config = self.default_config();
        if let Some(config_json) = &execution.execution_config {
            if let Ok(saved_config) = serde_json::from_str::<TaskExecutionConfig>(config_json) {
                config = saved_config;
            }
        }

        let context = TaskContext::new(
            execution.clone(),
            config,
            cwd.clone(),
            tool_registry.clone(),
            None,
            self.database(),
            self.agent_persistence(),
            self.ui_persistence(),
        )
        .await?;

        // Restore message history from database -> Anthropic 原生消息
        let mut restored_messages: Vec<MessageParam> = Vec::new();

        for db_msg in db_messages {
            match db_msg.role {
                crate::agent::persistence::MessageRole::System => {
                    // 系统消息不作为消息恢复，系统提示词通过 build_task_prompts 刷新
                    let _ = db_msg; // ignore
                }
                crate::agent::persistence::MessageRole::User => {
                    restored_messages.push(MessageParam {
                        role: crate::llm::anthropic_types::MessageRole::User,
                        content: MessageContent::Text(db_msg.content),
                    });
                }
                crate::agent::persistence::MessageRole::Assistant => {
                    // 旧存储可能包含 parts 字段，这里简单回退为 Text
                    restored_messages.push(MessageParam {
                        role: crate::llm::anthropic_types::MessageRole::Assistant,
                        content: MessageContent::Text(db_msg.content),
                    });
                }
                crate::agent::persistence::MessageRole::Tool => {
                    if let Ok(result) = serde_json::from_str::<ToolCallResult>(&db_msg.content) {
                        let serialized = serde_json::to_string(&result.result)
                            .unwrap_or_else(|_| "{}".to_string());
                        restored_messages.push(MessageParam {
                            role: crate::llm::anthropic_types::MessageRole::User,
                            content: MessageContent::Blocks(vec![ContentBlock::ToolResult {
                                tool_use_id: result.call_id,
                                content: Some(AnthropicToolResultContent::Text(serialized)),
                                is_error: Some(result.is_error),
                            }]),
                        });
                    }
                }
            }
        }

        context.restore_messages(restored_messages).await?;

        // Rebuild System Prompt with latest environment
        let (new_system_prompt, _) = self
            .build_task_prompts(
                conversation_id,
                task_id,
                &user_prompt,
                Some(&cwd),
                &tool_registry,
            )
            .await?;

        context.update_system_prompt(new_system_prompt).await?;

        Ok(context)
    }

    async fn resolve_task_cwd(&self) -> String {
        if let Ok(terminal_ctx) = self.terminal_context_service().get_active_context().await {
            if let Some(dir) = terminal_ctx.current_working_directory {
                let trimmed = dir.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }

        std::env::current_dir()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/".to_string())
    }

    async fn build_task_prompts(
        &self,
        conversation_id: i64,
        task_id: String,
        user_prompt: &str,
        working_directory: Option<&str>,
        tool_registry: &ToolRegistry,
    ) -> TaskExecutorResult<(String, String)> {
        let cwd = working_directory.unwrap_or("/");
        let tool_schemas_full =
            tool_registry.get_tool_schemas_with_context(&ToolDescriptionContext {
                cwd: cwd.to_string(),
            });
        let simple_tool_schemas: Vec<ToolSchema> = tool_schemas_full
            .into_iter()
            .map(|s| ToolSchema {
                name: s.name,
                description: s.description,
                parameters: s.parameters,
            })
            .collect();

        let tool_names: Vec<String> = simple_tool_schemas.iter().map(|t| t.name.clone()).collect();
        let agent_info = Agent {
            name: "OrbitX Agent".to_string(),
            description: "An AI coding assistant for OrbitX".to_string(),
            capabilities: vec![],
            tools: tool_names,
        };

        let task_for_prompt = Task {
            id: task_id,
            conversation_id,
            user_prompt: user_prompt.to_string(),
            xml: None,
            status: crate::agent::types::TaskStatus::Created,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut prompt_ctx = AgentContext::default();
        if let Some(dir) = working_directory {
            prompt_ctx.working_directory = Some(dir.to_string());
        }
        prompt_ctx.additional_context.insert(
            "taskPrompt".to_string(),
            serde_json::Value::String(user_prompt.to_string()),
        );

        // 获取用户规则
        let user_rules = self.inner().cache.get_user_rules().await;

        // 获取项目规则（指定使用哪个配置文件）
        let project_rules = self.inner().cache.get_project_rules().await;

        // 合并项目上下文和用户规则
        let mut prompt_parts = Vec::new();

        let loader = ProjectContextLoader::new(cwd);
        if let Some(ctx) = loader.load_with_preference(project_rules.as_deref()).await {
            prompt_parts.push(ctx.format_for_prompt());
        }

        if let Some(rules) = user_rules {
            prompt_parts.push(rules);
        }

        let ext_sys_prompt = if prompt_parts.is_empty() {
            None
        } else {
            Some(prompt_parts.join("\n\n"))
        };

        let system_prompt = build_agent_system_prompt(
            agent_info.clone(),
            Some(task_for_prompt.clone()),
            Some(prompt_ctx.clone()),
            simple_tool_schemas.clone(),
            ext_sys_prompt,
        )
        .await
        .map_err(|e| {
            TaskExecutorError::InternalError(format!("Failed to build system prompt: {}", e))
        })?;

        let user_prompt_built = build_agent_user_prompt(
            agent_info,
            Some(task_for_prompt),
            Some(prompt_ctx),
            simple_tool_schemas,
        )
        .await
        .map_err(|e| {
            TaskExecutorError::InternalError(format!("Failed to build user prompt: {}", e))
        })?;

        Ok((system_prompt, user_prompt_built))
    }

    fn spawn_react_execution(&self, context: Arc<TaskContext>, model_id: String) {
        let executor = self.clone();
        tokio::spawn(async move {
            let task_id = context.task_id.clone();
            let _conversation_id = context.conversation_id;
            let result = executor.run_react_loop(context.clone(), model_id).await;

            match result {
                Ok(_) => {
                    if let Err(e) = context.set_status(AgentTaskStatus::Completed).await {
                        error!("Failed to update task status to completed: {}", e);
                    }

                    if let Err(e) = context
                        .send_progress(TaskProgressPayload::TaskCompleted(TaskCompletedPayload {
                            task_id: task_id.clone(),
                            final_iteration: context.current_iteration().await,
                            completion_reason: "Task completed successfully".to_string(),
                            timestamp: Utc::now(),
                        }))
                        .await
                    {
                        error!("Failed to send task completed event: {}", e);
                    }
                }
                Err(e) => {
                    executor
                        .handle_task_error(&task_id, e.into(), context.clone())
                        .await;
                }
            }

            // 显式关闭 progress channel，确保前端收到 stream 结束信号
            context.set_progress_channel(None).await;

            executor.active_tasks().remove(&task_id);

            info!(
                "Task {} execution completed, conversation context preserved",
                task_id
            );
        });
    }

    pub async fn trigger_conversation_summary(
        &self,
        conversation_id: i64,
        model_override: Option<String>,
    ) -> TaskExecutorResult<Option<SummaryResult>> {
        let persistence = self.agent_persistence();
        let mut executions = persistence
            .agent_executions()
            .list_recent_by_conversation(conversation_id, 1)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let Some(latest_execution) = executions.pop() else {
            return Ok(None);
        };

        let messages = persistence
            .execution_messages()
            .list_by_execution(&latest_execution.execution_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        if messages.is_empty() {
            return Ok(None);
        }

        let llm_messages = convert_execution_messages(&messages);
        let summarizer =
            ConversationSummarizer::new(conversation_id, persistence.clone(), self.database());

        let model_id = match model_override {
            Some(model) => model,
            None => self.get_default_model_id().await?,
        };

        let result = summarizer
            .summarize_now(&model_id, &llm_messages, &None)
            .await
            .map_err(|e| TaskExecutorError::InternalError(e.to_string()))?;

        persistence
            .agent_executions()
            .set_has_context(&latest_execution.execution_id, true)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        persistence
            .conversations()
            .touch(conversation_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        Ok(Some(result))
    }

    pub fn agent_persistence(&self) -> Arc<AgentPersistence> {
        Arc::clone(&self.inner().agent_persistence)
    }

    pub fn ui_persistence(&self) -> Arc<AgentUiPersistence> {
        Arc::clone(&self.inner().ui_persistence)
    }

    /// 从数据库获取模型的 context window 大小
    async fn get_model_context_window(&self, model_id: &str) -> Option<u32> {
        let db = self.database();
        let model = crate::storage::repositories::AIModels::new(&db)
            .find_by_id(model_id)
            .await
            .ok()??;

        // 从 options JSON 中读取 maxContextTokens
        if let Some(options) = model.options {
            if let Some(max_tokens) = options.get("maxContextTokens") {
                if let Some(value) = max_tokens.as_u64() {
                    return Some(value as u32);
                }
            }
        }

        None
    }

    /// ReAct循环执行（核心逻辑）
    /// ReAct循环执行（重构版：简化逻辑，移除idle计数安全网）
    pub(crate) async fn run_react_loop(
        &self,
        context: Arc<TaskContext>,
        model_id: String,
    ) -> TaskExecutorResult<()> {
        info!("Starting ReAct loop for task: {}", context.task_id);
        info!(
            "[MODEL_SELECTION] run_react_loop using model_id: {}",
            model_id
        );

        let mut iteration_snapshots: Vec<IterationSnapshot> = Vec::new();

        while !context.should_stop().await {
            context.check_aborted(false).await?;

            // ===== Phase 1: 迭代初始化 =====
            let iteration = context.increment_iteration().await?;
            debug!("Task {} starting iteration {}", context.task_id, iteration);

            let react_iteration_index = {
                let runtime_handle = context.react_runtime();
                let mut runtime = runtime_handle.write().await;
                runtime.start_iteration()
            };

            let iter_ctx = context.begin_iteration(iteration).await;

            // ===== Phase 2: 准备消息上下文（零转换） =====
            info!("[MODEL_SELECTION] Using model_id: {}", model_id);

            // 上下文窗口（后续 compactor/summarizer 将使用）
            let _context_window = self
                .get_model_context_window(&model_id)
                .await
                .unwrap_or(128_000);

            // ===== Phase 3: 调用 LLM =====
            let tool_registry = context.tool_registry();

            // ===== Phase 2 (续): 摘要、压缩、文件上下文（零持久化） =====
            // 注：这里转Vec是因为summarizer/compactor还用Vec，这是合理的边界转换
            let mut working_messages: Vec<MessageParam> = context.get_messages().await.into();
            let mut system_prompt: Option<SystemPrompt> = context.get_system_prompt().await;

            // 摘要（如果需要）
            let summarizer = ConversationSummarizer::new(
                context.conversation_id,
                self.agent_persistence(),
                self.database(),
            );
            if let Ok(Some(summary)) = summarizer
                .summarize_if_needed(&model_id, &working_messages, &system_prompt)
                .await
            {
                let sys_text = match system_prompt.clone() {
                    Some(SystemPrompt::Text(t)) => {
                        // 使用with_capacity预分配，避免多次内存分配
                        let capacity = t.len() + summary.summary.len() + 15;
                        let mut buf = String::with_capacity(capacity);
                        buf.push_str(&t);
                        buf.push_str("\n\n[summary]\n");
                        buf.push_str(&summary.summary);
                        buf
                    }
                    Some(SystemPrompt::Blocks(_)) => summary.summary, // 简化处理
                    None => summary.summary,
                };
                let _ = context.update_system_prompt(sys_text).await;
                system_prompt = context.get_system_prompt().await;
            }

            // 会话压缩历史（如有），并入 system prompt
            let compressed_history = context.session().get_compressed_history_text().await;
            if !compressed_history.is_empty() {
                let sys_text = match system_prompt.clone() {
                    Some(SystemPrompt::Text(t)) => {
                        // 使用with_capacity预分配，避免多次内存分配
                        let capacity = t.len() + compressed_history.len() + 15;
                        let mut buf = String::with_capacity(capacity);
                        buf.push_str(&t);
                        buf.push_str("\n\n[history]\n");
                        buf.push_str(&compressed_history);
                        buf
                    }
                    Some(SystemPrompt::Blocks(_)) => compressed_history.clone(),
                    None => compressed_history.clone(),
                };
                let _ = context.update_system_prompt(sys_text).await;
                system_prompt = context.get_system_prompt().await;
            }

            // 文件上下文（如有），追加为 user 临时消息
            let recent_iterations = context
                .react_runtime()
                .read()
                .await
                .get_snapshot()
                .iterations;
            let builder = self.get_context_builder(&context).await;
            if let Some(file_msg) = builder.build_file_context_message(&recent_iterations).await {
                working_messages.push(file_msg);
            }

            // 消息压缩（超过上下文窗口时）
            let context_window = self
                .get_model_context_window(&model_id)
                .await
                .unwrap_or(128_000);
            let compaction_result = MessageCompactor::new()
                .with_config(CompactionConfig::default())
                .compact_if_needed(
                    working_messages,
                    system_prompt.clone(),
                    &model_id,
                    context_window,
                )
                .await
                .map_err(|e| {
                    TaskExecutorError::InternalError(format!("Compaction failed: {}", e))
                })?;
            if let CompactionResult::Compacted {
                tokens_saved,
                messages_summarized,
                ..
            } = &compaction_result
            {
                info!(
                    "Compacted {} messages, saved {} tokens",
                    messages_summarized, tokens_saved
                );
            }
            let final_messages = compaction_result.messages();

            let llm_request = self
                .build_llm_request(
                    &context,
                    model_id.clone(),
                    &tool_registry,
                    &context.cwd,
                    Some(final_messages),
                )
                .await?;

            // 打印 system prompt
            if let Some(ref sp) = llm_request.system {
                if let crate::llm::anthropic_types::SystemPrompt::Text(text) = sp {
                    println!("\n{}\nFINAL SYSTEM PROMPT:\n{}\n{}\n{}\n", "=".repeat(80), "=".repeat(80), text, "=".repeat(80));
                }
            }

            let llm_service = crate::llm::service::LLMService::new(self.database());
            let cancel_token = context.register_step_token();
            let mut stream = llm_service
                .call_stream(llm_request, cancel_token)
                .await
                .map_err(|e| {
                    TaskExecutorError::InternalError(format!("LLM stream call failed: {}", e))
                })?;

            // 新的流处理状态
            let mut current_blocks: HashMap<usize, BlockAccumulator> = HashMap::new();
            let mut text_content: Vec<String> = Vec::new();
            let mut tool_use_blocks: Vec<ContentBlock> = Vec::new();
            let mut pending_tool_calls: Vec<(String, String, serde_json::Value)> = Vec::new();

            let mut thinking_stream_id: Option<String> = None;
            let mut text_stream_id: Option<String> = None;

            // ===== Phase 4: 处理 Anthropic StreamEvent =====
            while let Some(item) = stream.next().await {
                context.check_aborted(true).await?;

                match item {
                    Ok(StreamEvent::MessageStart { message }) => {
                        tracing::debug!("Message started: {}", message.id);
                    }
                    Ok(StreamEvent::ContentBlockStart {
                        index,
                        content_block,
                    }) => match content_block {
                        ContentBlockStart::Text { text } => {
                            current_blocks.insert(index, BlockAccumulator::Text(text));
                        }
                        ContentBlockStart::ToolUse { id, name } => {
                            current_blocks.insert(
                                index,
                                BlockAccumulator::ToolUse {
                                    id,
                                    name,
                                    input_json: String::new(),
                                },
                            );
                        }
                        ContentBlockStart::Thinking { thinking } => {
                            current_blocks.insert(index, BlockAccumulator::Thinking(thinking));
                        }
                    },
                    Ok(StreamEvent::ContentBlockDelta { index, delta }) => {
                        if let Some(block) = current_blocks.get_mut(&index) {
                            match delta {
                                ContentDelta::TextDelta { text } => {
                                    if let BlockAccumulator::Text(s) = block {
                                        s.push_str(&text);
                                        // emit text delta
                                        if text_stream_id.is_none() {
                                            text_stream_id = Some(Uuid::new_v4().to_string());
                                        }
                                        context
                                            .send_progress(TaskProgressPayload::Text(TextPayload {
                                                task_id: context.task_id.clone(),
                                                iteration,
                                                text,
                                                stream_id: text_stream_id.clone().unwrap(),
                                                stream_done: false,
                                                timestamp: Utc::now(),
                                            }))
                                            .await?;
                                        iter_ctx.append_output(&s).await;
                                    }
                                }
                                ContentDelta::InputJsonDelta { partial_json } => {
                                    if let BlockAccumulator::ToolUse { input_json, .. } = block {
                                        input_json.push_str(&partial_json);
                                    }
                                }
                                ContentDelta::ThinkingDelta { thinking } => {
                                    if let BlockAccumulator::Thinking(s) = block {
                                        s.push_str(&thinking);
                                        if thinking_stream_id.is_none() {
                                            thinking_stream_id = Some(Uuid::new_v4().to_string());
                                        }
                                        context
                                            .send_progress(TaskProgressPayload::Thinking(
                                                ThinkingPayload {
                                                    task_id: context.task_id.clone(),
                                                    iteration,
                                                    thought: thinking,
                                                    stream_id: thinking_stream_id.clone().unwrap(),
                                                    stream_done: false,
                                                    timestamp: Utc::now(),
                                                },
                                            ))
                                            .await?;
                                        iter_ctx.append_thinking(&s).await;
                                    }
                                }
                            }
                        }
                    }
                    Ok(StreamEvent::ContentBlockStop { index }) => {
                        if let Some(block) = current_blocks.remove(&index) {
                            match block {
                                BlockAccumulator::Text(text) => {
                                    if !text.is_empty() {
                                        text_content.push(text);
                                    }
                                }
                                BlockAccumulator::ToolUse {
                                    id,
                                    name,
                                    input_json,
                                } => {
                                    // 尝试解析完整 JSON
                                    let input: serde_json::Value = serde_json::from_str(
                                        &input_json,
                                    )
                                    .unwrap_or(serde_json::json!({"_streaming_args": input_json}));
                                    tool_use_blocks.push(ContentBlock::ToolUse {
                                        id: id.clone(),
                                        name: name.clone(),
                                        input: input.clone(),
                                    });

                                    // 记录动作与收集待执行工具
                                    context.react_runtime().write().await.record_action(
                                        react_iteration_index,
                                        name.clone(),
                                        input.clone(),
                                    );
                                    iter_ctx
                                        .add_tool_call(id.clone(), name.clone(), input.clone())
                                        .await;
                                    pending_tool_calls.push((id, name, input));
                                }
                                BlockAccumulator::Thinking(_) => {}
                            }
                        }
                    }
                    Ok(StreamEvent::MessageDelta { delta, usage }) => {
                        // 可在此记录 stop_reason/usage
                        let _ = delta;
                        let _ = usage;
                    }
                    Ok(StreamEvent::MessageStop) => {
                        break;
                    }
                    Ok(StreamEvent::Ping) => {}
                    Ok(StreamEvent::Error { error }) => {
                        return Err(TaskExecutorError::InternalError(error.message));
                    }
                    Err(e) => {
                        return Err(TaskExecutorError::InternalError(e.to_string()));
                    }
                }
            }

            // ===== Phase 4: 将累积内容写入上下文 =====
            let final_text = if !text_content.is_empty() {
                Some(text_content.join("\n"))
            } else {
                None
            };
            context
                .add_assistant_message(final_text.clone(), Some(tool_use_blocks))
                .await;

            // ===== Phase 5: 分类迭代结果 =====
            let outcome = if !pending_tool_calls.is_empty() {
                IterationOutcome::ContinueWithTools {
                    tool_calls: pending_tool_calls.clone(),
                }
            } else if final_text
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            {
                IterationOutcome::Complete {
                    thinking: None,
                    output: final_text.clone(),
                }
            } else {
                IterationOutcome::Empty
            };

            debug!("Iteration {} outcome: {}", iteration, outcome.description());

            // ===== Phase 6: 根据结果执行动作 =====
            match outcome {
                IterationOutcome::ContinueWithTools { ref tool_calls } => {
                    info!(
                        "Iteration {}: executing {} tools",
                        iteration,
                        tool_calls.len()
                    );

                    // 去重检测
                    let deduplicated_calls = self.deduplicate_tool_uses(tool_calls);
                    if deduplicated_calls.len() < tool_calls.len() {
                        let duplicates_count = tool_calls.len() - deduplicated_calls.len();
                        warn!(
                            "Detected {} duplicate tool calls in iteration {}",
                            duplicates_count, iteration
                        );

                        // 注入系统消息警告LLM（仅内存，不持久化）
                        let _ = context.set_system_prompt(format!(
                            "<system-reminder type=\"duplicate-tools\">\n\
                             You called {} duplicate tool(s) in this iteration.\n\
                             The results haven't changed. Please use the existing results instead of re-calling the same tools.\n\
                             </system-reminder>",
                            duplicates_count
                        )).await;
                    }

                    let results = self
                        .execute_tools_parallel(&context, iteration, deduplicated_calls)
                        .await?;

                    for result in results {
                        iter_ctx.add_tool_result(result.clone()).await;

                        let outcome = tool_call_result_to_outcome(&result);
                        context
                            .with_chain_mut({
                                let call_id = result.call_id.clone();
                                let outcome_for_chain = outcome.clone();
                                move |chain| {
                                    chain.update_tool_result(&call_id, outcome_for_chain);
                                }
                            })
                            .await;

                        {
                            let runtime_handle = context.react_runtime();
                            let mut runtime = runtime_handle.write().await;
                            runtime.record_observation(
                                react_iteration_index,
                                result.tool_name.clone(),
                                outcome,
                            );

                            if result.is_error {
                                runtime.fail_iteration(
                                    react_iteration_index,
                                    format!("Tool {} failed", result.tool_name),
                                );
                            } else {
                                runtime.reset_error_counter();
                            }
                        }
                    }

                    // 循环检测
                    if let Some(loop_warning) = self.detect_loop_pattern(&context, iteration).await
                    {
                        warn!("Loop pattern detected in iteration {}", iteration);
                        let _ = context.set_system_prompt(loop_warning).await;
                    }

                    self.finalize_iteration(&context, &mut iteration_snapshots)
                        .await?;
                    continue;
                }

                IterationOutcome::Complete { thinking, output } => {
                    info!(
                        "Iteration {}: task complete - {}",
                        iteration,
                        match (&thinking, &output) {
                            (Some(_), Some(_)) => "thinking + output",
                            (Some(_), None) => "thinking only",
                            (None, Some(_)) => "output only",
                            (None, None) => "empty (unexpected)",
                        }
                    );

                    context.react_runtime().write().await.complete_iteration(
                        react_iteration_index,
                        output.clone(),
                        None,
                    );

                    self.finalize_iteration(&context, &mut iteration_snapshots)
                        .await?;
                    break;
                }

                IterationOutcome::Empty => {
                    warn!(
                        "Iteration {}: empty response - terminating immediately",
                        iteration
                    );

                    self.finalize_iteration(&context, &mut iteration_snapshots)
                        .await?;
                    break; // 空响应直接终止，不再尝试
                }
            }
        }

        info!("ReAct loop completed for task: {}", context.task_id);
        if !iteration_snapshots.is_empty() {
            self.compress_iteration_batch(&context, &iteration_snapshots)
                .await?;
        }
        Ok(())
    }

    async fn finalize_iteration(
        &self,
        context: &Arc<TaskContext>,
        snapshots: &mut Vec<IterationSnapshot>,
    ) -> TaskExecutorResult<()> {
        if let Some(snapshot) = context.end_iteration().await {
            let tool_calls = snapshot.tools_used.len() as u32;
            let files = snapshot.files_touched.len() as u32;
            context
                .session()
                .update_stats(|stats| {
                    stats.total_iterations = stats.total_iterations.saturating_add(1);
                    stats.total_tool_calls = stats.total_tool_calls.saturating_add(tool_calls);
                    stats.files_read = stats.files_read.saturating_add(files);
                })
                .await;
            snapshots.push(snapshot);
            if snapshots.len() >= 5 {
                self.compress_iteration_batch(context, snapshots).await?;
                snapshots.clear();
            }
        }
        Ok(())
    }

    async fn compress_iteration_batch(
        &self,
        context: &Arc<TaskContext>,
        snapshots: &[IterationSnapshot],
    ) -> TaskExecutorResult<()> {
        if snapshots.is_empty() {
            return Ok(());
        }

        let start_iter = snapshots.first().unwrap().iteration;
        let end_iter = snapshots.last().unwrap().iteration;

        let mut files = Vec::new();
        let mut tools = Vec::new();
        let mut summary_parts = Vec::new();

        for snapshot in snapshots {
            files.extend(snapshot.files_touched.clone());
            tools.extend(snapshot.tools_used.clone());
            summary_parts.push(snapshot.summarize());
        }

        files.sort();
        files.dedup();
        tools.sort();
        tools.dedup();

        let memory = CompressedMemory {
            created_at: Utc::now(),
            iteration_range: (start_iter, end_iter),
            summary: summary_parts.join("\n"),
            files_touched: files,
            tools_used: tools,
            tokens_saved: 0,
        };

        context.session().add_compressed_memory(memory).await;

        Ok(())
    }

    /// 创建任务上下文
    pub(crate) async fn create_task_context(
        &self,
        params: ExecuteTaskParams,
        progress_channel: Option<Channel<TaskProgressPayload>>,
        cwd_override: Option<String>,
    ) -> TaskExecutorResult<TaskContext> {
        let mut config = self.default_config();
        if let Some(overrides) = params.config_overrides {
            self.apply_config_overrides(&mut config, overrides)?;
        }

        let persistence = self.agent_persistence();

        let user_prompt_raw = params.user_prompt.clone();

        let cwd = match cwd_override {
            Some(value) => value,
            None => self.resolve_task_cwd().await,
        };

        let tool_registry = crate::agent::tools::create_tool_registry(&params.chat_mode).await;

        let task_prompt_id = Uuid::new_v4().to_string();
        let (system_prompt, user_prompt) = self
            .build_task_prompts(
                params.conversation_id,
                task_prompt_id,
                &params.user_prompt,
                Some(&cwd),
                &tool_registry,
            )
            .await?;

        let config_json = serde_json::to_string(&config).ok();
        persistence
            .conversations()
            .ensure_with_id(params.conversation_id, None, None)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let execution = persistence
            .agent_executions()
            .create(
                params.conversation_id,
                &params.user_prompt,
                &system_prompt,
                config_json.as_deref(),
                false,
                config.max_iterations as i64,
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let context = TaskContext::new(
            execution,
            config,
            cwd.clone(),
            tool_registry,
            progress_channel.clone(),
            self.database(),
            persistence.clone(),
            self.ui_persistence(),
        )
        .await?;

        context
            .set_initial_prompts(system_prompt.clone(), user_prompt.clone())
            .await?;

        context.initialize_ui_track(&user_prompt_raw).await?;

        if let Some(channel) = &progress_channel {
            channel.send(TaskProgressPayload::TaskCreated(TaskCreatedPayload {
                task_id: context.task_id.clone(),
                conversation_id: context.conversation_id,
                user_prompt,
            }))?;
        }

        Ok(context)
    }

    /// 处理任务错误
    pub(crate) async fn handle_task_error(
        &self,
        task_id: &str,
        error: AgentError,
        context: Arc<TaskContext>,
    ) {
        error!("Task {} error: {}", task_id, error);

        // 更新任务状态为错误
        if let Err(e) = context.set_status(AgentTaskStatus::Error).await {
            error!("Failed to update task status to error: {}", e);
        }

        if let Err(e) = context
            .send_progress(TaskProgressPayload::TaskError(TaskErrorPayload {
                task_id: task_id.to_string(),
                iteration: context.current_iteration().await,
                error_message: error.to_string(),
                error_type: "TaskExecutorError".to_string(),
                is_recoverable: false,
                timestamp: Utc::now(),
            }))
            .await
        {
            error!("Failed to send error event: {}", e);
        }

        // 记录错误到执行日志
        let error_payload = serde_json::json!({
            "error": error.to_string(),
            "error_type": "TaskExecutorError",
            "is_recoverable": false
        });

        let event_data = serde_json::to_string(&error_payload).unwrap_or_else(|_| "{}".to_string());

        let persistence = self.agent_persistence();
        let result = persistence
            .execution_events()
            .record_event(
                &context.task_id,
                ExecutionEventType::Error,
                &event_data,
                context.current_iteration().await as i64,
            )
            .await;

        if let Err(e) = result {
            error!("Failed to log error to execution log: {}", e);
        }
    }

    /// 应用配置覆盖
    fn apply_config_overrides(
        &self,
        config: &mut TaskExecutionConfig,
        overrides: serde_json::Value,
    ) -> TaskExecutorResult<()> {
        if let Some(max_iterations) = overrides.get("max_iterations").and_then(|v| v.as_u64()) {
            config.max_iterations = max_iterations as u32;
        }
        if let Some(max_errors) = overrides.get("max_errors").and_then(|v| v.as_u64()) {
            config.max_errors = max_errors as u32;
        }
        Ok(())
    }

    // === 双轨架构新增方法 ===

    /// 创建新会话
    pub async fn create_conversation(
        &self,
        title: Option<String>,
        workspace_path: Option<String>,
    ) -> TaskExecutorResult<i64> {
        let persistence = self.agent_persistence();
        let conversation = persistence
            .conversations()
            .create(title.as_deref(), workspace_path.as_deref())
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        Ok(conversation.id)
    }

    // 工具去重和并行执行

    /// 去重工具调用 - 检测同一iteration内的重复调用
    fn deduplicate_tool_uses(
        &self,
        tool_calls: &[(String, String, serde_json::Value)],
    ) -> Vec<(String, String, serde_json::Value)> {
        use std::collections::HashSet;

        let mut seen = HashSet::new();
        let mut deduplicated = Vec::new();

        for (id, name, args) in tool_calls.iter() {
            // 使用 (tool_name, arguments) 作为唯一键
            let key = (
                name.clone(),
                serde_json::to_string(args).unwrap_or_default(),
            );

            if seen.insert(key) {
                deduplicated.push((id.clone(), name.clone(), args.clone()));
            } else {
                debug!(
                    "Skipping duplicate tool call: {} with args {:?}",
                    name, args
                );
            }
        }

        deduplicated
    }

    /// 并行执行多个工具调用
    async fn execute_tools_parallel(
        &self,
        context: &Arc<TaskContext>,
        iteration: u32,
        tool_calls: Vec<(String, String, serde_json::Value)>,
    ) -> TaskExecutorResult<Vec<ToolCallResult>> {
        use futures::future::join_all;

        if tool_calls.is_empty() {
            return Ok(Vec::new());
        }

        // 创建所有工具调用的futures
        let futures: Vec<_> = tool_calls
            .into_iter()
            .map(|(id, name, args)| {
                let executor = self.clone();
                let ctx = Arc::clone(context);
                async move {
                    executor
                        .execute_tool_call(&ctx, iteration, id, name, args)
                        .await
                }
            })
            .collect();

        // 并行执行所有工具调用
        let results = join_all(futures).await;

        // 收集成功的结果，记录失败的
        let mut successful_results = Vec::new();
        for result in results {
            match result {
                Ok(tool_result) => successful_results.push(tool_result),
                Err(e) => {
                    error!("Tool execution failed in parallel batch: {}", e);
                    // 继续执行其他工具，不中断整个批次
                }
            }
        }

        Ok(successful_results)
    }

    // 循环检测系统

    /// 检测循环模式 - 分析最近N个iterations是否形成重复模式
    async fn detect_loop_pattern(
        &self,
        context: &Arc<TaskContext>,
        current_iteration: u32,
    ) -> Option<String> {
        const LOOP_DETECTION_WINDOW: usize = 3;

        if current_iteration < LOOP_DETECTION_WINDOW as u32 {
            return None;
        }

        // 从ReactRuntime获取iterations历史
        let runtime_handle = context.react_runtime();
        let runtime_guard = runtime_handle.read().await;
        let snapshot = runtime_guard.get_snapshot();
        let iterations = &snapshot.iterations;

        if iterations.len() < LOOP_DETECTION_WINDOW {
            return None;
        }

        // 获取最近N个iterations
        let recent: Vec<_> = iterations
            .iter()
            .rev()
            .take(LOOP_DETECTION_WINDOW)
            .collect();

        // 检测模式1：完全相同的工具调用序列
        if let Some(warning) = self.detect_identical_tool_sequence(&recent) {
            return Some(warning);
        }

        // 检测模式2：相似的工具调用模式（同样的工具，不同参数）
        if let Some(warning) = self.detect_similar_tool_pattern(&recent) {
            return Some(warning);
        }

        None
    }

    /// 检测完全相同的工具调用序列
    fn detect_identical_tool_sequence(
        &self,
        recent_iterations: &[&crate::agent::react::types::ReactIteration],
    ) -> Option<String> {
        if recent_iterations.len() < 2 {
            return None;
        }

        // 提取每个iteration的工具名称序列
        let tool_sequences: Vec<Vec<String>> = recent_iterations
            .iter()
            .map(|iter| {
                iter.action
                    .as_ref()
                    .map(|action| vec![action.tool_name.clone()])
                    .unwrap_or_default()
            })
            .collect();

        // 检查是否所有序列都相同且非空
        let first = &tool_sequences[0];
        if first.is_empty() {
            return None;
        }

        let all_identical = tool_sequences[1..].iter().all(|seq| seq == first);

        if all_identical {
            let tools_list = first.join(", ");
            return Some(format!(
                "<system-reminder type=\"loop-warning\">\n\
                 You've called the same tools {} times in a row: {}\n\n\
                 The results haven't changed. Consider:\n\
                 - Have you gathered enough information?\n\
                 - Can you proceed with what you have?\n\
                 - Do you need to try a different approach?\n\n\
                 Break the loop by using the information you already have or trying different tools.\n\
                 </system-reminder>",
                recent_iterations.len(),
                tools_list
            ));
        }

        None
    }

    /// 检测相似的工具调用模式（同样的工具类型，可能不同参数）
    fn detect_similar_tool_pattern(
        &self,
        recent_iterations: &[&crate::agent::react::types::ReactIteration],
    ) -> Option<String> {
        if recent_iterations.len() < 3 {
            return None;
        }

        // 提取工具名称（不考虑参数）
        let tool_names: Vec<Option<String>> = recent_iterations
            .iter()
            .map(|iter| iter.action.as_ref().map(|a| a.tool_name.clone()))
            .collect();

        // 统计工具使用频率
        let mut tool_counts = std::collections::HashMap::new();
        for name in tool_names.iter().flatten() {
            *tool_counts.entry(name.clone()).or_insert(0) += 1;
        }

        // 如果有工具被重复调用超过2次
        for (tool, count) in tool_counts {
            if count >= 3 {
                return Some(format!(
                    "<system-reminder type=\"loop-warning\">\n\
                     You've called '{}' tool {} times in the last {} iterations.\n\n\
                     You may be stuck in a pattern. Consider:\n\
                     - Are you getting new information each time?\n\
                     - Can you analyze the results you already have?\n\
                     - Should you try a different approach?\n\n\
                     Try to make progress with the information you've gathered.\n\
                     </system-reminder>",
                    tool,
                    count,
                    recent_iterations.len()
                ));
            }
        }

        None
    }
}

fn tool_call_result_to_outcome(result: &ToolCallResult) -> ToolOutcome {
    let content = if result.is_error {
        let message = result
            .result
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Tool execution failed")
            .to_string();
        ToolResultContent::Error(message)
    } else {
        // 将 JSON 结果转换为字符串
        let result_str = serde_json::to_string(&result.result)
            .unwrap_or_else(|_| "Tool execution succeeded".to_string());
        ToolResultContent::Success(result_str)
    };

    ToolOutcome {
        content: vec![content],
        is_error: result.is_error,
        execution_time_ms: Some(result.execution_time_ms),
        ext_info: None,
    }
}

fn tail_vec<T: Clone>(items: Vec<T>, limit: usize) -> Vec<T> {
    if limit == 0 || items.len() <= limit {
        items
    } else {
        items[items.len() - limit..].to_vec()
    }
}

fn convert_execution_messages(messages: &[ExecutionMessage]) -> Vec<MessageParam> {
    messages
        .iter()
        .map(|msg| MessageParam {
            role: match msg.role.as_str() {
                "user" => crate::llm::anthropic_types::MessageRole::User,
                _ => crate::llm::anthropic_types::MessageRole::Assistant,
            },
            content: MessageContent::Text(msg.content.clone()),
        })
        .collect()
}
