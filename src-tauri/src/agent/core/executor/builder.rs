/*!
 * TaskContext builder - creates a fresh TaskContext per user turn.
 *
 * New agent system design:
 * - No persisted "agent_executions" table.
 * - Session/message tables are the single source of truth for history.
 * - A task_id is runtime-only, used for streaming + cancellation.
 */

use std::sync::Arc;

use tauri::ipc::Channel;

use crate::agent::config::TaskExecutionConfig;
use crate::agent::core::context::TaskContext;
use crate::agent::core::executor::{ExecuteTaskParams, TaskExecutor};
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::tools::RunnableTool;
use crate::agent::types::TaskEvent;

impl TaskExecutor {
    pub async fn build_or_restore_context(
        &self,
        params: &ExecuteTaskParams,
        progress_channel: Option<Channel<TaskEvent>>,
    ) -> TaskExecutorResult<Arc<TaskContext>> {
        self.finish_running_task_for_session(params.session_id).await?;
        self.create_new_context(params, progress_channel).await
    }

    async fn finish_running_task_for_session(&self, session_id: i64) -> TaskExecutorResult<()> {
        let mut to_cancel = Vec::new();
        for entry in self.active_tasks().iter() {
            if entry.value().session_id == session_id {
                to_cancel.push(entry.key().clone());
            }
        }

        for task_id in to_cancel {
            if let Some((_, ctx)) = self.active_tasks().remove(&task_id) {
                ctx.abort();
            }
        }

        Ok(())
    }

    async fn create_new_context(
        &self,
        params: &ExecuteTaskParams,
        progress_channel: Option<Channel<TaskEvent>>,
    ) -> TaskExecutorResult<Arc<TaskContext>> {
        let task_id = format!("task_{}", uuid::Uuid::new_v4());

        let requested_workspace = params.workspace_path.clone();
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
            .map(|t| Arc::new(t) as Arc<dyn RunnableTool>)
            .collect::<Vec<_>>();

        let tool_registry = crate::agent::tools::create_tool_registry(
            "agent",
            effective.permissions,
            mcp_tools,
            self.vector_search_engine(),
        )
        .await;

        let ctx = TaskContext::new(
            task_id.clone(),
            params.session_id,
            params.user_prompt.clone(),
            "coder".to_string(),
            TaskExecutionConfig::default(),
            cwd,
            progress_channel,
            crate::agent::core::context::TaskContextDeps {
                tool_registry,
                repositories: Arc::clone(&self.database()),
                agent_persistence: Arc::clone(&self.agent_persistence()),
                checkpoint_service: self.checkpoint_service(),
                workspace_changes: self.workspace_changes(),
            },
        )
        .await?;

        let ctx = Arc::new(ctx);
        self.active_tasks()
            .insert(task_id.clone(), Arc::clone(&ctx));

        Ok(ctx)
    }
}

