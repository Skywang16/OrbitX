/*!
 * Prompt Orchestrator - 从 executor.rs 提取的 prompt 构建逻辑
 */

use std::sync::Arc;

use chrono::Utc;

use crate::agent::context::ProjectContextLoader;
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::prompt::{build_agent_system_prompt, build_agent_user_prompt};
use crate::agent::tools::{ToolDescriptionContext, ToolRegistry};
use crate::agent::types::{Agent, Context as AgentContext, Task, TaskStatus};
use crate::storage::UnifiedCache;

pub struct PromptOrchestrator {
    cache: Arc<UnifiedCache>,
}

impl PromptOrchestrator {
    pub fn new(cache: Arc<UnifiedCache>) -> Self {
        Self { cache }
    }

    pub async fn build_task_prompts(
        &self,
        conversation_id: i64,
        task_id: String,
        user_prompt: &str,
        working_directory: Option<&str>,
        tool_registry: &ToolRegistry,
    ) -> TaskExecutorResult<(String, String)> {
        let cwd = working_directory.unwrap_or("/");
        let tool_schemas_full =
            tool_registry.get_tool_schemas_with_context(&ToolDescriptionContext {
                cwd: cwd.to_string(),
            });

        let tool_names: Vec<String> = tool_schemas_full.iter().map(|s| s.name.clone()).collect();

        let agent_info = Agent {
            name: "OrbitX Agent".to_string(),
            description: "An AI coding assistant for OrbitX".to_string(),
            capabilities: vec![],
            tools: tool_names,
        };

        let task_for_prompt = Task {
            id: task_id,
            conversation_id,
            user_prompt: user_prompt.to_owned(),
            xml: None,
            status: TaskStatus::Created,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut prompt_ctx = AgentContext::default();
        if let Some(dir) = working_directory {
            prompt_ctx.working_directory = Some(dir.to_owned());
        }
        prompt_ctx.additional_context.insert(
            "taskPrompt".to_string(),
            serde_json::Value::String(user_prompt.to_owned()),
        );

        // 获取用户规则
        let user_rules = self.cache.get_user_rules().await;

        // 获取项目规则
        let project_rules = self.cache.get_project_rules().await;

        // 合并项目上下文和用户规则
        let mut prompt_parts = Vec::new();

        let loader = ProjectContextLoader::new(cwd);
        if let Some(ctx) = loader.load_with_preference(project_rules.as_deref()).await {
            prompt_parts.push(ctx.format_for_prompt());
        }

        if let Some(rules) = user_rules {
            prompt_parts.push(rules);
        }

        let ext_sys_prompt = if prompt_parts.is_empty() {
            None
        } else {
            Some(prompt_parts.join("\n\n"))
        };

        let system_prompt = build_agent_system_prompt(
            agent_info.clone(),
            Some(task_for_prompt.clone()),
            Some(prompt_ctx.clone()),
            tool_schemas_full.clone(),
            ext_sys_prompt,
        )
        .await
        .map_err(|e| TaskExecutorError::InternalError(e.to_string()))?;

        let user_prompt_built = build_agent_user_prompt(
            agent_info,
            Some(task_for_prompt),
            Some(prompt_ctx),
            tool_schemas_full,
        )
        .await
        .map_err(|e| {
            TaskExecutorError::InternalError(format!("Failed to build user prompt: {}", e))
        })?;

        Ok((system_prompt, user_prompt_built))
    }
}
