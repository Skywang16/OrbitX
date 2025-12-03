/*!
 * TaskContext构建器 - 从数据库恢复或创建新任务
 */

use std::sync::Arc;

use chrono::Utc;
use tauri::ipc::Channel;

use crate::agent::config::TaskExecutionConfig;
use crate::agent::core::context::TaskContext;
use crate::agent::core::executor::{ExecuteTaskParams, TaskExecutor};
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::events::TaskProgressPayload;
use crate::agent::persistence::{AgentExecution, ExecutionStatus};

impl TaskExecutor {
    /// 构建或恢复TaskContext
    ///
    /// # 返回值
    /// 返回 Arc<TaskContext> 而非 TaskContext，避免后续clone
    pub async fn build_or_restore_context(
        &self,
        params: &ExecuteTaskParams,
        progress_channel: Option<Channel<TaskProgressPayload>>,
    ) -> TaskExecutorResult<Arc<TaskContext>> {
        let conversation_id = params.conversation_id;

        // 尝试从数据库恢复
        if let Some(existing_ctx) = self.try_restore_from_db(conversation_id).await? {

            // 更新progress channel
            existing_ctx.set_progress_channel(progress_channel).await;

            // 存储到active_tasks
            self.active_tasks()
                .insert(existing_ctx.task_id.to_string(), Arc::clone(&existing_ctx));

            return Ok(existing_ctx);
        }

        // 创建新任务
        self.create_new_context(params, progress_channel).await
    }

    /// 尝试从数据库恢复任务
    async fn try_restore_from_db(
        &self,
        conversation_id: i64,
    ) -> TaskExecutorResult<Option<Arc<TaskContext>>> {
        // 先检查内存中是否已有
        if let Some(entry) = self.conversation_contexts().get(&conversation_id) {
            return Ok(Some(Arc::clone(entry.value())));
        }

        // 从数据库查询最近的执行记录
        let executions = self
            .agent_persistence()
            .agent_executions()
            .list_recent_by_conversation(conversation_id, 1)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let Some(execution) = executions.into_iter().next() else {
            return Ok(None);
        };

        // 只恢复未完成的任务
        if matches!(
            execution.status,
            ExecutionStatus::Completed | ExecutionStatus::Cancelled | ExecutionStatus::Error
        ) {
            return Ok(None);
        }


        // 构建TaskContext
        let ctx = self.build_context_from_execution(execution, None).await?;

        // 恢复消息历史
        self.restore_messages_for_context(&ctx).await?;

        // 恢复UI track
        ctx.restore_ui_track().await?;

        let ctx_arc = Arc::new(ctx);

        // 缓存到内存
        self.conversation_contexts()
            .insert(conversation_id, Arc::clone(&ctx_arc));

        Ok(Some(ctx_arc))
    }

    /// 创建新的TaskContext
    async fn create_new_context(
        &self,
        params: &ExecuteTaskParams,
        progress_channel: Option<Channel<TaskProgressPayload>>,
    ) -> TaskExecutorResult<Arc<TaskContext>> {
        let task_id = format!("exec_{}", uuid::Uuid::new_v4());
        let _cwd = params.cwd.clone().unwrap_or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "/".to_string())
        });

        // 创建execution记录
        let execution = AgentExecution {
            id: 0, // 由数据库自动生成
            execution_id: task_id.clone(),
            conversation_id: params.conversation_id,
            user_request: params.user_prompt.clone(),
            system_prompt_used: String::new(),
            execution_config: Some(serde_json::to_string(&TaskExecutionConfig::default()).unwrap()),
            has_conversation_context: params.has_context,
            status: ExecutionStatus::Running,
            current_iteration: 0,
            error_count: 0,
            max_iterations: TaskExecutionConfig::default().max_iterations as i64,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost: 0.0,
            context_tokens: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
        };

        // 持久化execution记录
        let created_execution = self
            .agent_persistence()
            .agent_executions()
            .create(
                execution.conversation_id,
                &execution.user_request,
                &execution.system_prompt_used,
                execution.execution_config.as_deref(),
                execution.has_conversation_context,
                execution.max_iterations,
            )
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        // 构建TaskContext
        let ctx = self
            .build_context_from_execution(created_execution, progress_channel)
            .await?;

        let ctx_arc = Arc::new(ctx);

        // 缓存到内存
        self.active_tasks()
            .insert(task_id.clone(), Arc::clone(&ctx_arc));
        self.conversation_contexts()
            .insert(params.conversation_id, Arc::clone(&ctx_arc));

        Ok(ctx_arc)
    }

    /// 从AgentExecution构建TaskContext
    async fn build_context_from_execution(
        &self,
        execution: AgentExecution,
        progress_channel: Option<Channel<TaskProgressPayload>>,
    ) -> TaskExecutorResult<TaskContext> {
        let config = if let Some(config_str) = &execution.execution_config {
            serde_json::from_str(config_str).unwrap_or_else(|_| TaskExecutionConfig::default())
        } else {
            TaskExecutionConfig::default()
        };

        let cwd = self
            .get_workspace_cwd(execution.conversation_id)
            .await
            .or_else(|| {
                std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .ok()
            })
            .unwrap_or_else(|| "/".to_string());

        let tool_registry = crate::agent::tools::create_tool_registry("agent").await;

        TaskContext::new(
            execution,
            config,
            cwd,
            tool_registry,
            progress_channel,
            Arc::clone(&self.database()),
            Arc::clone(&self.agent_persistence()),
            Arc::clone(&self.ui_persistence()),
        )
        .await
    }

    /// 恢复消息历史
    async fn restore_messages_for_context(&self, ctx: &TaskContext) -> TaskExecutorResult<()> {
        let messages = self
            .agent_persistence()
            .execution_messages()
            .list_by_execution(&ctx.task_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        if messages.is_empty() {
            return Ok(());
        }

        // 转换为Anthropic MessageParam格式
        let anthropic_messages = messages
            .into_iter()
            .filter_map(|msg| {
                use crate::agent::persistence::MessageRole;
                use crate::llm::anthropic_types::{
                    MessageContent, MessageParam, MessageRole as AnthropicRole,
                };

                let role = match msg.role {
                    MessageRole::User => AnthropicRole::User,
                    MessageRole::Assistant => AnthropicRole::Assistant,
                    _ => return None,
                };

                Some(MessageParam {
                    role,
                    content: MessageContent::Text(msg.content),
                })
            })
            .collect::<Vec<_>>();

        ctx.restore_messages(anthropic_messages).await?;

        Ok(())
    }

    async fn get_workspace_cwd(&self, conversation_id: i64) -> Option<String> {
        self.agent_persistence()
            .conversations()
            .get(conversation_id)
            .await
            .ok()
            .flatten()
            .and_then(|conv| conv.workspace_path)
    }
}
