//! Prompt Orchestrator - 构建任务提示词

use std::sync::Arc;

use crate::agent::agents::AgentConfigLoader;
use crate::agent::context::ProjectContextLoader;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::prompt::{BuiltinPrompts, PromptBuilder, SystemPromptParts};
use crate::agent::skill::{SkillManager, SkillMatchingMode};
use crate::agent::tools::{ToolDescriptionContext, ToolRegistry};
use crate::config::paths::ConfigPaths;
use crate::settings::SettingsManager;
use crate::storage::repositories::AppPreferences;
use crate::storage::{DatabaseManager, UnifiedCache};

pub struct PromptOrchestrator {
    cache: Arc<UnifiedCache>,
    database: Arc<DatabaseManager>,
    settings_manager: Arc<SettingsManager>,
    config_paths: Arc<ConfigPaths>,
}

impl PromptOrchestrator {
    pub fn new(
        cache: Arc<UnifiedCache>,
        database: Arc<DatabaseManager>,
        settings_manager: Arc<SettingsManager>,
        config_paths: Arc<ConfigPaths>,
    ) -> Self {
        Self {
            cache,
            database,
            settings_manager,
            config_paths,
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


    fn format_skills_for_prompt(
        &self,
        skills: &[crate::agent::skill::SkillContent],
    ) -> Option<String> {
        if skills.is_empty() {
            return None;
        }

        let mut blocks = Vec::new();
        for skill in skills {
            if skill.instructions.trim().is_empty() {
                continue;
            }
            blocks.push(format!(
                "<skill name=\"{}\">\n{}\n</skill>",
                skill.metadata.name,
                skill.instructions.trim()
            ));
        }

        if blocks.is_empty() {
            None
        } else {
            Some(blocks.join("\n\n"))
        }
    }

    pub async fn build_task_prompts(
        &self,
        session_id: i64,
        _task_id: String,
        user_prompt: &str,
        agent_type: &str,
        workspace_path: &str,
        tool_registry: &ToolRegistry,
    ) -> TaskExecutorResult<(String, String)> {
        let cwd = workspace_path;

        // 加载 agent 配置
        let agent_configs = AgentConfigLoader::load_for_workspace(&std::path::PathBuf::from(cwd))
            .await
            .unwrap_or_default();

        let agent_cfg = agent_configs
            .get(agent_type)
            .or_else(|| agent_configs.get("coder"));

        // 获取工具描述
        let tool_schemas = tool_registry.get_tool_schemas_with_context(&ToolDescriptionContext {
            cwd: cwd.to_string(),
        });
        let tools_description = tool_schemas
            .iter()
            .map(|s| format!("## {}\n{}", s.name, s.description))
            .collect::<Vec<_>>()
            .join("\n\n");

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

        // 加载 skills - 使用新的 SkillManager (全局 + 工作区)
        let skill_manager = SkillManager::new();
        let workspace_path_buf = std::path::PathBuf::from(cwd);

        // 获取全局 skills 目录
        let global_skills_dir = self.config_paths.skills_dir();

        // 发现技能 (全局 + 工作区)
        if let Ok(_) = skill_manager
            .discover_skills(Some(global_skills_dir), Some(&workspace_path_buf))
            .await
        {
            // 提取显式引用的技能列表
            let explicit_skills = agent_cfg
                .as_ref()
                .map(|cfg| cfg.skills.clone())
                .unwrap_or_default();

            // 激活技能 (使用 Hybrid 模式)
            if let Ok(activated_skills) = skill_manager
                .activate_skills(
                    user_prompt,
                    SkillMatchingMode::Hybrid,
                    Some(&explicit_skills),
                )
                .await
            {
                if let Some(skill_block) = self.format_skills_for_prompt(&activated_skills) {
                    custom_parts.push(skill_block);
                }
            }
        }

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
                tools_description: Some(tools_description),
            })
            .await;

        // user prompt 直接返回原始内容
        let user_prompt_built = user_prompt.to_string();

        Ok((system_prompt, user_prompt_built))
    }
}
