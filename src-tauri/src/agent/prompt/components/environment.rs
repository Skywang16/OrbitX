use async_trait::async_trait;
use chrono::Utc;
use crate::agent::config::PromptComponent;
use crate::agent::error::AgentResult;
use crate::agent::prompt::components::types::{ComponentContext, ComponentDefinition};

pub struct EnvironmentComponent;

impl EnvironmentComponent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ComponentDefinition for EnvironmentComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::Environment
    }

    fn name(&self) -> &str {
        "Environment"
    }

    fn description(&self) -> &str {
        "Dynamic environment information (time, directory, platform)"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some(
            r#"# Environment Information
<env>
Current time: {{current_time}}
Working directory: {{working_directory}}
Platform: {{platform}}
{{#if is_git_repo}}
Git repository: Yes
{{else}}
Git repository: No
{{/if}}
</env>
"#,
        )
    }

    async fn render(
        &self,
        context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let current_time = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        
        let working_directory = context
            .context
            .as_ref()
            .and_then(|ctx| ctx.working_directory.clone())
            .unwrap_or_else(|| "/".to_string());

        let platform = std::env::consts::OS;

        // Check if it's a git repo
        let is_git_repo = tokio::fs::metadata(format!("{}/.git", working_directory))
            .await
            .is_ok();

        let template = template_override.unwrap_or_else(|| {
            self.default_template().unwrap()
        });

        let mut handlebars = handlebars::Handlebars::new();
        handlebars.register_escape_fn(handlebars::no_escape);
        
        let mut data = serde_json::json!({
            "current_time": current_time,
            "working_directory": working_directory,
            "platform": platform,
            "is_git_repo": is_git_repo,
        });

        let rendered = handlebars
            .render_template(template, &data)
            .map_err(|e| crate::agent::error::AgentError::PromptError(e.to_string()))?;

        Ok(Some(rendered.trim().to_string()))
    }
}
