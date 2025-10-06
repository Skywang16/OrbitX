use std::cmp::min;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::agent::context::FileOperationRecord;
use crate::agent::core::context::TaskContext;
use crate::agent::persistence::FileRecordSource;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolExecutorResult, ToolMetadata, ToolPermission, ToolPriority,
    ToolResult, ToolResultContent,
};

use super::file_utils::{ensure_absolute, is_probably_binary};

const DEFAULT_MAX_LINES: usize = 2000;
const MAX_LINE_LENGTH: usize = 2000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadFileArgs {
    path: String,
    offset: Option<i64>,
    limit: Option<i64>,
}

pub struct ReadFileTool;

impl ReadFileTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read and display file contents. Supports line range pagination via offset/limit."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file. Must be a complete path, for example: \"/Users/user/project/src/main.ts\""
                },
                "offset": {
                    "type": "number",
                    "minimum": 0,
                    "description": "Optional: 0-based line number to start reading from."
                },
                "limit": {
                    "type": "number",
                    "minimum": 1,
                    "description": "Optional: Maximum number of lines to read."
                }
            },
            "required": ["path"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileRead, ToolPriority::Standard)
            .with_tags(vec!["filesystem".into(), "read".into()])
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: ReadFileArgs = serde_json::from_value(args)?;
        let path = match ensure_absolute(&args.path, &context.cwd) {
            Ok(resolved) => resolved,
            Err(err) => return Ok(validation_error(err.to_string())),
        };

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
                "Path {} is a directory, please use list_files tool to view directory contents",
                path.display()
            )));
        }

        if is_probably_binary(&path) {
            return Ok(validation_error(format!(
                "File {} is binary, cannot read as text",
                path.display()
            )));
        }

        let raw_bytes = match fs::read(&path).await {
            Ok(bytes) => bytes,
            Err(err) => {
                return Ok(tool_error(format!(
                    "Failed to read file {}: {}",
                    path.display(),
                    err
                )));
            }
        };

        let raw_content = match String::from_utf8(raw_bytes) {
            Ok(content) => content,
            Err(_) => {
                return Ok(validation_error(format!(
                    "File {} is binary, cannot read as text",
                    path.display()
                )));
            }
        };

        let lines: Vec<&str> = raw_content.split('\n').collect();
        let total_lines = lines.len();

        let offset = match args.offset {
            Some(v) if v < 0 => {
                return Ok(validation_error(
                    "offset must be greater than or equal to 0",
                ));
            }
            Some(v) => v as usize,
            None => 0,
        };
        let limit = match args.limit {
            Some(v) if v <= 0 => {
                return Ok(validation_error("limit must be greater than 0"));
            }
            Some(v) => v as usize,
            None => DEFAULT_MAX_LINES,
        };

        let start_line = min(offset, total_lines);
        let end_line = min(start_line.saturating_add(limit), total_lines);

        let mut output_lines = Vec::new();
        let mut truncated_line_detected = false;

        for (idx, line) in lines
            .iter()
            .enumerate()
            .skip(start_line)
            .take(end_line.saturating_sub(start_line))
        {
            let mut char_iter = line.chars();
            let mut truncated = String::new();
            for _ in 0..MAX_LINE_LENGTH {
                if let Some(ch) = char_iter.next() {
                    truncated.push(ch);
                } else {
                    break;
                }
            }
            if char_iter.next().is_some() {
                truncated.push_str("... [truncated]");
                truncated_line_detected = true;
            }
            let display_number = idx + 1; // 1-based display
            output_lines.push(format!("{:>4}  {}", display_number, truncated));
        }

        let result_text = output_lines.join("\n");

        context
            .file_tracker()
            .track_file_operation(FileOperationRecord::new(
                path.as_path(),
                FileRecordSource::ReadTool,
            ))
            .await?;

        Ok(ToolResult {
            content: vec![ToolResultContent::Text { text: result_text }],
            is_error: false,
            execution_time_ms: None,
            ext_info: Some(json!({
                "path": path.display().to_string(),
                "startLine": if total_lines == 0 { 0 } else { start_line + 1 },
                "endLine": end_line,
                "totalLines": total_lines,
                "limit": limit,
                "linesReturned": output_lines.len(),
                "hasMore": end_line < total_lines,
                "lineTruncated": truncated_line_detected,
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
