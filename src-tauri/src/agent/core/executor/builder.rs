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

use crate::agent::agents::AgentConfigLoader;
use crate::agent::config::TaskExecutionConfig;
use crate::agent::core::context::TaskContext;
use crate::agent::core::executor::{ExecuteTaskParams, TaskExecutor};
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::types::TaskEvent;

impl TaskExecutor {
    pub async fn build_or_restore_context(
        &self,
        params: &ExecuteTaskParams,
        progress_channel: Option<Channel<TaskEvent>>,
    ) -> TaskExecutorResult<Arc<TaskContext>> {
        self.finish_running_task_for_session(params.session_id)
            .await?;
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
            let _ = self
                .cancel_task(&task_id, Some("superseded by new user message".to_string()))
                .await;
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

        let session = self
            .agent_persistence()
            .sessions()
            .get(params.session_id)
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?
            .ok_or_else(|| {
                TaskExecutorError::StatePersistenceFailed(format!(
                    "session {} not found",
                    params.session_id
                ))
            })?;
        let agent_type = params
            .agent_type
            .clone()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| session.agent_type.clone());

        let effective = self
            .settings_manager()
            .get_effective_settings(Some(workspace_root.clone()))
            .await
            .map_err(|e| TaskExecutorError::ConfigurationError(e.to_string()))?;

        let agent_configs = AgentConfigLoader::load_for_workspace(&workspace_root)
            .await
            .map_err(|e| {
                tracing::warn!("Failed to load agent configs: {}, using defaults", e);
            })
            .unwrap_or_default();

        // Get tool filter for the requested agent type.
        // If not found, fall back to "coder" config (the default primary agent).
        let agent_tool_filter = agent_configs
            .get(&agent_type)
            .or_else(|| {
                if agent_type != "coder" {
                    tracing::debug!("Agent type '{}' not found, falling back to 'coder'", agent_type);
                }
                agent_configs.get("coder")
            })
            .map(|cfg| cfg.tool_filter.clone());

        let tool_registry = crate::agent::tools::create_tool_registry(
            "agent",
            effective.permissions,
            agent_tool_filter,
            self.tool_confirmations(),
            Vec::new(),
            self.vector_search_engine(),
        )
        .await;

        let ctx = TaskContext::new(
            task_id.clone(),
            params.session_id,
            params.user_prompt.clone(),
            agent_type,
            TaskExecutionConfig::default(),
            cwd,
            true,
            progress_channel,
            crate::agent::core::context::TaskContextDeps {
                tool_registry,
                repositories: Arc::clone(&self.database()),
                agent_persistence: Arc::clone(&self.agent_persistence()),
                checkpoint_service: self.checkpoint_service(),
                workspace_changes: self.workspace_changes(),
                subtask_runner: Arc::new(self.clone()),
            },
        )
        .await?;

        let ctx = Arc::new(ctx);
        self.active_tasks()
            .insert(task_id.clone(), Arc::clone(&ctx));

        Ok(ctx)
    }
}
