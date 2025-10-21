use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::agent::context::FileOperationRecord;
use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::persistence::FileRecordSource;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPermission, ToolPriority, ToolResult,
    ToolResultContent,
};

use super::file_utils::{ensure_absolute, is_probably_binary};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteFileArgs {
    path: String,
    content: String,
}

pub struct WriteFileTool;

impl WriteFileTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Writes a file to the local filesystem.

Usage:
- The path parameter must be an absolute path (e.g., '/Users/user/project/src/main.ts')
- This tool will overwrite the existing file if there is one at the provided path.
- If this is an existing file, you MUST use the read_file tool first to read the file's contents. This tool will fail if you did not read the file first.
- ALWAYS prefer editing existing files in the codebase. NEVER write new files unless explicitly required.
- NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested by the User.
- Only use emojis if the user explicitly requests it. Avoid writing emojis to files unless asked."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute path to the file to write. For example: \"/Users/user/project/src/main.ts\". Parent directory must already exist."
                },
                "content": {
                    "type": "string",
                    "description": "The complete content to write to the file. This will overwrite any existing content. For existing files, prefer using edit_file tool instead."
                }
            },
            "required": ["path", "content"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileWrite, ToolPriority::Standard)
            .with_tags(vec!["filesystem".into(), "write".into()])
            .with_summary_key_arg("path")
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: WriteFileArgs = serde_json::from_value(args)?;
        let path = match ensure_absolute(&args.path, &context.cwd) {
            Ok(resolved) => resolved,
            Err(err) => return Ok(error_result(err.to_string())),
        };

        if is_probably_binary(&path) {
            return Ok(error_result(format!(
                "File {} appears to be binary, cannot write as text",
                path.display()
            )));
        }

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Ok(error_result(format!(
                    "Parent directory does not exist: {}. Please verify the path or create the directory first.",
                    parent.display()
                )));
            }
        }

        if let Ok(meta) = fs::metadata(&path).await {
            if meta.is_dir() {
                return Ok(error_result(format!(
                    "Path {} is a directory, not a file",
                    path.display()
                )));
            }
        }

        if let Err(err) = fs::write(&path, args.content).await {
            return Ok(error_result(format!(
                "Failed to write file {}: {}",
                path.display(),
                err
            )));
        }

        context
            .file_tracker()
            .track_file_operation(FileOperationRecord::new(
                path.as_path(),
                FileRecordSource::AgentEdited,
            ))
            .await?;

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(format!(
                "write_file applied\nfile={}",
                path.display()
            ))],
            is_error: false,
            execution_time_ms: None,
            ext_info: Some(json!({
                "path": path.display().to_string()
            })),
        })
    }
}

fn error_result(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        is_error: true,
        execution_time_ms: None,
        ext_info: None,
    }
}
