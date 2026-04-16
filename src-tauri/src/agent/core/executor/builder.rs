/*!
 * AgentRunContext builder - creates a fresh AgentRunContext per user turn.
 *
 * New agent system design:
 * - No persisted "agent_executions" table.
 * - Session/message tables are the single source of truth for history.
 * - A run_id is runtime-only, used for streaming + cancellation.
 */

use std::sync::Arc;

use tauri::ipc::Channel;

use crate::agent::agents::AgentConfigLoader;
use crate::agent::command_system::CommandConfigLoader;
use crate::agent::common::truncate_chars;
use crate::agent::config::AgentRunConfig;
use crate::agent::core::context::AgentRunContext;
use crate::agent::core::executor::{AgentRunExecutor, ExecuteRunParams};
use crate::agent::error::{AgentRunError, AgentRunResult};
use crate::agent::types::AgentRunEvent;

const MAX_ACTIVE_TASKS_GLOBAL: usize = 5;

impl AgentRunExecutor {
    pub async fn build_or_restore_context(
        &self,
        params: &ExecuteRunParams,
        progress_channel: Option<Channel<AgentRunEvent>>,
    ) -> AgentRunResult<Arc<AgentRunContext>> {
        self.finish_running_run_for_thread(params.thread_id).await?;
        self.enforce_task_limits().await?;
        self.create_new_context(params, progress_channel).await
    }

    async fn finish_running_run_for_thread(&self, thread_id: i64) -> AgentRunResult<()> {
        let mut to_cancel = Vec::new();
        for entry in self.active_runs().iter() {
            if entry.value().thread_id == thread_id {
                to_cancel.push(entry.key().clone());
            }
        }

        for run_id in to_cancel {
            if let Err(err) = self
                .cancel_run(&run_id, Some("superseded by new user message".to_string()))
                .await
            {
                tracing::warn!(
                    "Failed to cancel superseded running task '{}': {}",
                    run_id,
                    err
                );
            }
        }

        Ok(())
    }

    async fn create_new_context(
        &self,
        params: &ExecuteRunParams,
        progress_channel: Option<Channel<AgentRunEvent>>,
    ) -> AgentRunResult<Arc<AgentRunContext>> {
        let run_id = format!("task_{}", uuid::Uuid::new_v4());

        let requested_workspace = params.workspace_path.clone();
        let workspace_root =
            match tokio::fs::canonicalize(std::path::PathBuf::from(&requested_workspace)).await {
                Ok(path) => path,
                Err(err) => {
                    tracing::warn!(
                        "Failed to canonicalize requested workspace '{}': {}",
                        requested_workspace,
                        err
                    );
                    std::path::PathBuf::from(&requested_workspace)
                }
            };
        let cwd = workspace_root.to_string_lossy().to_string();

        let thread = self
            .agent_persistence()
            .threads()
            .get(params.thread_id)
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?
            .ok_or_else(|| {
                AgentRunError::StatePersistenceFailed(format!(
                    "thread {} not found",
                    params.thread_id
                ))
            })?;
        let mut agent_type = match params.agent_type.clone().filter(|v| !v.trim().is_empty()) {
            Some(agent_type) => agent_type,
            None => thread.agent_type.clone(),
        };

        let effective = self
            .settings_manager()
            .get_effective_settings(Some(workspace_root.clone()))
            .await
            .map_err(|e| AgentRunError::ConfigurationError(e.to_string()))?;

        let agent_configs = AgentConfigLoader::load_for_workspace(&workspace_root)
            .await
            .map_err(|e| {
                AgentRunError::ConfigurationError(format!("Failed to load agent configs: {e}"))
            })?;

        // Get tool filter for the requested agent type.
        // If not found, fall back to "coder" config (the default primary agent).
        let agent_tool_filter = if let Some(config) = agent_configs.get(&agent_type) {
            Some(config.tool_filter.clone())
        } else {
            if agent_type != "coder" {
                tracing::debug!(
                    "Agent type '{}' not found, falling back to 'coder'",
                    agent_type
                );
            }
            agent_configs
                .get("coder")
                .map(|cfg| cfg.tool_filter.clone())
        };

        // Initialize Skill system: automatically discover global and workspace skills
        let skill_manager = {
            let manager = Arc::new(crate::agent::skill::SkillManager::new());

            // Get global skills directory
            let global_skills_dir = crate::config::paths::skills_dir();

            // Automatically discover skills
            if let Err(e) = manager
                .discover_skills(Some(&global_skills_dir), Some(&workspace_root))
                .await
            {
                tracing::warn!("Failed to discover skills: {}", e);
            } else {
                let count = manager.list_all().len();
                if count > 0 {
                    tracing::info!("Discovered {} skills", count);
                }
            }

            Some(manager)
        };

        let tool_registry = crate::agent::tools::create_tool_registry(
            "agent",
            effective.permissions,
            agent_tool_filter,
            self.tool_confirmations(),
            Vec::new(),
            self.vector_search_engine(),
            Some(self.lsp_manager()),
            skill_manager,
        )
        .await;

        // user_prompt here is the text sent to the LLM for this turn.
        // UI storage is handled separately in the lifecycle layer.
        let raw_user_prompt = params.user_prompt.clone();
        tracing::debug!(
            "ExecuteTask raw user_prompt: {}",
            truncate_chars(&raw_user_prompt, 400)
        );

        // Process command_id if present (render built-in command template with raw user input)
        let user_prompt = if let Some(cmd_id) = params.command_id.as_deref() {
            if let Some(cmd_config) = CommandConfigLoader::get(cmd_id) {
                let rendered = CommandConfigLoader::render(cmd_config, &raw_user_prompt);
                if params.agent_type.is_none() {
                    if let Some(command_agent) =
                        rendered.agent.as_ref().filter(|v| !v.trim().is_empty())
                    {
                        agent_type = command_agent.clone();
                    }
                }
                tracing::info!("Rendered built-in command '{}' template", cmd_id);
                rendered.prompt
            } else {
                tracing::warn!("Command '{}' not found, using raw prompt", cmd_id);
                raw_user_prompt.clone()
            }
        } else {
            raw_user_prompt.clone()
        };

        let ctx = AgentRunContext::new(crate::agent::core::context::AgentRunContextInit {
            run_id: run_id.clone(),
            thread_id: params.thread_id,
            user_prompt,
            agent_type,
            config: AgentRunConfig::default(),
            workspace_path: cwd,
            emit_task_events: true,
            progress_channel,
            deps: crate::agent::core::context::AgentRunContextDeps {
                tool_registry,
                repositories: Arc::clone(&self.database()),
                agent_persistence: Arc::clone(&self.agent_persistence()),
                checkpoint_service: self.checkpoint_service(),
                workspace_changes: self.workspace_changes(),
                subagent_runner: Arc::new(self.clone()),
                settings_manager: self.settings_manager(),
            },
        })
        .await?;

        let ctx = Arc::new(ctx);
        self.active_runs().insert(run_id.clone(), Arc::clone(&ctx));

        Ok(ctx)
    }

    async fn enforce_task_limits(&self) -> AgentRunResult<()> {
        let global_count = self
            .active_runs()
            .iter()
            .filter(|entry| entry.value().emits_task_events())
            .count();

        if global_count >= MAX_ACTIVE_TASKS_GLOBAL {
            return Err(AgentRunError::TooManyActiveTasksGlobal {
                current: global_count,
                limit: MAX_ACTIVE_TASKS_GLOBAL,
            });
        }

        Ok(())
    }
}
