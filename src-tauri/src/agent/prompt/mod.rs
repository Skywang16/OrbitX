use crate::agent::error::AgentResult;
use crate::agent::{Agent, Context, Task, ToolSchema};

pub mod builders;
pub mod components;
pub mod orchestrator;
pub mod template_engine;

pub use builders::{
    PromptBuildOptions as BuildersPromptBuildOptions, PromptBuilder as CorePromptBuilder,
};
pub use components::types::{ComponentContext as ComponentsComponentContext, ComponentDefinition};
pub use template_engine::TemplateEngine;

/// Convenience API for building prompts.
pub async fn build_agent_system_prompt(
    agent: Agent,
    task: Option<Task>,
    context: Option<Context>,
    tools: Vec<ToolSchema>,
    ext_sys_prompt: Option<String>,
) -> AgentResult<String> {
    builders::build_agent_system_prompt(agent, task, context, tools, ext_sys_prompt).await
}

pub async fn build_agent_user_prompt(
    task: Option<Task>,
    context: Option<Context>,
    tools: Vec<ToolSchema>,
) -> AgentResult<String> {
    builders::build_agent_user_prompt(task, context, tools).await
}
