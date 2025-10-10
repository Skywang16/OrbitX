use std::collections::HashMap;

use crate::agent::config::{PromptComponent, PromptConfig, PromptType};
use crate::agent::error::AgentResult;
use crate::agent::prompt::builders::prompt_builder::{PromptBuildOptions, PromptBuilder};
use crate::agent::prompt::components::types::ComponentContext;
use crate::agent::{Agent, Context, Task, ToolSchema};

pub struct DialoguePromptBuilder {
    builder: PromptBuilder,
    config: PromptConfig,
}

impl DialoguePromptBuilder {
    pub fn new() -> Self {
        Self {
            builder: PromptBuilder::new(),
            config: PromptConfig::default(),
        }
    }

    pub async fn build_dialogue_system_prompt(
        &mut self,
        agent: Agent,
        task: Option<Task>,
        context: Option<Context>,
        tools: Vec<ToolSchema>,
    ) -> AgentResult<String> {
        let component_context = ComponentContext {
            agent,
            task,
            context,
            tools,
            ext_sys_prompt: None,
            additional_context: HashMap::new(),
        };

        let components = component_order(&self.config, PromptType::Dialogue);

        let options = PromptBuildOptions {
            components,
            skip_missing: true,
            ..Default::default()
        };

        self.builder
            .build(component_context, options)
            .await
            .map(|result| result.trim().to_string())
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

pub async fn build_dialogue_system_prompt(
    agent: Agent,
    task: Option<Task>,
    context: Option<Context>,
    tools: Vec<ToolSchema>,
) -> AgentResult<String> {
    let mut builder = DialoguePromptBuilder::new();
    builder
        .build_dialogue_system_prompt(agent, task, context, tools)
        .await
}
