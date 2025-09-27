use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::config::PromptComponent;
use crate::agent::prompt::components::types::{ComponentContext, ComponentDefinition};
use crate::agent::prompt::template_engine::TemplateEngine;
use crate::agent::{AgentError, AgentResult};

pub fn definitions() -> Vec<Arc<dyn ComponentDefinition>> {
    vec![
        Arc::new(SystemInfoComponent),
        Arc::new(DateTimeComponent),
        Arc::new(PlatformComponent),
    ]
}

struct SystemInfoComponent;

#[async_trait]
impl ComponentDefinition for SystemInfoComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::SystemInfo
    }

    fn name(&self) -> &str {
        "System Info"
    }

    fn description(&self) -> &str {
        "System information and environment context"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some("SYSTEM INFORMATION\n\nOperating System: {platform}\nWorking Directory: {working_directory}\nEnvironment Context: {environment_context}")
    }

    async fn render(
        &self,
        context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::PromptBuildingError("missing template".into()))?;

        let working_directory = context
            .context
            .as_ref()
            .and_then(|c| c.working_directory.as_deref())
            .unwrap_or("Not specified");

        let environment_context = if let Some(ctx) = &context.context {
            if ctx.environment_vars.is_empty() {
                "No environment variables specified".to_string()
            } else {
                format!(
                    "{} environment variables available",
                    ctx.environment_vars.len()
                )
            }
        } else {
            "No environment context available".to_string()
        };

        let mut template_context = HashMap::new();
        template_context.insert("platform".to_string(), json!("macOS"));
        template_context.insert("working_directory".to_string(), json!(working_directory));
        template_context.insert(
            "environment_context".to_string(),
            json!(environment_context),
        );

        let result = TemplateEngine::new()
            .resolve(template, &template_context)
            .map_err(AgentError::PromptBuildingError)?;

        Ok(Some(result))
    }
}

struct DateTimeComponent;

#[async_trait]
impl ComponentDefinition for DateTimeComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::DateTime
    }

    fn name(&self) -> &str {
        "DateTime"
    }

    fn description(&self) -> &str {
        "Current date and time information"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some("Current time: {current_time}")
    }

    async fn render(
        &self,
        _context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::PromptBuildingError("missing template".into()))?;

        let mut template_context = HashMap::new();
        template_context.insert(
            "current_time".to_string(),
            json!(chrono::Utc::now()
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string()),
        );

        let result = TemplateEngine::new()
            .resolve(template, &template_context)
            .map_err(AgentError::PromptBuildingError)?;

        Ok(Some(result))
    }
}

struct PlatformComponent;

#[async_trait]
impl ComponentDefinition for PlatformComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::Platform
    }

    fn name(&self) -> &str {
        "Platform"
    }

    fn description(&self) -> &str {
        "Platform information"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some("Platform: {platform}")
    }

    async fn render(
        &self,
        context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::PromptBuildingError("missing template".into()))?;

        let platform = context
            .context
            .as_ref()
            .and_then(|ctx| ctx.environment_vars.get("OS"))
            .cloned()
            .unwrap_or_else(|| "macOS".to_string());

        let mut template_context = HashMap::new();
        template_context.insert("platform".to_string(), json!(platform));

        let result = TemplateEngine::new()
            .resolve(template, &template_context)
            .map_err(AgentError::PromptBuildingError)?;
        Ok(Some(result))
    }
}
