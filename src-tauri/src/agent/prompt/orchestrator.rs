//! Prompt Orchestrator - 构建任务提示词

use std::sync::Arc;

use crate::agent::agents::AgentConfigLoader;
use crate::agent::context::ProjectContextLoader;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::prompt::{BuiltinPrompts, PromptBuilder, SystemPromptParts};
use crate::agent::tools::ToolRegistry;
use crate::settings::SettingsManager;
use crate::storage::repositories::AppPreferences;
use crate::storage::{DatabaseManager, UnifiedCache};

pub struct PromptOrchestrator {
    cache: Arc<UnifiedCache>,
    database: Arc<DatabaseManager>,
    settings_manager: Arc<SettingsManager>,
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
    ) -> TaskExecutorResult<(Option<String>, Option<String>)> {
        let effective = self
            .settings_manager
            .get_effective_settings(Some(std::path::PathBuf::from(workspace_path)))
            .await
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

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
            .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        let _ = self.cache.set_global_rules(global_rules.clone()).await;
        let _ = self.cache.set_project_rules(project_rules.clone()).await;

        Ok((global_rules, project_rules))
    }

    async fn has_agent_messages(
        &self,
        session_id: i64,
        agent_type: &str,
    ) -> TaskExecutorResult<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(1) FROM messages WHERE session_id = ? AND agent_type = ? LIMIT 1",
        )
        .bind(session_id)
        .bind(agent_type)
        .fetch_one(self.database.pool())
        .await
        .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

        Ok(count > 0)
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


    // format_skills_for_prompt 已移除：
    // Skill 系统改为 Tool 机制，LLM 通过 tool calling 自动激活 skill
    // skill 内容不再预先注入到 system prompt 中

    pub async fn build_task_prompts(
        &self,
        session_id: i64,
        _task_id: String,
        user_prompt: &str,
        agent_type: &str,
        workspace_path: &str,
        _tool_registry: &ToolRegistry,
    ) -> TaskExecutorResult<(String, String)> {
        let cwd = workspace_path;

        // 加载 agent 配置
        let agent_configs = AgentConfigLoader::load_for_workspace(&std::path::PathBuf::from(cwd))
            .await
            .unwrap_or_default();

        let agent_cfg = agent_configs
            .get(agent_type)
            .or_else(|| agent_configs.get("coder"));

        // 加载规则
        let (global_rules, project_rules) = self.load_rules(workspace_path).await?;

        // 加载项目上下文
        let loader = ProjectContextLoader::new(cwd);
        let project_context = loader.load_with_preference(project_rules.as_deref()).await;

        // 构建自定义指令
        let mut custom_parts = Vec::new();
        if let Some(ctx) = project_context {
            custom_parts.push(ctx.format_for_prompt());
        }
        if let Some(rules) = global_rules {
            custom_parts.push(rules);
        }

        // Skill 系统已改为 Tool 机制，不再需要在此处预先注入
        // LLM 会通过 tool calling 自动激活所需的 skill

        let custom_instructions = if custom_parts.is_empty() {
            None
        } else {
            Some(custom_parts.join("\n\n"))
        };

        // 获取 reminder
        let has_plan_history = self
            .has_agent_messages(session_id, "plan")
            .await
            .unwrap_or(false);
        let reminder = self.get_reminder(agent_type, has_plan_history);

        // 构建环境信息
        let builder = PromptBuilder::new(Some(workspace_path.to_string()));
        let env_info = builder.build_env_info(Some(cwd), None);

        // 获取 agent 提示词
        let agent_prompt = agent_cfg.map(|cfg| cfg.system_prompt.clone());

        // 组装 system prompt
        let mut prompt_builder = PromptBuilder::new(Some(workspace_path.to_string()));
        let system_prompt = prompt_builder
            .build_system_prompt(SystemPromptParts {
                agent_prompt,
                rules: Some(BuiltinPrompts::rules().to_string()),
                methodology: Some(BuiltinPrompts::methodology().to_string()),
                env_info: Some(env_info),
                reminder,
                custom_instructions,
            })
            .await;

        // user prompt 直接返回原始内容
        let user_prompt_built = user_prompt.to_string();

        Ok((system_prompt, user_prompt_built))
    }
}
