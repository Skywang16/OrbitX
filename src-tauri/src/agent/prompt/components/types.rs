use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::config::PromptComponent;
use crate::agent::error::AgentResult;
use crate::agent::{Agent, Context, Task, ToolSchema};

/// Component assembly context shared across builders.
#[derive(Debug, Clone)]
pub struct ComponentContext {
    pub agent: Agent,
    pub task: Option<Task>,
    pub context: Option<Context>,
    pub tools: Vec<ToolSchema>,
    pub ext_sys_prompt: Option<String>,
    pub additional_context: HashMap<String, Value>,
}

impl ComponentContext {
    pub fn with_additional_context(mut self, data: HashMap<String, Value>) -> Self {
        self.additional_context.extend(data);
        self
    }
}

/// Trait implemented by every prompt component definition.
#[async_trait]
pub trait ComponentDefinition: Send + Sync {
    fn id(&self) -> PromptComponent;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn required(&self) -> bool;
    fn dependencies(&self) -> &[PromptComponent];
    fn default_template(&self) -> Option<&str>;

    /// Render the component into a string. Returning `Ok(None)` skips the section.
    async fn render(
        &self,
        context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>>;
}

/// Convenient alias to store component definitions in a registry.
pub type ComponentRegistry = HashMap<PromptComponent, Arc<dyn ComponentDefinition>>;
