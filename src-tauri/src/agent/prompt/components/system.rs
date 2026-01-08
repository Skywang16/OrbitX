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
    vec![Arc::new(SystemInfoComponent)]
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
        Some("Here is useful information about the environment you are running in:\n<env>\nWorking directory: {working_directory}\nPlatform: {platform}\nToday's date: {current_date}\n</env>\n{workspace_status}\n{file_list_preview}")
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
            .filter(|w| !w.trim().is_empty())
            .unwrap_or("Not specified");

        let has_workspace = working_directory != "Not specified";

        let file_list_preview = if has_workspace {
            get_directory_preview(working_directory).await
        } else {
            String::new()
        };

        let workspace_status = if has_workspace {
            String::new()
        } else {
            "\n<important>\nNOTE: The user is NOT currently in any workspace directory. File system tools (read_file, write_file, list_files, shell, etc.) are NOT available. You can only have a general conversation. If the user asks about files or wants to run commands, inform them that they need to open a terminal tab first to establish a workspace.\n</important>\n".to_string()
        };

        let mut template_context = HashMap::new();
        template_context.insert("platform".to_string(), json!("macOS"));
        template_context.insert("working_directory".to_string(), json!(working_directory));
        template_context.insert(
            "current_date".to_string(),
            json!(chrono::Utc::now().format("%Y-%m-%d").to_string()),
        );
        template_context.insert("file_list_preview".to_string(), json!(file_list_preview));
        template_context.insert("workspace_status".to_string(), json!(workspace_status));

        let result = TemplateEngine::new().resolve(template, &template_context);

        Ok(Some(result))
    }
}
