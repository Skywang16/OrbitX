/*!
 * TaskContext构建器 - 每次创建新任务，从数据库加载历史
 *
 * 设计原则：
 * - 每次发消息创建全新的 TaskContext（状态天然干净）
 * - 历史消息从 DB 加载，不依赖内存缓存
 * - 应用重启也不会丢失上下文
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
    /// 构建新的 TaskContext
    ///
    /// 每次都创建新的 Context，从 DB 加载历史消息。
    /// 不再复用内存中的 Context，避免状态污染问题。
    pub async fn build_or_restore_context(
        &self,
        params: &ExecuteTaskParams,
        progress_channel: Option<Channel<TaskProgressPayload>>,
    ) -> TaskExecutorResult<Arc<TaskContext>> {
        let conversation_id = params.conversation_id;

        // 检查是否有正在运行的任务，如果有则标记为完成
        self.finish_running_task_for_conversation(conversation_id)
            .await?;

        // 创建全新的任务
        self.create_new_context(params, progress_channel).await
    }

    /// 结束会话中正在运行的任务
    async fn finish_running_task_for_conversation(
        &self,
        conversation_id: i64,
    ) -> TaskExecutorResult<()> {
        // 从数据库查询最近的执行记录
        let executions = self
            .agent_persistence()
            .agent_executions()
            .list_recent_by_conversation(conversation_id, 1)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        if let Some(execution) = executions.into_iter().next() {
            // 如果任务还在运行，标记为完成
            if matches!(execution.status, ExecutionStatus::Running) {
                self.agent_persistence()
                    .agent_executions()
                    .mark_finished(&execution.execution_id, ExecutionStatus::Completed)
                    .await
                    .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

                // 从 active_tasks 中移除
                self.active_tasks().remove(&execution.execution_id);
            }
        }

        Ok(())
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
            execution_config: Some(
                serde_json::to_string(&TaskExecutionConfig::default()).unwrap(),
            ),
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

        // 只存储到 active_tasks（用于中断），不再存储到 conversation_contexts
        self.active_tasks()
            .insert(task_id.clone(), Arc::clone(&ctx_arc));

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

    /// 恢复消息历史（用于 has_context=true 时加载历史对话）
    #[allow(dead_code)]
    pub(crate) async fn restore_messages_for_context(
        &self,
        ctx: &TaskContext,
        execution_id: &str,
    ) -> TaskExecutorResult<()> {
        let messages = self
            .agent_persistence()
            .execution_messages()
            .list_by_execution(execution_id)
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
