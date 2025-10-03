/*!
 * Insert Content Tool
 * Insert a text snippet into a file at a given position, supports previewOnly.
 */

use async_trait::async_trait;
use serde::Deserialize;
use tokio::fs;

use crate::agent::context::FileOperationRecord;
use crate::agent::persistence::FileRecordSource;
use crate::agent::state::context::TaskContext;
use crate::agent::tools::{
    error::ToolExecutorResult, RunnableTool, ToolPermission, ToolResult, ToolResultContent,
};

use super::file_utils::{ensure_absolute, is_probably_binary};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InsertContentArgs {
    path: String,
    line: i64,
    content: String,
    #[serde(rename = "previewOnly", default)]
    preview_only: bool,
    #[serde(rename = "requireApproval", default)]
    require_approval: bool,
}

pub struct InsertContentTool;
impl InsertContentTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for InsertContentTool {
    fn name(&self) -> &str {
        "insert_content"
    }

    fn description(&self) -> &str {
        "Insert a text snippet into a file by line number, after a marker, or before a marker."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Absolute file path" },
                "line": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "1-based line number, or 0 to append to the end"
                },
                "content": { "type": "string", "description": "Content to insert" },
                "previewOnly": { "type": "boolean", "default": false },
                "requireApproval": { "type": "boolean", "default": false }
            },
            "required": ["path", "line", "content"]
        })
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }
    fn tags(&self) -> Vec<String> {
        vec!["file".to_string(), "insert".to_string()]
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: InsertContentArgs = serde_json::from_value(args)?;

        if args.path.trim().is_empty() {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Error {
                    message: "File path cannot be empty".to_string(),
                    details: None,
                }],
                is_error: true,
                execution_time_ms: None,
                ext_info: None,
            });
        }

        if args.line < 0 {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Error {
                    message: "Invalid line number. Must be >= 0".to_string(),
                    details: None,
                }],
                is_error: true,
                execution_time_ms: None,
                ext_info: None,
            });
        }

        let path = std::path::PathBuf::from(args.path.clone());
        if let Err(msg) = ensure_absolute(&path) {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Error {
                    message: msg,
                    details: Some(args.path.clone()),
                }],
                is_error: true,
                execution_time_ms: None,
                ext_info: None,
            });
        }

        let mut file_created = false;
        let mut original_content = String::new();

        match fs::metadata(&path).await {
            Ok(meta) => {
                if meta.is_dir() {
                    return Ok(ToolResult {
                        content: vec![ToolResultContent::Error {
                            message: format!(
                                "Path {} is a directory, cannot insert content",
                                path.display()
                            ),
                            details: Some(args.path.clone()),
                        }],
                        is_error: true,
                        execution_time_ms: None,
                        ext_info: None,
                    });
                }

                if is_probably_binary(&path) {
                    return Ok(ToolResult {
                        content: vec![ToolResultContent::Error {
                            message: format!(
                                "File {} appears to be binary, text insertion not supported",
                                path.display()
                            ),
                            details: Some(args.path.clone()),
                        }],
                        is_error: true,
                        execution_time_ms: None,
                        ext_info: None,
                    });
                }

                original_content = fs::read_to_string(&path).await?;
            }
            Err(_) => {
                file_created = true;
                if args.line > 1 {
                    return Ok(ToolResult {
                        content: vec![ToolResultContent::Error {
                            message: format!(
                                "Cannot insert content at line {} into a non-existent file. For new files, line must be 0 or 1",
                                args.line
                            ),
                            details: None,
                        }],
                        is_error: true,
                        execution_time_ms: None,
                        ext_info: None,
                    });
                }
            }
        }

        let mut lines: Vec<String> = if original_content.is_empty() {
            Vec::new()
        } else {
            original_content
                .split('\n')
                .map(|s| s.to_string())
                .collect()
        };

        let insert_index = if args.line == 0 {
            lines.len()
        } else {
            (args.line.saturating_sub(1) as usize).min(lines.len())
        };

        let insert_lines: Vec<String> = args.content.split('\n').map(|s| s.to_string()).collect();
        lines.splice(insert_index..insert_index, insert_lines.iter().cloned());
        let updated_content = lines.join("\n");

        let preview_text = format!(
            "insert_content preview\nFile: {}\nCreated: {}\nInsert at line: {}\nInserted lines: {}",
            path.display(),
            file_created,
            args.line,
            insert_lines.len()
        );

        if args.preview_only {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Text {
                    text: preview_text.clone(),
                }],
                is_error: false,
                execution_time_ms: None,
                ext_info: Some(serde_json::json!({
                    "file": path.display().to_string(),
                    "created": file_created,
                    "line": args.line,
                    "insertedLinesCount": insert_lines.len(),
                    "previewOnly": true,
                })),
            });
        }

        if args.require_approval {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Text {
                    text: "Changes were rejected by the user.".to_string(),
                }],
                is_error: false,
                execution_time_ms: None,
                ext_info: Some(serde_json::json!({
                    "file": path.display().to_string(),
                    "created": file_created,
                    "line": args.line,
                    "insertedLinesCount": insert_lines.len(),
                    "previewOnly": false,
                    "approved": false,
                })),
            });
        }

        fs::write(&path, &updated_content).await?;
        context
            .file_tracker()
            .track_file_operation(FileOperationRecord::new(
                path.as_path(),
                FileRecordSource::AgentEdited,
            ))
            .await?;

        Ok(ToolResult {
            content: vec![ToolResultContent::Text {
                text: format!(
                    "insert_content applied\nFile: {}\nInsert at line: {}\nInserted lines: {}",
                    path.display(),
                    args.line,
                    insert_lines.len()
                ),
            }],
            is_error: false,
            execution_time_ms: None,
            ext_info: Some(serde_json::json!({
                "file": path.display().to_string(),
                "created": file_created,
                "line": args.line,
                "insertedLinesCount": insert_lines.len(),
                "previewOnly": false,
                "approved": !args.require_approval,
            })),
        })
    }
}
