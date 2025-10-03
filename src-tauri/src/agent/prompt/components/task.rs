use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Context};

use crate::agent::config::PromptComponent;
use crate::agent::error::AgentResult;
use crate::agent::prompt::components::types::{ComponentContext, ComponentDefinition};
use crate::agent::prompt::template_engine::TemplateEngine;

pub fn definitions() -> Vec<Arc<dyn ComponentDefinition>> {
    vec![
        Arc::new(TaskContextComponent),
        Arc::new(TaskNodesComponent),
        Arc::new(TaskExamplesComponent),
    ]
}

struct TaskContextComponent;

#[async_trait]
impl ComponentDefinition for TaskContextComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::TaskContext
    }

    fn name(&self) -> &str {
        "Task Context"
    }

    fn description(&self) -> &str {
        "Current task context and information"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some("TASK CONTEXT\n\nUser Request: {user_prompt}\nTask Status: {task_status}\n{additional_context}")
    }

    async fn render(
        &self,
        context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let task = match &context.task {
            Some(task) => task,
            None => return Ok(None),
        };

        let template = template_override
            .or_else(|| self.default_template())
            .context("missing task context template")?;

        let additional_context = if context.additional_context.is_empty() {
            "".to_string()
        } else {
            format!(
                "\nAdditional Context:\n{}",
                serde_json::to_string_pretty(&context.additional_context)
                    .unwrap_or_else(|_| "{}".to_string())
            )
        };

        let mut template_context = HashMap::new();
        template_context.insert("user_prompt".to_string(), json!(task.user_prompt.clone()));
        template_context.insert("task_status".to_string(), json!(task.status.to_string()));
        template_context.insert("additional_context".to_string(), json!(additional_context));

        let result = TemplateEngine::new()
            .resolve(template, &template_context)
            .map_err(|e| anyhow!("failed to render task context template: {}", e))?;

        Ok(Some(result))
    }
}

struct TaskNodesComponent;

#[async_trait]
impl ComponentDefinition for TaskNodesComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::TaskNodes
    }

    fn name(&self) -> &str {
        "Task Nodes"
    }

    fn description(&self) -> &str {
        "Task node processing description"
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

struct TaskExamplesComponent;

#[async_trait]
impl ComponentDefinition for TaskExamplesComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::TaskExamples
    }

    fn name(&self) -> &str {
        "Task Examples"
    }

    fn description(&self) -> &str {
        "Task processing examples"
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
