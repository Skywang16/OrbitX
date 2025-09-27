use std::path::PathBuf;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::agent::state::context::TaskContext;
use crate::agent::tools::{
    RunnableTool, ToolExecutorResult, ToolPermission, ToolResult, ToolResultContent,
};
use crate::filesystem::commands::fs_list_directory;

use super::file_utils::ensure_absolute;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListFilesArgs {
    path: String,
    recursive: Option<bool>,
}

pub struct ListFilesTool;

impl ListFilesTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for ListFilesTool {
    fn name(&self) -> &str {
        "list_files"
    }

    fn description(&self) -> &str {
        "List files and directories within the specified directory."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory path (relative or absolute)" },
                "recursive": { "type": "boolean", "description": "List recursively if true" }
            },
            "required": ["path"]
        })
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }

    fn tags(&self) -> Vec<String> {
        vec!["file".into(), "list".into(), "directory".into()]
    }

    async fn run(
        &self,
        _context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: ListFilesArgs = serde_json::from_value(args)?;
        let raw_path = args.path.trim();
        if raw_path.is_empty() {
            return Ok(validation_error("Directory path cannot be empty"));
        }

        let mut path = PathBuf::from(raw_path);
        if !path.is_absolute() {
            match std::env::current_dir() {
                Ok(cwd) => {
                    path = cwd.join(&path);
                }
                Err(err) => {
                    return Ok(tool_error(format!(
                        "Failed to resolve relative path '{}': {}",
                        raw_path, err
                    )));
                }
            }
        }

        if let Err(msg) = ensure_absolute(&path) {
            return Ok(validation_error(msg));
        }

        let recursive = args.recursive.unwrap_or(false);
        let request_path = path.to_string_lossy().to_string();

        let response = fs_list_directory(request_path.clone(), recursive).await;
        let api_response = match response {
            Ok(resp) => resp,
            Err(err) => {
                return Ok(tool_error(format!("Directory listing failed: {}", err)));
            }
        };

        if api_response.code != 200 {
            let message = api_response
                .message
                .unwrap_or_else(|| "Failed to list directory".to_string());
            return Ok(tool_error(message));
        }

        let entries = api_response.data.unwrap_or_default();
        let header = format!(
            "Directory listing for {} ({}, {} entries):",
            path.display(),
            if recursive {
                "recursive"
            } else {
                "non-recursive"
            },
            entries.len()
        );
        let mut text = header.clone();
        if !entries.is_empty() {
            text.push('\n');
            text.push_str(&entries.join("\n"));
        }

        Ok(ToolResult {
            content: vec![ToolResultContent::Text { text }],
            is_error: false,
            execution_time_ms: None,
            metadata: Some(json!({
                "path": path.display().to_string(),
                "count": entries.len(),
                "recursive": recursive,
                "entries": entries,
            })),
        })
    }
}

fn validation_error(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error {
            message: message.into(),
            details: None,
        }],
        is_error: true,
        execution_time_ms: None,
        metadata: None,
    }
}

fn tool_error(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error {
            message: message.into(),
            details: None,
        }],
        is_error: true,
        execution_time_ms: None,
        metadata: None,
    }
}
