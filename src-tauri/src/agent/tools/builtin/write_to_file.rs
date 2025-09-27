use std::path::PathBuf;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::agent::state::context::TaskContext;
use crate::agent::tools::{
    RunnableTool, ToolExecutorResult, ToolPermission, ToolResult, ToolResultContent,
};

use super::file_utils::{ensure_absolute, is_probably_binary};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteToFileArgs {
    path: String,
    content: String,
}

pub struct WriteToFileTool;

impl WriteToFileTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for WriteToFileTool {
    fn name(&self) -> &str {
        "write_to_file"
    }

    fn description(&self) -> &str {
        "Create or overwrite a file with provided text content."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute file path"
                },
                "content": {
                    "type": "string",
                    "description": "New file content"
                }
            },
            "required": ["path", "content"]
        })
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }

    fn tags(&self) -> Vec<String> {
        vec!["file".into(), "write".into(), "overwrite".into()]
    }

    async fn run(
        &self,
        _context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: WriteToFileArgs = serde_json::from_value(args)?;
        let path = PathBuf::from(&args.path);

        if let Err(msg) = ensure_absolute(&path) {
            return Ok(validation_error(msg));
        }

        if is_probably_binary(&path) {
            return Ok(validation_error(format!(
                "File {} appears to be binary, text writing not supported",
                path.display()
            )));
        }

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Ok(tool_error(format!(
                    "Parent directory does not exist: {}",
                    parent.display()
                )));
            }
        }

        let (created, is_directory) = match fs::metadata(&path).await {
            Ok(meta) => (false, meta.is_dir()),
            Err(_) => (true, false),
        };

        if is_directory {
            return Ok(validation_error(format!(
                "Path {} is a directory, cannot write file",
                path.display()
            )));
        }

        if let Err(err) = fs::write(&path, args.content).await {
            return Ok(tool_error(format!(
                "Failed to write file {}: {}",
                path.display(),
                err
            )));
        }

        let message = format!(
            "write_to_file applied\nFile: {}\nCreated: {}",
            path.display(),
            created
        );

        Ok(ToolResult {
            content: vec![ToolResultContent::Text { text: message }],
            is_error: false,
            execution_time_ms: None,
            metadata: Some(json!({
                "file": path.display().to_string(),
                "created": created,
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
