//! Prompt Orchestrator - Build task prompts

use std::path::Path;
use std::sync::Arc;

use crate::agent::agents::AgentConfigLoader;
use crate::agent::context::ProjectContextLoader;
use crate::agent::error::{AgentRunError, AgentRunResult};
use crate::agent::prompt::{BuiltinPrompts, PromptBuilder, SystemPromptParts};
use crate::agent::rollout::replay_rollout;
use crate::agent::tools::ToolRegistry;
use crate::settings::SettingsManager;
use crate::storage::repositories::AppPreferences;
use crate::storage::{DatabaseManager, UnifiedCache};

pub struct PromptOrchestrator {
    cache: Arc<UnifiedCache>,
    database: Arc<DatabaseManager>,
    settings_manager: Arc<SettingsManager>,
}

#[derive(Debug, Clone, Default)]
pub struct TaskPrompts {
    pub instructions: String,
    pub developer_context: Vec<String>,
    pub user_prompt: String,
}

impl PromptOrchestrator {
    pub fn new(
        cache: Arc<UnifiedCache>,
        database: Arc<DatabaseManager>,
        settings_manager: Arc<SettingsManager>,
    ) -> Self {
        Self {
            cache,
            database,
            settings_manager,
        }
    }

    async fn load_rules(
        &self,
        workspace_path: &str,
    ) -> AgentRunResult<(Option<String>, Option<String>)> {
        let effective = self
            .settings_manager
            .get_effective_settings(Some(std::path::PathBuf::from(workspace_path)))
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        let global_rules = {
            let rules = effective.rules_content.trim();
            if rules.is_empty() {
                None
            } else {
                Some(rules.to_string())
            }
        };

        let prefs = AppPreferences::new(&self.database);
        let project_rules = prefs
            .get("workspace.project_rules")
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        if let Err(err) = self.cache.set_global_rules(global_rules.clone()).await {
            tracing::warn!("Failed to sync global rules cache: {}", err);
        }
        if let Err(err) = self.cache.set_project_rules(project_rules.clone()).await {
            tracing::warn!("Failed to sync project rules cache: {}", err);
        }

        Ok((global_rules, project_rules))
    }

    async fn has_agent_thread_messages(
        &self,
        thread_id: i64,
        agent_type: &str,
    ) -> AgentRunResult<bool> {
        let rollout_path: Option<String> =
            sqlx::query_scalar("SELECT rollout_path FROM threads WHERE id = ?")
                .bind(thread_id)
                .fetch_optional(self.database.pool())
                .await
                .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        let Some(rollout_path) = rollout_path else {
            return Ok(false);
        };

        let replayed = replay_rollout(Path::new(&rollout_path))
            .await
            .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

        Ok(replayed
            .messages
            .iter()
            .any(|message| message.agent_type == agent_type))
    }

    fn get_reminder(&self, agent_type: &str, has_plan_history: bool) -> Option<String> {
        if agent_type == "plan" {
            return Some(BuiltinPrompts::reminder_plan_mode().to_string());
        }

        if agent_type == "coder" && has_plan_history {
            return Some(BuiltinPrompts::reminder_coder_with_plan().to_string());
        }

        None
    }

    fn get_directory_listing(workspace_path: &str) -> Option<String> {
        let path = Path::new(workspace_path);
        let read_dir = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(err) => {
                tracing::warn!(
                    "Failed to read top-level directory listing for '{}': {}",
                    workspace_path,
                    err
                );
                return None;
            }
        };

        let mut entries = Vec::new();
        for entry in read_dir {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    tracing::warn!(
                        "Failed to read directory entry in '{}': {}",
                        workspace_path,
                        err
                    );
                    continue;
                }
            };

            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "~" {
                continue;
            }

            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(err) => {
                    tracing::warn!(
                        "Failed to read file type for '{}' entry '{}': {}",
                        workspace_path,
                        name,
                        err
                    );
                    continue;
                }
            };

            if file_type.is_dir() {
                entries.push(format!("{name}/"));
            } else {
                entries.push(name);
            }
        }

        entries.sort();
        (!entries.is_empty()).then(|| {
            format!(
                "Directory listing (top-level):\n```\n{}\n```",
                entries.join("\n")
            )
        })
    }

    fn get_git_info(workspace_path: &str) -> Option<String> {
        let path = Path::new(workspace_path);
        if !path.join(".git").exists() {
            return None;
        }

        let mut info_parts = Vec::new();

        // Get current branch
        if let Some(output) =
            run_git_command(workspace_path, &["rev-parse", "--abbrev-ref", "HEAD"])
        {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            info_parts.push(format!("Git branch: {branch}"));
        }

        // Get short status (dirty/clean)
        if let Some(output) = run_git_command(workspace_path, &["status", "--porcelain"]) {
            let status = String::from_utf8_lossy(&output.stdout);
            let changed_count = status.lines().count();
            if changed_count > 0 {
                info_parts.push(format!("Git status: {changed_count} file(s) changed"));
            } else {
                info_parts.push("Git status: clean".to_string());
            }
        }

        if info_parts.is_empty() {
            None
        } else {
            Some(info_parts.join("\n"))
        }
    }

    pub async fn build_task_prompts(
        &self,
        thread_id: i64,
        _task_id: String,
        user_prompt: &str,
        agent_type: &str,
        workspace_path: &str,
        _tool_registry: &ToolRegistry,
        model_id: Option<&str>,
    ) -> AgentRunResult<TaskPrompts> {
        let cwd = workspace_path;

        // Load agent configuration
        let agent_configs = AgentConfigLoader::load_for_workspace(&std::path::PathBuf::from(cwd))
            .await
            .map_err(|err| AgentRunError::ConfigurationError(err.to_string()))?;

        let agent_cfg = agent_configs.get(agent_type);

        // Load rules
        let (global_rules, project_rules) = self.load_rules(workspace_path).await?;

        // Load project context
        let loader = ProjectContextLoader::new(cwd);
        let project_context = loader.load_with_preference(project_rules.as_deref()).await;

        // Build custom instructions
        let mut custom_parts = Vec::new();
        if let Some(ctx) = project_context {
            custom_parts.push(ctx.format_for_prompt());
        }
        if let Some(rules) = global_rules {
            custom_parts.push(rules);
        }

        let custom_instructions = if custom_parts.is_empty() {
            None
        } else {
            Some(custom_parts.join("\n\n"))
        };

        // Get reminder
        let has_plan_history = self.has_agent_thread_messages(thread_id, "plan").await?;
        let reminder = self.get_reminder(agent_type, has_plan_history);

        // Build environment info with directory listing and git status
        let mut prompt_builder = PromptBuilder::new(Some(workspace_path.to_string()));
        let dir_listing = Self::get_directory_listing(cwd);
        let git_info = Self::get_git_info(cwd);
        let env_info =
            prompt_builder.build_env_info(Some(cwd), dir_listing.as_deref(), git_info.as_deref());

        let agent_prompt = agent_cfg.map(|cfg| cfg.system_prompt.clone());

        // Model-specific prompt profile (workspace override > builtin > generic fallback)
        let model_profile = if let Some(mid) = model_id {
            let family = super::model_harness::ModelFamily::detect(mid);
            let profile_key = family.profile_key();
            tracing::debug!(
                "Model harness: model_id={}, family={}, profile={}",
                mid,
                family.name(),
                profile_key
            );

            match prompt_builder.get_model_profile_prompt(profile_key).await {
                Some(profile) => Some(profile),
                None if profile_key != "generic" => {
                    tracing::warn!(
                        "Missing model profile '{}' for model '{}', fallback to generic",
                        profile_key,
                        mid
                    );
                    prompt_builder.get_model_profile_prompt("generic").await
                }
                None => None,
            }
        } else {
            tracing::debug!("Model harness: no model_id, using generic profile");
            prompt_builder.get_model_profile_prompt("generic").await
        };

        let layers = prompt_builder.build_prompt_layers(SystemPromptParts {
            agent_prompt,
            model_profile,
            env_info: Some(env_info),
            reminder,
            custom_instructions,
            user_system: None,
        });

        Ok(TaskPrompts {
            instructions: layers.instructions,
            developer_context: layers.developer_context,
            user_prompt: user_prompt.to_string(),
        })
    }
}

fn run_git_command(workspace_path: &str, args: &[&str]) -> Option<std::process::Output> {
    use std::process::Command;

    match Command::new("git")
        .args(args)
        .current_dir(workspace_path)
        .output()
    {
        Ok(output) if output.status.success() => Some(output),
        Ok(output) => {
            tracing::warn!(
                "Git command failed in '{}': git {} (status={})",
                workspace_path,
                args.join(" "),
                output.status
            );
            None
        }
        Err(err) => {
            tracing::warn!(
                "Failed to execute git command in '{}': git {} ({})",
                workspace_path,
                args.join(" "),
                err
            );
            None
        }
    }
}
