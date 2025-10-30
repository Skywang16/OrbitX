/*!
 * 任务生命周期管理
 */

use std::sync::Arc;

use tauri::ipc::Channel;
use tokio::task;
use tracing::{error, info};

use crate::agent::core::context::TaskContext;
use crate::agent::core::executor::{ExecuteTaskParams, TaskExecutor};
use crate::agent::core::status::AgentTaskStatus;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::events::{
    FinishPayload, TaskCancelledPayload, TaskCompletedPayload, TaskCreatedPayload,
    TaskErrorPayload, TaskPausedPayload, TaskProgressPayload, TaskResumedPayload,
    TaskStartedPayload,
};

impl TaskExecutor {
    /// 执行新任务或恢复现有任务
    ///
    /// # 返回值
    /// 返回 Arc<TaskContext>，调用者可以持有引用而不需要clone
    pub async fn execute_task(
        &self,
        params: ExecuteTaskParams,
        progress_channel: Channel<TaskProgressPayload>,
    ) -> TaskExecutorResult<Arc<TaskContext>> {
        info!(
            "Starting task execution for conversation_id={}, model={}",
            params.conversation_id, params.model_id
        );

        // 构建或恢复上下文
        let ctx = self
            .build_or_restore_context(&params, Some(progress_channel))
            .await?;

        // 发送TaskCreated事件
        ctx.send_progress(TaskProgressPayload::TaskCreated(TaskCreatedPayload {
            task_id: ctx.task_id.to_string(),
            conversation_id: ctx.conversation_id,
            user_prompt: params.user_prompt.clone(),
        }))
        .await?;

        // 初始化UI track
        ctx.initialize_ui_track(&params.user_prompt).await?;

        // 构建prompts
        let (system_prompt, user_prompt) = self
            .prompt_orchestrator()
            .build_task_prompts(
                params.conversation_id,
                ctx.task_id.to_string(),
                &params.user_prompt,
                params.cwd.as_deref(),
                &ctx.tool_registry(),
            )
            .await?;

        // 设置初始prompts
        ctx.set_initial_prompts(system_prompt, user_prompt).await?;

        // 设置状态为Running
        ctx.set_status(AgentTaskStatus::Running).await?;

        // 发送TaskStarted事件
        ctx.send_progress(TaskProgressPayload::TaskStarted(TaskStartedPayload {
            task_id: ctx.task_id.to_string(),
            iteration: 0,
        }))
        .await?;

        // 克隆必要的数据用于spawn
        let executor = self.clone();
        let ctx_for_spawn = Arc::clone(&ctx);
        let model_id = params.model_id.clone();

        // Spawn ReAct循环
        task::spawn(async move {
            if let Err(e) = executor.run_task_loop(ctx_for_spawn, model_id).await {
                error!("Task execution failed: {}", e);
            }
        });

        Ok(ctx)
    }

    /// 执行任务循环（内部方法）
    /// 
    /// 零成本抽象：直接传递self，编译器会内联所有trait方法调用
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

    /// 暂停任务
    pub async fn pause_task(&self, task_id: &str, abort_current_step: bool) -> TaskExecutorResult<()> {
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

        info!("Task {} paused", task_id);
        Ok(())
    }

    /// 恢复任务
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

        info!("Task {} resumed from iteration {}", task_id, iteration);
        Ok(())
    }

    /// 取消任务
    pub async fn cancel_task(&self, task_id: &str, _reason: Option<String>) -> TaskExecutorResult<()> {
        let ctx = self
            .active_tasks()
            .get(task_id)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or_else(|| TaskExecutorError::TaskNotFound(task_id.to_string()))?;

        // 中止执行
        ctx.abort();
        ctx.set_status(AgentTaskStatus::Cancelled).await?;

        ctx.send_progress(TaskProgressPayload::TaskCancelled(TaskCancelledPayload {
            task_id: task_id.to_string(),
            reason: "user_requested".to_string(),
            timestamp: chrono::Utc::now(),
        }))
        .await?;

        // 从active_tasks移除
        self.active_tasks().remove(task_id);

        info!("Task {} cancelled", task_id);
        Ok(())
    }

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

    /// 触发会话摘要生成
    pub async fn trigger_conversation_summary(
        &self,
        conversation_id: i64,
        model_override: Option<String>,
    ) -> TaskExecutorResult<Option<crate::agent::context::SummaryResult>> {
        use crate::agent::context::ConversationSummarizer;
        use crate::llm::anthropic_types::{MessageContent, MessageParam, MessageRole};

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

        let summarizer = ConversationSummarizer::new(
            conversation_id,
            persistence.clone(),
            self.database(),
        );

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

        persistence
            .conversations()
            .touch(conversation_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        Ok(Some(result))
    }

    /// 继续对话（在已有conversation中添加新的user message）
    pub async fn continue_conversation(
        &self,
        conversation_id: i64,
        user_prompt: String,
        model_id: String,
        progress_channel: Channel<TaskProgressPayload>,
    ) -> TaskExecutorResult<Arc<TaskContext>> {
        // 获取或创建context
        let ctx = if let Some(entry) = self.conversation_contexts().get(&conversation_id) {
            let ctx = Arc::clone(entry.value());

            // 重置cancellation token（允许继续执行）
            ctx.reset_cancellation().await;

            // 更新progress channel
            ctx.set_progress_channel(Some(progress_channel)).await;

            ctx
        } else {
            // 创建新任务
            let params = ExecuteTaskParams {
                conversation_id,
                user_prompt: user_prompt.clone(),
                chat_mode: "agent".to_string(),
                model_id: model_id.clone(),
                cwd: None,
                has_context: false,
            };

            self.build_or_restore_context(&params, Some(progress_channel))
                .await?
        };

        // 开始新的turn
        ctx.begin_followup_turn(&user_prompt).await?;

        // 添加user message
        ctx.add_user_message(user_prompt).await;

        // 设置状态为Running
        ctx.set_status(AgentTaskStatus::Running).await?;

        // Spawn执行循环
        let executor = self.clone();
        let ctx_for_spawn = Arc::clone(&ctx);
        task::spawn(async move {
            if let Err(e) = executor.run_task_loop(ctx_for_spawn, model_id).await {
                error!("Continue conversation failed: {}", e);
            }
        });

        Ok(ctx)
    }

}
