use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPermission, ToolPriority, ToolResult,
    ToolResultContent,
};
use crate::filesystem::commands::{code_list_definition_names, CodeDefItem};

use super::file_utils::ensure_absolute;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListCodeDefinitionsArgs {
    path: String,
}

pub struct ListCodeDefinitionNamesTool;

impl ListCodeDefinitionNamesTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for ListCodeDefinitionNamesTool {
    fn name(&self) -> &str {
        "list_code_definition_names"
    }

    fn description(&self) -> &str {
        "List definition names from source code files within a path."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File or directory path (relative or absolute)" }
            },
            "required": ["path"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::CodeAnalysis, ToolPriority::Standard)
            .with_tags(vec!["code".into(), "list".into()])
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: ListCodeDefinitionsArgs = serde_json::from_value(args)?;
        let trimmed = args.path.trim();
        if trimmed.is_empty() {
            return Ok(validation_error("Path cannot be empty"));
        }

        let path = match ensure_absolute(trimmed, &context.cwd) {
            Ok(resolved) => resolved,
            Err(err) => return Ok(validation_error(err.to_string())),
        };

        let request_path = path.to_string_lossy().to_string();
        let response = code_list_definition_names(request_path.clone()).await;
        let api_response = match response {
            Ok(resp) => resp,
            Err(err) => {
                return Ok(tool_error(format!(
                    "Failed to list code definitions: {}",
                    err
                )));
            }
        };

        if api_response.code != 200 {
            let message = api_response
                .message
                .unwrap_or_else(|| "Failed to list code definitions".to_string());
            return Ok(tool_error(message));
        }

        let definitions: Vec<CodeDefItem> = api_response.data.unwrap_or_default();
        let count = definitions.len();
        let header = format!("Found {} definition(s)", count);
        let mut body_lines = Vec::new();
        for def in &definitions {
            let mut label = def.kind.clone();
            if def.exported.unwrap_or(false) {
                label.push_str(" export");
            }
            if def.is_default.unwrap_or(false) {
                label.push_str(" default");
            }
            body_lines.push(format!(
                "{} {} @ {}:L{}",
                label, def.name, def.file, def.line
            ));
        }

        let mut text = header.clone();
        if !body_lines.is_empty() {
            text.push('\n');
            text.push_str(&body_lines.join("\n"));
        }

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(text)],
            is_error: false,
            execution_time_ms: None,
            ext_info: Some(json!({
                "count": count,
                "definitions": definitions,
                "path": path.display().to_string(),
            })),
        })
    }
}

fn validation_error(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        is_error: true,
        execution_time_ms: None,
        ext_info: None,
    }
}

fn tool_error(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        is_error: true,
        execution_time_ms: None,
        ext_info: None,
    }
}
