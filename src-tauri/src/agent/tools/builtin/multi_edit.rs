// MultiEdit tool — atomic multi-edit for a single file.
// Applies an array of find-and-replace operations sequentially.
// If any edit fails, none are applied (atomic rollback).
// Reuses the replace pipeline from unified_edit.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::agent::core::context::AgentRunContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::tools::{RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult};

use super::file_utils::ensure_absolute;
use super::unified_edit::{
    error_result, load_file_text, replace, snapshot_before_edit, success_result, track_edit,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MultiEditArgs {
    #[serde(alias = "filePath", alias = "file_path")]
    path: String,
    edits: Vec<EditOp>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditOp {
    #[serde(alias = "oldString", alias = "old_string")]
    old_text: String,
    #[serde(alias = "newString", alias = "new_string")]
    new_text: String,
    #[serde(default, alias = "replaceAll", alias = "replace_all")]
    replace_all: bool,
}

pub struct MultiEditTool;

impl Default for MultiEditTool {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiEditTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for MultiEditTool {
    fn name(&self) -> &str {
        "multi_edit_file"
    }

    fn description(&self) -> &str {
        r#"Makes multiple edits to a single file in one atomic operation. Built on top of edit_file.
Prefer this tool over edit_file when you need to make multiple edits to the same file.

Before using this tool:
1. Use read_file to understand the file's contents and context.
2. Verify the file path is correct.

Parameters:
- path: The absolute path to the file to modify.
- edits: An array of edit operations, each with:
  - old_text: The text to replace (must match file content exactly).
  - new_text: The replacement text (must differ from old_text).
  - replace_all: If true, replaces ALL occurrences. Default: false.

IMPORTANT:
- All edits are applied in sequence, in the order provided.
- Each edit operates on the result of the previous edit.
- Atomic: if ANY edit fails, NONE are applied.
- Plan edits carefully to avoid conflicts between sequential operations."#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute path to the file to modify"
                },
                "edits": {
                    "type": "array",
                    "description": "Array of edit operations to perform sequentially",
                    "items": {
                        "type": "object",
                        "properties": {
                            "old_text": {
                                "type": "string",
                                "description": "The text to replace"
                            },
                            "new_text": {
                                "type": "string",
                                "description": "The text to replace it with (must differ from old_text)"
                            },
                            "replace_all": {
                                "type": "boolean",
                                "description": "Replace all occurrences (default false)"
                            }
                        },
                        "required": ["old_text", "new_text"]
                    }
                }
            },
            "required": ["path", "edits"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileWrite, ToolPriority::Standard)
            .with_confirmation()
            .with_tags(vec!["filesystem".into(), "edit".into()])
            .with_summary_key_arg("path")
    }

    async fn run(
        &self,
        context: &AgentRunContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: MultiEditArgs = serde_json::from_value(args)?;

        if args.edits.is_empty() {
            return Ok(error_result("edits array is empty"));
        }

        let path = match ensure_absolute(&args.path, &context.cwd) {
            Ok(resolved) => resolved,
            Err(err) => return Ok(error_result(err.to_string())),
        };

        // Must read file before editing
        if let Err(e) = context
            .file_tracker()
            .assert_file_not_modified(path.as_path())
            .await
        {
            return Ok(error_result(e.to_string()));
        }

        let original = match load_file_text(&path).await {
            Ok(text) => text,
            Err(err) => return Ok(err),
        };

        // Normalize line endings
        let normalized = original.replace("\r\n", "\n");
        let uses_crlf = original.contains("\r\n");

        // Apply edits sequentially on an in-memory copy (atomic: don't touch disk yet)
        let mut content = normalized;
        for (i, edit) in args.edits.iter().enumerate() {
            let normalized_old = edit.old_text.replace("\r\n", "\n");
            let normalized_new = edit.new_text.replace("\r\n", "\n");
            match replace(&content, &normalized_old, &normalized_new, edit.replace_all) {
                Ok(updated) => content = updated,
                Err(err_msg) => {
                    return Ok(error_result(format!(
                        "Edit {} of {} failed: {}",
                        i + 1,
                        args.edits.len(),
                        err_msg
                    )));
                }
            }
        }

        // Restore CRLF if original used it
        let final_content = if uses_crlf {
            content.replace('\n', "\r\n")
        } else {
            content
        };

        // All edits succeeded — now write to disk
        context.note_agent_write_intent(path.as_path()).await;
        snapshot_before_edit(context, self.name(), path.as_path()).await?;

        if let Err(err) = fs::write(&path, &final_content).await {
            return Ok(error_result(format!(
                "Failed to write file {}: {}",
                path.display(),
                err
            )));
        }

        track_edit(context, &path).await?;

        let edit_summaries: Vec<serde_json::Value> = args
            .edits
            .iter()
            .map(|e| {
                json!({
                    "old": e.old_text,
                    "new": e.new_text,
                    "replace_all": e.replace_all,
                })
            })
            .collect();

        Ok(success_result(
            format!(
                "multi_edit_file applied {} edits\nfile={}",
                args.edits.len(),
                path.display()
            ),
            json!({
                "file": path.display().to_string(),
                "editCount": args.edits.len(),
                "edits": edit_summaries,
            }),
        ))
    }
}
