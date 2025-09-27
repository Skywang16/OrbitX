use std::collections::HashMap;

use serde_json::{json, Value};

use crate::agent::config::{PromptComponent, PromptVariant};
use crate::agent::prompt::components::{
    registry::PromptComponentRegistry, types::ComponentContext,
};
use crate::agent::prompt::template_engine::TemplateEngine;
use crate::agent::{AgentError, AgentResult};

/// Builder options mirroring eko-core PromptBuildOptions.
#[derive(Debug, Clone)]
pub struct PromptBuildOptions {
    pub components: Vec<PromptComponent>,
    pub template_overrides: HashMap<PromptComponent, String>,
    pub additional_context: HashMap<String, Value>,
    pub skip_missing: bool,
}

impl Default for PromptBuildOptions {
    fn default() -> Self {
        Self {
            components: Vec::new(),
            template_overrides: HashMap::new(),
            additional_context: HashMap::new(),
            skip_missing: true,
        }
    }
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
        if !errors.is_empty() && !options.skip_missing {
            return Err(AgentError::PromptBuildingError(errors.join(", ")));
        }

        let sorted = self
            .registry
            .sort_by_dependencies(&components)
            .map_err(|e| AgentError::PromptBuildingError(e))?;

        let mut rendered = HashMap::new();
        for component_id in sorted.iter() {
            let Some(def) = self.registry.get(component_id.clone()) else {
                if options.skip_missing {
                    continue;
                }
                return Err(AgentError::PromptBuildingError(format!(
                    "组件不存在: {:?}",
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

    pub async fn build_from_variant(
        &mut self,
        context: ComponentContext,
        variant: &PromptVariant,
    ) -> AgentResult<String> {
        let options = PromptBuildOptions {
            components: variant.components.clone(),
            ..Default::default()
        };

        let rendered_map = self
            .build_components(context.clone(), options.clone())
            .await?;

        let mut template_context: HashMap<String, Value> = HashMap::new();
        for (component, value) in &rendered_map {
            template_context.insert(format!("{:?}", component).to_uppercase(), json!(value));
        }

        TemplateEngine::new()
            .resolve(&variant.template, &template_context)
            .map_err(AgentError::PromptBuildingError)
    }

    async fn build_components(
        &mut self,
        mut context: ComponentContext,
        options: PromptBuildOptions,
    ) -> AgentResult<HashMap<PromptComponent, String>> {
        context
            .additional_context
            .extend(options.additional_context.clone());

        let components = options.components.clone();
        let sorted = self
            .registry
            .sort_by_dependencies(&components)
            .map_err(|e| AgentError::PromptBuildingError(e))?;

        let mut rendered = HashMap::new();
        for component_id in sorted.iter() {
            let Some(def) = self.registry.get(component_id.clone()) else {
                if options.skip_missing {
                    continue;
                }
                return Err(AgentError::PromptBuildingError(format!(
                    "组件不存在: {:?}",
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

        Ok(rendered)
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
