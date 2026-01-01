/*!
 * 任务生命周期管理
 */

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::task;
use tracing::{error, warn};

use crate::agent::core::context::TaskContext;
use crate::agent::core::executor::{ExecuteTaskParams, TaskExecutor};
use crate::agent::core::status::AgentTaskStatus;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::events::{
    FinishPayload, TaskCancelledPayload, TaskCompletedPayload, TaskCreatedPayload,
    TaskErrorPayload, TaskPausedPayload, TaskProgressPayload, TaskResumedPayload,
    TaskStartedPayload,
};
use crate::workspace::{WorkspaceService, UNGROUPED_WORKSPACE_PATH};

impl TaskExecutor {
    pub async fn execute_task(
        &self,
        params: ExecuteTaskParams,
        progress_channel: Channel<TaskProgressPayload>,
    ) -> TaskExecutorResult<Arc<TaskContext>> {
        // 规范化参数：空工作区或 session_id=0 时使用未分组会话
        let params = self.normalize_task_params(params).await?;

        let ctx = self
            .build_or_restore_context(&params, Some(progress_channel))
            .await?;

        ctx.send_progress(TaskProgressPayload::TaskCreated(TaskCreatedPayload {
            task_id: ctx.task_id.to_string(),
            session_id: ctx.session_id,
            workspace_path: ctx.cwd.to_string(),
            user_prompt: params.user_prompt.clone(),
        }))
        .await?;

        // 先创建用户消息（获取 message_id）
        let user_message_id = ctx
            .initialize_ui_track(&params.user_prompt, params.images.as_deref())
            .await?;

        if ctx.checkpointing_enabled() {
            if let Err(err) = ctx.init_checkpoint(user_message_id).await {
                warn!("Failed to initialize checkpoint: {}", err);
            }
        }

        let (system_prompt, _) = self
            .prompt_orchestrator()
            .build_task_prompts(
                ctx.session_id,
                ctx.task_id.to_string(),
                &params.user_prompt,
                &ctx.cwd,
                &ctx.tool_registry(),
            )
            .await?;

        ctx.set_system_prompt(system_prompt).await?;

        // 自动检测会话是否有历史执行记录，有则恢复上下文
        let has_history = self
            .agent_persistence()
            .agent_executions()
            .list_recent_by_session(ctx.session_id, 2)
            .await
            .map(|execs| execs.len() > 1) // 当前执行 + 至少一个历史执行
            .unwrap_or(false);

        if has_history {
            self.restore_session_history(&ctx, ctx.session_id).await?;
        }

        ctx.add_user_message_with_images(params.user_prompt, params.images.as_deref())
            .await;
        ctx.set_status(AgentTaskStatus::Running).await?;

        ctx.send_progress(TaskProgressPayload::TaskStarted(TaskStartedPayload {
            task_id: ctx.task_id.to_string(),
            iteration: 0,
        }))
        .await?;

        let executor = self.clone();
        let ctx_for_spawn = Arc::clone(&ctx);
        let model_id = params.model_id.clone();

        task::spawn(async move {
            if let Err(e) = executor.run_task_loop(ctx_for_spawn, model_id).await {
                error!("Task execution failed: {}", e);
            }
        });

        Ok(ctx)
    }

    async fn run_task_loop(
        &self,
        ctx: Arc<TaskContext>,
        model_id: String,
    ) -> TaskExecutorResult<()> {
        // 直接调用ReactOrchestrator，传递self作为ReactHandler
        // 编译器会为TaskExecutor生成特化代码，完全内联
        let result = self
            .react_orchestrator()
            .run_react_loop(&ctx, &model_id, self)
            .await;

        match result {
            Ok(()) => {
                ctx.set_status(AgentTaskStatus::Completed).await?;
                let iteration = ctx.current_iteration().await;

                ctx.send_progress(TaskProgressPayload::TaskCompleted(TaskCompletedPayload {
                    task_id: ctx.task_id.to_string(),
                    final_iteration: iteration,
                    completion_reason: "success".to_string(),
                    timestamp: chrono::Utc::now(),
                }))
                .await?;

                ctx.send_progress(TaskProgressPayload::Finish(FinishPayload {
                    task_id: ctx.task_id.to_string(),
                    iteration,
                    finish_reason: "completed".to_string(),
                    usage: None,
                    timestamp: chrono::Utc::now(),
                }))
                .await?;
            }
            Err(e) => {
                error!("Task failed: {}", e);
                ctx.set_status(AgentTaskStatus::Error).await?;
                let iteration = ctx.current_iteration().await;

                ctx.send_progress(TaskProgressPayload::TaskError(TaskErrorPayload {
                    task_id: ctx.task_id.to_string(),
                    iteration,
                    error_type: "execution_error".to_string(),
                    error_message: e.to_string(),
                    is_recoverable: false,
                    timestamp: chrono::Utc::now(),
                }))
                .await?;
            }
        }

        Ok(())
    }

    pub async fn pause_task(
        &self,
        task_id: &str,
        abort_current_step: bool,
    ) -> TaskExecutorResult<()> {
        let ctx = self
            .active_tasks()
            .get(task_id)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or_else(|| TaskExecutorError::TaskNotFound(task_id.to_string()))?;

        ctx.set_pause(true, abort_current_step);
        ctx.set_status(AgentTaskStatus::Paused).await?;

        ctx.send_progress(TaskProgressPayload::TaskPaused(TaskPausedPayload {
            task_id: task_id.to_string(),
            reason: if abort_current_step {
                "user_requested_with_abort"
            } else {
                "user_requested"
            }
            .to_string(),
            timestamp: chrono::Utc::now(),
        }))
        .await?;

        Ok(())
    }

    pub async fn resume_task(
        &self,
        task_id: &str,
        progress_channel: Channel<TaskProgressPayload>,
    ) -> TaskExecutorResult<()> {
        let ctx = self
            .active_tasks()
            .get(task_id)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or_else(|| TaskExecutorError::TaskNotFound(task_id.to_string()))?;

        // 更新progress channel
        ctx.set_progress_channel(Some(progress_channel)).await;

        // 恢复执行
        ctx.set_pause(false, false);
        ctx.set_status(AgentTaskStatus::Running).await?;

        let iteration = ctx.current_iteration().await;
        ctx.send_progress(TaskProgressPayload::TaskResumed(TaskResumedPayload {
            task_id: task_id.to_string(),
            from_iteration: iteration,
            timestamp: chrono::Utc::now(),
        }))
        .await?;

        Ok(())
    }

    pub async fn cancel_task(
        &self,
        task_id: &str,
        _reason: Option<String>,
    ) -> TaskExecutorResult<()> {
        let ctx = self
            .active_tasks()
            .get(task_id)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or_else(|| TaskExecutorError::TaskNotFound(task_id.to_string()))?;

        ctx.abort();
        ctx.set_status(AgentTaskStatus::Cancelled).await?;

        ctx.send_progress(TaskProgressPayload::TaskCancelled(TaskCancelledPayload {
            task_id: task_id.to_string(),
            reason: "user_requested".to_string(),
            timestamp: chrono::Utc::now(),
        }))
        .await?;

        self.active_tasks().remove(task_id);

        Ok(())
    }

    pub async fn trigger_session_summary(
        &self,
        session_id: i64,
        model_override: Option<String>,
    ) -> TaskExecutorResult<Option<crate::agent::context::SummaryResult>> {
        use crate::agent::context::SessionSummarizer;
        use crate::llm::anthropic_types::{MessageContent, MessageParam, MessageRole};

        let persistence = self.agent_persistence();
        let mut executions = persistence
            .agent_executions()
            .list_recent_by_session(session_id, 1)
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

        // 转换消息格式
        let llm_messages: Vec<MessageParam> = messages
            .iter()
            .map(|msg| MessageParam {
                role: match msg.role.as_str() {
                    "user" => MessageRole::User,
                    _ => MessageRole::Assistant,
                },
                content: MessageContent::Text(msg.content.clone()),
            })
            .collect();

        let summarizer = SessionSummarizer::new(session_id, persistence.clone(), self.database());

        let model_id = model_override.unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string());

        let result = summarizer
            .summarize_now(&model_id, &llm_messages, &None)
            .await
            .map_err(|e| TaskExecutorError::InternalError(e.to_string()))?;

        persistence
            .agent_executions()
            .set_has_context(&latest_execution.execution_id, true)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        Ok(Some(result))
    }
    async fn restore_session_history(
        &self,
        ctx: &TaskContext,
        session_id: i64,
    ) -> TaskExecutorResult<()> {
        use crate::agent::persistence::MessageRole;
        use crate::llm::anthropic_types::{
            MessageContent, MessageParam, MessageRole as AnthropicRole,
        };

        let executions = self
            .agent_persistence()
            .agent_executions()
            .list_recent_by_session(session_id, 10)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        if executions.is_empty() {
            return Ok(());
        }

        let current_task_id = ctx.task_id.to_string();
        let mut all_messages = Vec::new();

        for execution in executions.iter().rev() {
            if execution.execution_id == current_task_id {
                continue;
            }

            let messages = self
                .agent_persistence()
                .execution_messages()
                .list_by_execution(&execution.execution_id)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

            if messages.is_empty() {
                continue;
            }

            for msg in messages {
                let role = match msg.role {
                    MessageRole::User => AnthropicRole::User,
                    MessageRole::Assistant => AnthropicRole::Assistant,
                    MessageRole::Tool | MessageRole::System => continue,
                };

                all_messages.push(MessageParam {
                    role,
                    content: MessageContent::Text(msg.content),
                });
            }
        }

        if !all_messages.is_empty() {
            ctx.restore_messages(all_messages).await?;
        }

        Ok(())
    }

    /// 规范化任务参数：空工作区时自动使用未分组会话
    async fn normalize_task_params(
        &self,
        mut params: ExecuteTaskParams,
    ) -> TaskExecutorResult<ExecuteTaskParams> {
        let needs_ungrouped =
            params.workspace_path.is_empty() || params.workspace_path.trim().is_empty();

        if needs_ungrouped || params.session_id <= 0 {
            let workspace_path = if needs_ungrouped {
                UNGROUPED_WORKSPACE_PATH.to_string()
            } else {
                params.workspace_path.clone()
            };

            let service = WorkspaceService::new(self.database());
            let session = service
                .ensure_active_session(&workspace_path)
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

            params.workspace_path = workspace_path;
            params.session_id = session.id;
        }

        Ok(params)
    }
}
