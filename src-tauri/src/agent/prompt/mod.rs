use std::collections::HashMap;

use crate::agent::config::{PromptComponent, PromptConfig, PromptType};
use crate::agent::prompt::builders::{
    build_agent_system_prompt as build_agent_system_prompt_impl,
    build_agent_user_prompt as build_agent_user_prompt_impl,
    build_dialogue_system_prompt as build_dialogue_system_prompt_impl,
    build_plan_system_prompt as build_plan_system_prompt_impl,
    build_plan_user_prompt as build_plan_user_prompt_impl,
    build_tree_plan_system_prompt as build_tree_plan_system_prompt_impl,
    build_tree_plan_user_prompt as build_tree_plan_user_prompt_impl, PromptBuildOptions,
    PromptBuilder,
};
use crate::agent::prompt::components::types::ComponentContext;
use crate::agent::{Agent, AgentResult, Context, Task, ToolSchema};

pub mod builders;
pub mod components;
pub mod template_engine;

pub use builders::{
    AgentPromptBuilder, DialoguePromptBuilder, PlanPromptBuilder,
    PromptBuildOptions as BuildersPromptBuildOptions, PromptBuilder as CorePromptBuilder,
};
pub use components::types::{ComponentContext as ComponentsComponentContext, ComponentDefinition};
pub use template_engine::TemplateEngine;

/// Backwards-compatible prompt manager facade.
pub struct PromptManager {
    builder: PromptBuilder,
    config: PromptConfig,
}

impl PromptManager {
    pub fn new() -> Self {
        Self {
            builder: PromptBuilder::new(),
            config: PromptConfig::default(),
        }
    }

    pub async fn load_components(&mut self) -> AgentResult<()> {
        self.builder.ensure_components_loaded();
        Ok(())
    }

    pub async fn build_prompt(
        &mut self,
        prompt_type: PromptType,
        context: ComponentContext,
        template_overrides: Option<HashMap<PromptComponent, String>>,
    ) -> AgentResult<String> {
        let components = component_order(&self.config, prompt_type);
        let overrides =
            template_overrides.unwrap_or_else(|| scenario_template_overrides(&self.config, None));

        let options = PromptBuildOptions {
            components,
            template_overrides: overrides,
            skip_missing: true,
            ..Default::default()
        };

        self.builder
            .build(context, options)
            .await
            .map(|result| result.trim().to_string())
    }
}

impl Default for PromptManager {
    fn default() -> Self {
        Self::new()
    }
}

fn component_order(config: &PromptConfig, prompt_type: PromptType) -> Vec<PromptComponent> {
    let mut order = config
        .default_component_order
        .get(&prompt_type)
        .cloned()
        .unwrap_or_default();

    order.retain(|component| {
        config
            .component_config
            .get(component)
            .map(|c| c.enabled)
            .unwrap_or(true)
    });

    order.sort_by_key(|component| {
        config
            .component_config
            .get(component)
            .map(|c| c.priority)
            .unwrap_or(0)
    });

    order
}

fn scenario_template_overrides(
    config: &PromptConfig,
    scenario: Option<&str>,
) -> HashMap<PromptComponent, String> {
    if let Some(name) = scenario {
        if let Some(overrides) = config.template_overrides.get(name) {
            return overrides.clone();
        }
    }

    config
        .template_overrides
        .get("default")
        .cloned()
        .unwrap_or_default()
}

/// Convenience API aligned with eko-core builders.
pub async fn build_agent_system_prompt(
    agent: Agent,
    task: Option<Task>,
    context: Option<Context>,
    tools: Vec<ToolSchema>,
    ext_sys_prompt: Option<String>,
) -> AgentResult<String> {
    build_agent_system_prompt_impl(agent, task, context, tools, ext_sys_prompt).await
}

pub async fn build_agent_user_prompt(
    agent: Agent,
    task: Option<Task>,
    context: Option<Context>,
    tools: Vec<ToolSchema>,
) -> AgentResult<String> {
    build_agent_user_prompt_impl(agent, task, context, tools).await
}

pub async fn build_dialogue_system_prompt(
    agent: Agent,
    task: Option<Task>,
    context: Option<Context>,
    tools: Vec<ToolSchema>,
) -> AgentResult<String> {
    build_dialogue_system_prompt_impl(agent, task, context, tools).await
}

pub async fn build_plan_system_prompt(
    agent: Agent,
    task: Option<Task>,
    context: Option<Context>,
    tools: Vec<ToolSchema>,
) -> AgentResult<String> {
    build_plan_system_prompt_impl(agent, task, context, tools).await
}

pub fn build_plan_user_prompt(task_prompt: &str) -> AgentResult<String> {
    Ok(build_plan_user_prompt_impl(task_prompt))
}

pub async fn build_tree_plan_system_prompt() -> String {
    build_tree_plan_system_prompt_impl().await
}

pub fn build_tree_plan_user_prompt(task_prompt: &str) -> String {
    build_tree_plan_user_prompt_impl(task_prompt)
}

pub async fn build_agent_system_prompt_with_context(
    context: ComponentContext,
    prompt_type: PromptType,
) -> AgentResult<String> {
    let mut manager = PromptManager::new();
    manager.build_prompt(prompt_type, context, None).await
}
