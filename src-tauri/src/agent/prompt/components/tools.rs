use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::config::PromptComponent;
use crate::agent::prompt::components::types::{ComponentContext, ComponentDefinition};
use crate::agent::prompt::template_engine::TemplateEngine;
use crate::agent::{AgentError, AgentResult};

pub fn definitions() -> Vec<Arc<dyn ComponentDefinition>> {
    vec![Arc::new(ToolsDescriptionComponent)]
}

struct ToolsDescriptionComponent;

#[async_trait]
impl ComponentDefinition for ToolsDescriptionComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::ToolsDescription
    }

    fn name(&self) -> &str {
        "Tools Description"
    }

    fn description(&self) -> &str {
        "Description of available tools"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some("# Available Tools\n{tools_list}\n\nEach tool has specific parameters and usage patterns. Always provide the required parameters in the correct JSON format.")
    }

    async fn render(
        &self,
        context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        if context.tools.is_empty() {
            return Ok(None);
        }

        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::PromptBuildingError("missing template".into()))?;

        let tools_list = context
            .tools
            .iter()
            .map(|tool| format!("- {}: {}", tool.name, tool.description.clone()))
            .collect::<Vec<_>>()
            .join("\n");

        let mut template_context = HashMap::new();
        template_context.insert("tools_list".to_string(), json!(tools_list));

        let result = TemplateEngine::new()
            .resolve(template, &template_context)
            .map_err(AgentError::PromptBuildingError)?;
        Ok(Some(result))
    }
}
