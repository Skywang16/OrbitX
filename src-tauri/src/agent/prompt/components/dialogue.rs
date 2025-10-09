use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::config::PromptComponent;
use crate::agent::error::{AgentError, AgentResult};
use crate::agent::prompt::components::types::{ComponentContext, ComponentDefinition};
use crate::agent::prompt::template_engine::TemplateEngine;

pub fn definitions() -> Vec<Arc<dyn ComponentDefinition>> {
    vec![
        Arc::new(DialogueCapabilitiesComponent),
        Arc::new(DialogueGuidelinesComponent),
    ]
}

struct DialogueCapabilitiesComponent;

#[async_trait]
impl ComponentDefinition for DialogueCapabilitiesComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::DialogueCapabilities
    }

    fn name(&self) -> &str {
        "Dialogue Capabilities"
    }

    fn description(&self) -> &str {
        "Dialogue capabilities description"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some(
            r#"# Terminal Environment Capabilities
You excel at helping users with:
- File operations and text editing
- Shell command execution and scripting
- Code development and project management
- System administration and automation
- Terminal-based workflows and productivity"#,
        )
    }

    async fn render(
        &self,
        _context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::Internal("missing dialogue capabilities template".to_string()))?;

        let result = TemplateEngine::new()
            .resolve(template, &HashMap::new())
            .map_err(|e| AgentError::TemplateRender(format!("failed to render dialogue capabilities template: {}", e)))?;
        Ok(Some(result))
    }
}

struct DialogueGuidelinesComponent;

#[async_trait]
impl ComponentDefinition for DialogueGuidelinesComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::DialogueGuidelines
    }

    fn name(&self) -> &str {
        "Dialogue Guidelines"
    }

    fn description(&self) -> &str {
        "Dialogue guidance principles"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some(
            r#"# Dialogue Guidelines
- Provide clear and helpful responses
- Ask clarifying questions when needed
- Offer practical solutions and examples
- Maintain context throughout the conversation

## Safety & Scope Confirmation
- If the user's requested scope appears overly broad or system-managed (e.g., '/', '/Users', '/home', '/var'), ask for confirmation or a narrower subpath before describing large operations."#,
        )
    }

    async fn render(
        &self,
        _context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::Internal("missing dialogue guidelines template".to_string()))?;

        let result = TemplateEngine::new()
            .resolve(template, &HashMap::new())
            .map_err(|e| AgentError::TemplateRender(format!("failed to render dialogue guidelines template: {}", e)))?;
        Ok(Some(result))
    }
}
