use std::path::PathBuf;

use async_trait::async_trait;
use regex::RegexBuilder;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::agent::context::FileOperationRecord;
use crate::agent::persistence::FileRecordSource;
use crate::agent::state::context::TaskContext;
use crate::agent::tools::{
    RunnableTool, ToolExecutorResult, ToolPermission, ToolResult, ToolResultContent,
};

use super::file_utils::{ensure_absolute, is_probably_binary};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditFileArgs {
    path: String,
    search: Option<String>,
    replace: Option<String>,
    old_string: Option<String>,
    new_string: Option<String>,
    use_regex: Option<bool>,
    ignore_case: Option<bool>,
    start_line: Option<i64>,
    end_line: Option<i64>,
    preview_only: Option<bool>,
}

pub struct EditFileTool;

impl EditFileTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Find and replace file contents with optional regex and line range controls."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute file path"
                },
                "search": {
                    "type": "string",
                    "description": "Search string (literal or regex when useRegex=true)."
                },
                "replace": {
                    "type": "string",
                    "description": "Replacement string (can be empty)."
                },
                "oldString": {
                    "type": "string",
                    "description": "Deprecated alias of search."
                },
                "newString": {
                    "type": "string",
                    "description": "Deprecated alias of replace."
                },
                "useRegex": {
                    "type": "boolean",
                    "description": "Enable regex matching."
                },
                "ignoreCase": {
                    "type": "boolean",
                    "description": "Case-insensitive matching."
                },
                "startLine": {
                    "type": "number",
                    "minimum": 1,
                    "description": "Optional 1-based start line (inclusive)."
                },
                "endLine": {
                    "type": "number",
                    "minimum": 1,
                    "description": "Optional 1-based end line (inclusive)."
                },
                "previewOnly": {
                    "type": "boolean",
                    "description": "Preview changes without writing to disk."
                }
            },
            "required": ["path"]
        })
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }

    fn tags(&self) -> Vec<String> {
        vec!["file".into(), "edit".into(), "replace".into()]
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let mut args: EditFileArgs = serde_json::from_value(args)?;
        let path = PathBuf::from(&args.path);

        if let Err(msg) = ensure_absolute(&path) {
            return Ok(validation_error(msg));
        }

        if is_probably_binary(&path) {
            return Ok(validation_error(format!(
                "File {} appears to be binary, text replacement not supported",
                path.display()
            )));
        }

        let metadata = match fs::metadata(&path).await {
            Ok(meta) => meta,
            Err(_) => {
                return Ok(validation_error(format!(
                    "File not found: {}",
                    path.display()
                )));
            }
        };

        if metadata.is_dir() {
            return Ok(validation_error(format!(
                "Path {} is a directory, cannot perform content replacement",
                path.display()
            )));
        }

        let search = match args.search.take().or_else(|| args.old_string.take()) {
            Some(value) => value,
            None => {
                return Ok(validation_error(
                    "Missing required parameter: search (or oldString)",
                ));
            }
        };
        let replace = match args.replace.take().or_else(|| args.new_string.take()) {
            Some(value) => value,
            None => {
                return Ok(validation_error(
                    "Missing required parameter: replace (or newString)",
                ));
            }
        };

        if let Some(start) = args.start_line {
            if start < 1 {
                return Ok(validation_error(
                    "startLine must be greater than or equal to 1",
                ));
            }
        }
        if let Some(end) = args.end_line {
            if end < 1 {
                return Ok(validation_error(
                    "endLine must be greater than or equal to 1",
                ));
            }
        }
        if let (Some(start), Some(end)) = (args.start_line, args.end_line) {
            if end < start {
                return Ok(validation_error(
                    "endLine must be greater than or equal to startLine",
                ));
            }
        }

        let original_content = match fs::read_to_string(&path).await {
            Ok(content) => content,
            Err(err) => {
                return Ok(tool_error(format!(
                    "Failed to read file {}: {}",
                    path.display(),
                    err
                )));
            }
        };

        context
            .file_tracker()
            .track_file_operation(FileOperationRecord::new(
                path.as_path(),
                FileRecordSource::ReadTool,
            ))
            .await?;

        let lines: Vec<String> = original_content
            .split('\n')
            .map(|s| s.to_string())
            .collect();
        let total_lines = lines.len();
        let last_index = total_lines.saturating_sub(1);

        let use_regex = args.use_regex.unwrap_or(false);
        let ignore_case = args.ignore_case.unwrap_or(false);
        let preview_only = args.preview_only.unwrap_or(false);

        let start_index = args
            .start_line
            .map(|v| v.max(1) as usize - 1)
            .unwrap_or(0)
            .min(last_index);
        let end_index = args
            .end_line
            .map(|v| v.max(1) as usize - 1)
            .unwrap_or(last_index)
            .min(last_index);

        let target_slice = if total_lines == 0 {
            Vec::<String>::new()
        } else {
            lines[start_index..=end_index].to_vec()
        };
        let target = target_slice.join("\n");

        let pattern = if use_regex {
            search.clone()
        } else {
            regex::escape(&search)
        };

        let regex = match RegexBuilder::new(&pattern)
            .case_insensitive(ignore_case)
            .build()
        {
            Ok(r) => r,
            Err(err) => {
                return Ok(validation_error(format!("Invalid regex: {}", err)));
            }
        };

        let match_count = regex.find_iter(&target).count();
        if match_count == 0 {
            let message = format!("No changes needed for '{}' (0 matches).", path.display());
            return Ok(ToolResult {
                content: vec![ToolResultContent::Text { text: message }],
                is_error: false,
                execution_time_ms: None,
                ext_info: Some(json!({
                    "file": path.display().to_string(),
                    "replacedCount": 0,
                    "useRegex": use_regex,
                    "ignoreCase": ignore_case,
                    "startLine": args.start_line,
                    "endLine": args.end_line,
                    "previewOnly": preview_only,
                })),
            });
        }

        let modified_target = regex.replace_all(&target, replace.as_str()).to_string();
        let modified_lines: Vec<String> =
            modified_target.split('\n').map(|s| s.to_string()).collect();

        let mut new_lines: Vec<String> = Vec::new();
        if start_index > 0 {
            new_lines.extend_from_slice(&lines[..start_index]);
        }
        new_lines.extend(modified_lines.clone());
        if total_lines > 0 && end_index + 1 < total_lines {
            new_lines.extend_from_slice(&lines[end_index + 1..]);
        }
        let new_content = new_lines.join("\n");

        let base_line = if total_lines == 0 { 1 } else { start_index + 1 };
        let mut affected_lines = Vec::new();
        for mat in regex.find_iter(&target) {
            let offset = target[..mat.start()] // safe slice, regex ensures UTF-8 boundaries
                .chars()
                .filter(|&ch| ch == '\n')
                .count();
            affected_lines.push(base_line + offset);
        }

        let old_snippet = regex
            .find(&target)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| search.clone());
        let new_snippet = replace.clone();

        if preview_only {
            let message = format!(
                "Preview: {} replacement(s) will be made in '{}'.\nLines affected (first 50): {}",
                match_count,
                path.display(),
                affected_lines
                    .iter()
                    .take(50)
                    .map(|line| line.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            return Ok(ToolResult {
                content: vec![ToolResultContent::Text { text: message }],
                is_error: false,
                execution_time_ms: None,
                ext_info: Some(json!({
                    "file": path.display().to_string(),
                    "replacedCount": match_count,
                    "affectedLines": affected_lines,
                    "useRegex": use_regex,
                    "ignoreCase": ignore_case,
                    "startLine": args.start_line,
                    "endLine": args.end_line,
                    "previewOnly": true,
                    "old": old_snippet,
                    "new": new_snippet,
                })),
            });
        }

        if let Err(err) = fs::write(&path, new_content).await {
            return Ok(tool_error(format!(
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

        let message = format!(
            "File edited successfully: {}\nStatus: {} replacement(s) applied.",
            path.display(),
            match_count
        );

        Ok(ToolResult {
            content: vec![ToolResultContent::Text { text: message }],
            is_error: false,
            execution_time_ms: None,
            ext_info: Some(json!({
                "file": path.display().to_string(),
                "replacedCount": match_count,
                "affectedLines": affected_lines,
                "useRegex": use_regex,
                "ignoreCase": ignore_case,
                "startLine": args.start_line,
                "endLine": args.end_line,
                "previewOnly": false,
                "old": old_snippet,
                "new": new_snippet,
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
        ext_info: None,
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
        ext_info: None,
    }
}
