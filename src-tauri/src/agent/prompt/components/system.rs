use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::config::PromptComponent;
use crate::agent::error::{AgentError, AgentResult};
use crate::agent::prompt::components::types::{ComponentContext, ComponentDefinition};
use crate::agent::prompt::template_engine::TemplateEngine;
use crate::filesystem::commands::fs_list_directory;

const MAX_PREVIEW_FILES: usize = 50;

/// 获取目录文件列表预览（最多50个文件）
async fn get_directory_preview(working_directory: &str) -> String {
    if working_directory == "Not specified" || working_directory.trim().is_empty() {
        return String::new();
    }

    match fs_list_directory(working_directory.to_string(), false).await {
        Ok(response) if response.code == 200 => {
            if let Some(mut entries) = response.data {
                let total = entries.len();
                let truncated = total > MAX_PREVIEW_FILES;

                entries.truncate(MAX_PREVIEW_FILES);

                let mut preview = String::from("Files in Current Directory:\n");
                for entry in entries {
                    preview.push_str(&format!("  {}", entry));
                    preview.push('\n');
                }

                if truncated {
                    preview.push_str(&format!(
                        "  ... and {} more files (use list_files tool to see all)\n",
                        total - MAX_PREVIEW_FILES
                    ));
                }

                preview
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

pub fn definitions() -> Vec<Arc<dyn ComponentDefinition>> {
    vec![Arc::new(SystemInfoComponent), Arc::new(DateTimeComponent)]
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
        Some("Here is useful information about the environment you are running in:\n<env>\nWorking directory: {working_directory}\nPlatform: {platform}\nToday's date: {current_date}\n</env>\n\n{file_list_preview}")
    }

    async fn render(
        &self,
        context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::Internal("missing system info template".to_string()))?;

        let working_directory = context
            .context
            .as_ref()
            .and_then(|c| c.working_directory.as_deref())
            .unwrap_or("Not specified");

        // 获取当前目录的文件列表（最多50个）
        let file_list_preview = get_directory_preview(working_directory).await;

        let mut template_context = HashMap::new();
        template_context.insert("platform".to_string(), json!("macOS"));
        template_context.insert("working_directory".to_string(), json!(working_directory));
        template_context.insert(
            "current_date".to_string(),
            json!(chrono::Utc::now().format("%Y-%m-%d").to_string()),
        );
        template_context.insert("file_list_preview".to_string(), json!(file_list_preview));

        let result = TemplateEngine::new()
            .resolve(template, &template_context)
            .map_err(|e| {
                AgentError::TemplateRender(format!("failed to render system info template: {}", e))
            })?;

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
        Some("You are OrbitX Agent, a terminal-focused AI assistant.")
    }

    async fn render(
        &self,
        _context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::Internal("missing datetime template".to_string()))?;

        Ok(Some(template.to_string()))
    }
}
