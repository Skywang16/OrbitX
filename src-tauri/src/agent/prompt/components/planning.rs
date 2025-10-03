use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Context};

use crate::agent::config::PromptComponent;
use crate::agent::error::AgentResult;
use crate::agent::prompt::components::types::{ComponentContext, ComponentDefinition};
use crate::agent::prompt::template_engine::TemplateEngine;

pub fn definitions() -> Vec<Arc<dyn ComponentDefinition>> {
    vec![
        Arc::new(PlanningGuidelinesComponent),
        Arc::new(PlanningExamplesComponent),
        Arc::new(OutputFormatComponent),
    ]
}

struct PlanningGuidelinesComponent;

#[async_trait]
impl ComponentDefinition for PlanningGuidelinesComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::PlanningGuidelines
    }

    fn name(&self) -> &str {
        "Planning Guidelines"
    }

    fn description(&self) -> &str {
        "Planning guidance principles"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some(
            r#"# Planning Guidelines
- Adaptive planning: create the minimal number of nodes required to finish the task
- Sequential execution: break down tasks into logical steps and order them clearly
- Tool utilization: reference available tools when describing steps that require them
- Efficient planning: focus on the most direct path to complete the user's request"#,
        )
    }

    async fn render(
        &self,
        _context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let template = template_override
            .or_else(|| self.default_template())
            .context("missing planning guidelines template")?;

        let result = TemplateEngine::new()
            .resolve(template, &HashMap::new())
            .map_err(|e| anyhow!("failed to render planning guidelines template: {}", e))?;
        Ok(Some(result))
    }
}

struct PlanningExamplesComponent;

#[async_trait]
impl ComponentDefinition for PlanningExamplesComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::PlanningExamples
    }

    fn name(&self) -> &str {
        "Planning Examples"
    }

    fn description(&self) -> &str {
        "Planning examples"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        None
    }

    async fn render(
        &self,
        _context: &ComponentContext,
        _template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        Ok(None)
    }
}

struct OutputFormatComponent;

#[async_trait]
impl ComponentDefinition for OutputFormatComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::OutputFormat
    }

    fn name(&self) -> &str {
        "Output Format"
    }

    fn description(&self) -> &str {
        "Output format description"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        None
    }

    async fn render(
        &self,
        _context: &ComponentContext,
        _template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        Ok(None)
    }
}
