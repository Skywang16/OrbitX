/*!
 * TaskContext构建器 - 每次创建新任务，从数据库加载历史
 *
 * 设计原则：
 * - 每次发消息创建全新的 TaskContext（状态天然干净）
 * - 历史消息从 DB 加载，不依赖内存缓存
 * - 应用重启也不会丢失上下文
 */

use chrono::Utc;
use tauri::ipc::Channel;

use crate::agent::config::TaskExecutionConfig;
use crate::agent::core::context::TaskContext;
use crate::agent::core::executor::{ExecuteTaskParams, TaskExecutor};
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::persistence::{AgentExecution, ExecutionStatus};
use crate::agent::types::TaskEvent;
use std::sync::Arc;

impl TaskExecutor {
    /// 构建新的 TaskContext
    ///
    /// 每次都创建新的 Context，从 DB 加载历史消息。
    /// 不再复用内存中的 Context，避免状态污染问题。
    pub async fn build_or_restore_context(
        &self,
        params: &ExecuteTaskParams,
        progress_channel: Option<Channel<TaskEvent>>,
    ) -> TaskExecutorResult<Arc<TaskContext>> {
        self.finish_running_task_for_session(params.session_id)
            .await?;

        // 创建全新的任务
        self.create_new_context(params, progress_channel).await
    }

    /// 结束会话中正在运行的任务
    async fn finish_running_task_for_session(&self, session_id: i64) -> TaskExecutorResult<()> {
        // 从数据库查询最近的执行记录
        let executions = self
            .agent_persistence()
            .agent_executions()
            .list_recent_by_session(session_id, 1)
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
        progress_channel: Option<Channel<TaskEvent>>,
    ) -> TaskExecutorResult<Arc<TaskContext>> {
        let task_id = format!("exec_{}", uuid::Uuid::new_v4());

        // 创建execution记录
        let execution = AgentExecution {
            id: 0, // 由数据库自动生成
            execution_id: task_id.clone(),
            session_id: params.session_id,
            user_request: params.user_prompt.clone(),
            system_prompt_used: String::new(),
            execution_config: Some(serde_json::to_string(&TaskExecutionConfig::default()).unwrap()),
            has_conversation_context: false, // 由后端自动检测
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

        // 持久化execution记录，传入已生成的 task_id 确保一致性
        let created_execution = self
            .agent_persistence()
            .agent_executions()
            .create(
                &task_id,
                execution.session_id,
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
            .build_context_from_execution(
                created_execution,
                params.workspace_path.clone(),
                progress_channel,
            )
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
        workspace_path: String,
        progress_channel: Option<Channel<TaskEvent>>,
    ) -> TaskExecutorResult<TaskContext> {
        let config = if let Some(config_str) = &execution.execution_config {
            serde_json::from_str(config_str).unwrap_or_else(|_| TaskExecutionConfig::default())
        } else {
            TaskExecutionConfig::default()
        };

        let requested_workspace = workspace_path;
        let workspace_root =
            tokio::fs::canonicalize(std::path::PathBuf::from(&requested_workspace))
                .await
                .unwrap_or_else(|_| std::path::PathBuf::from(&requested_workspace));
        let cwd = workspace_root.to_string_lossy().to_string();

        let effective = self
            .settings_manager()
            .get_effective_settings(Some(workspace_root.clone()))
            .await
            .map_err(|e| TaskExecutorError::ConfigurationError(e.to_string()))?;

        let workspace_settings = self
            .settings_manager()
            .get_workspace_settings(&workspace_root)
            .await
            .map_err(|e| TaskExecutorError::ConfigurationError(e.to_string()))?;
        let _ = self
            .mcp_registry()
            .init_workspace_servers(&workspace_root, &effective, workspace_settings.as_ref())
            .await;

        let mcp_tools = self
            .mcp_registry()
            .get_tools_for_workspace(&cwd)
            .into_iter()
            .map(|t| Arc::new(t) as Arc<dyn crate::agent::tools::RunnableTool>)
            .collect::<Vec<_>>();

        let tool_registry = crate::agent::tools::create_tool_registry(
            "agent",
            effective.permissions,
            mcp_tools,
            self.vector_search_engine(),
        )
        .await;

        TaskContext::new(
            execution,
            config,
            cwd,
            tool_registry,
            progress_channel,
            Arc::clone(&self.database()),
            Arc::clone(&self.agent_persistence()),
            self.checkpoint_service(),
            self.workspace_changes(),
        )
        .await
    }
}
