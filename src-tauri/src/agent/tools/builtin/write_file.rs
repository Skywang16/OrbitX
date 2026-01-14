use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::agent::context::FileOperationRecord;
use crate::agent::context::FileRecordSource;
use crate::agent::core::context::TaskContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};

use super::file_utils::{ensure_absolute, is_probably_binary};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteFileArgs {
    path: String,
    content: String,
}

pub struct WriteFileTool;

impl Default for WriteFileTool {
    fn default() -> Self {
        Self::new()
    }
}

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
        r#"Writes a file to the local filesystem. Use this tool to create new files or completely overwrite existing ones.

Usage:
- The path parameter must be an absolute path (e.g., '/Users/user/project/src/main.ts')
- This tool will overwrite the existing file if there is one at the provided path
- If this is an existing file, you MUST use the read_file tool first to read the file's contents. This tool will fail if you did not read the file first
- ALWAYS prefer editing existing files in the codebase using edit_file tool. NEVER write new files unless explicitly required
- NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested by the User
- Only use emojis if the user explicitly requests it. Avoid writing emojis to files unless asked
- Parent directory must already exist - this tool will not create directories

File Creation Guidelines:
- Use this tool only when creating genuinely new files
- For existing files, prefer edit_file tool for better change tracking
- Ensure the file extension matches the content type
- Consider the project's existing file structure and naming conventions

Examples:
- Create new file: {"path": "/path/to/new-file.js", "content": "console.log('Hello World');"}
- Overwrite existing: {"path": "/path/to/existing.js", "content": "// Completely new content"} (after reading first)

Common Errors to Avoid:
- Don't create files without reading existing ones first
- Don't create documentation files unless explicitly requested
- Don't write to paths where parent directories don't exist"#
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
            .with_confirmation()
            .with_tags(vec!["filesystem".into(), "write".into()])
            .with_summary_key_arg("path")
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

        context.note_agent_write_intent(path.as_path()).await;
        snapshot_before_edit(context, self.name(), path.as_path()).await?;

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
            status: ToolResultStatus::Success,
            cancel_reason: None,
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
        status: ToolResultStatus::Error,
        cancel_reason: None,
        execution_time_ms: None,
        ext_info: None,
    }
}

async fn snapshot_before_edit(
    context: &TaskContext,
    tool_name: &str,
    path: &Path,
) -> ToolExecutorResult<()> {
    context
        .snapshot_file_before_edit(path)
        .await
        .map_err(|err| ToolExecutorError::ExecutionFailed {
            tool_name: tool_name.to_string(),
            error: err.to_string(),
        })
}
