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
        Arc::new(WorkspaceSnapshotComponent),
        Arc::new(AdditionalContextComponent),
    ]
}

struct WorkspaceSnapshotComponent;

#[async_trait]
impl ComponentDefinition for WorkspaceSnapshotComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::WorkspaceSnapshot
    }

    fn name(&self) -> &str {
        "Workspace Snapshot"
    }

    fn description(&self) -> &str {
        "Workspace state extracted from context"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some("## Workspace Snapshot\n{snapshot}")
    }

    async fn render(
        &self,
        context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let snapshot_value = context
            .additional_context
            .get("workspace_snapshot")
            .cloned()
            .or_else(|| {
                context
                    .context
                    .as_ref()
                    .and_then(|ctx| ctx.workspace_info.clone())
            });

        let snapshot_str = match snapshot_value {
            Some(serde_json::Value::String(s)) => Some(s),
            Some(value) => serde_json::to_string_pretty(&value).ok(),
            None => None,
        };

        let Some(snapshot) = snapshot_str.filter(|s| !s.trim().is_empty()) else {
            return Ok(None);
        };

        let template = template_override
            .or_else(|| self.default_template())
            .context("missing workspace snapshot template")?;

        let mut template_context = HashMap::new();
        template_context.insert("snapshot".to_string(), json!(snapshot));

        let result = TemplateEngine::new()
            .resolve(template, &template_context)
            .map_err(|e| anyhow!("failed to render workspace snapshot template: {}", e))?;

        Ok(Some(result))
    }
}

struct AdditionalContextComponent;

#[async_trait]
impl ComponentDefinition for AdditionalContextComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::AdditionalContext
    }

    fn name(&self) -> &str {
        "Additional Context"
    }

    fn description(&self) -> &str {
        "Additional structured context for the agent"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some("# Additional Context\n{context}")
    }

    async fn render(
        &self,
        context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        if context.additional_context.is_empty() {
            return Ok(None);
        }

        let template = template_override
            .or_else(|| self.default_template())
            .context("missing additional context template")?;

        let pretty = serde_json::to_string_pretty(&context.additional_context)
            .context("failed to format additional context as JSON")?;

        let mut template_context = HashMap::new();
        template_context.insert("context".to_string(), json!(pretty));

        let result = TemplateEngine::new()
            .resolve(template, &template_context)
            .map_err(|e| anyhow!("failed to render additional context template: {}", e))?;

        Ok(Some(result))
    }
}
