use std::collections::HashMap;

use serde_json::Value;

use crate::agent::config::PromptComponent;
use crate::agent::error::{AgentError, AgentResult};
use crate::agent::prompt::components::{
    registry::PromptComponentRegistry, types::ComponentContext,
};

/// Builder options for prompt building.
#[derive(Debug, Clone, Default)]
pub struct PromptBuildOptions {
    pub components: Vec<PromptComponent>,
    pub template_overrides: HashMap<PromptComponent, String>,
    pub additional_context: HashMap<String, Value>,
}

/// Core prompt builder coordinating component assembly.
pub struct PromptBuilder {
    registry: PromptComponentRegistry,
}

impl PromptBuilder {
    pub fn ensure_components_loaded(&mut self) {
        self.registry.ensure_loaded();
    }

    pub fn new() -> Self {
        Self {
            registry: PromptComponentRegistry::new(),
        }
    }

    pub async fn build(
        &mut self,
        mut context: ComponentContext,
        options: PromptBuildOptions,
    ) -> AgentResult<String> {
        context
            .additional_context
            .extend(options.additional_context.clone());

        let components = options.components.clone();
        let errors = self.registry.validate_dependencies(&components);
        if !errors.is_empty() {
            return Err(AgentError::Internal(format!(
                "Prompt component dependency errors: {}",
                errors.join(", ")
            )));
        }

        let sorted = self
            .registry
            .sort_by_dependencies(&components)
            .map_err(|e| AgentError::Internal(format!("Prompt component dependency cycle: {e}")))?;

        let mut rendered = HashMap::new();
        for component_id in sorted.iter() {
            let Some(def) = self.registry.get(component_id.clone()) else {
                return Err(AgentError::Internal(format!(
                    "Component not found: {:?}",
                    component_id
                )));
            };

            let template_override = options
                .template_overrides
                .get(component_id)
                .map(|s| s.as_str());

            if let Some(content) = def.render(&context, template_override).await? {
                rendered.insert(component_id.clone(), content);
            }
        }

        self.assemble_prompt(&sorted, &rendered)
    }

    fn assemble_prompt(
        &self,
        ordered_components: &[PromptComponent],
        rendered: &HashMap<PromptComponent, String>,
    ) -> AgentResult<String> {
        let mut sections = Vec::new();
        for component_id in ordered_components {
            if let Some(content) = rendered.get(component_id) {
                if !content.trim().is_empty() {
                    sections.push(content.clone());
                }
            }
        }
        Ok(sections.join("\n\n").trim().to_string())
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}
