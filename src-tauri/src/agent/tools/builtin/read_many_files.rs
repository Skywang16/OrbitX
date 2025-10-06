use async_trait::async_trait;
use serde::Deserialize;
use tokio::fs;

use super::file_utils::{ensure_absolute, is_probably_binary};
use crate::agent::context::FileOperationRecord;
use crate::agent::core::context::TaskContext;
use crate::agent::persistence::FileRecordSource;
use crate::agent::tools::{
    error::ToolExecutorResult, RunnableTool, ToolCategory, ToolMetadata, ToolPermission,
    ToolPriority, ToolResult, ToolResultContent,
};

const DEFAULT_MAX_FILE_SIZE: usize = 1_048_576; // 1 MiB
const MAX_LINES_PER_FILE: usize = 2000;
const MAX_LINE_LENGTH: usize = 2000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadManyFilesArgs {
    paths: Vec<String>,
    #[serde(rename = "showLineNumbers", default)]
    show_line_numbers: bool,
    #[serde(rename = "maxFileSize", default = "default_max_file_size")]
    max_file_size: usize,
}

fn default_max_file_size() -> usize {
    DEFAULT_MAX_FILE_SIZE
}

pub struct ReadManyFilesTool;

impl ReadManyFilesTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for ReadManyFilesTool {
    fn name(&self) -> &str {
        "read_many_files"
    }

    fn description(&self) -> &str {
        "Read multiple files at once with optional size limits."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "paths": {
                    "type": "array",
                    "description": "List of absolute file paths",
                    "items": { "type": "string" },
                    "minItems": 1
                },
                "showLineNumbers": {
                    "type": "boolean",
                    "description": "Whether to show line numbers",
                    "default": false
                },
                "maxFileSize": {
                    "type": "integer",
                    "minimum": 1024,
                    "description": "Maximum file size in bytes (default 1048576)"
                }
            },
            "required": ["paths"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileRead, ToolPriority::Expensive)
            .with_tags(vec!["filesystem".into(), "batch".into()])
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: ReadManyFilesArgs = serde_json::from_value(args)?;
        let tracker = context.file_tracker();

        if args.paths.is_empty() {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Error {
                    message: "paths must be a non-empty array".to_string(),
                    details: None,
                }],
                is_error: true,
                execution_time_ms: None,
                ext_info: None,
            });
        }

        let mut summary_lines = Vec::new();
        let mut ext_files = Vec::new();

        summary_lines.push(format!(
            "Batch file read results ({} files):\n",
            args.paths.len()
        ));

        for raw_path in args.paths {
            let trimmed = raw_path.trim();
            if trimmed.is_empty() {
                return Ok(ToolResult {
                    content: vec![ToolResultContent::Error {
                        message: "All paths must be non-empty strings".to_string(),
                        details: None,
                    }],
                    is_error: true,
                    execution_time_ms: None,
                    ext_info: None,
                });
            }

            let path = match ensure_absolute(trimmed, &context.cwd) {
                Ok(resolved) => resolved,
                Err(err) => {
                    return Ok(ToolResult {
                        content: vec![ToolResultContent::Error {
                            message: err.to_string(),
                            details: Some(trimmed.to_string()),
                        }],
                        is_error: true,
                        execution_time_ms: None,
                        ext_info: None,
                    })
                }
            };

            match fs::metadata(&path).await {
                Ok(meta) => {
                    if meta.is_dir() {
                        let line = format!("Failed {}: path is a directory\n", path.display());
                        summary_lines.push(line.clone());
                        ext_files.push(serde_json::json!({
                            "path": path.display().to_string(),
                            "success": false,
                            "error": "Path is a directory"
                        }));
                        continue;
                    }

                    if meta.len() as usize > args.max_file_size {
                        let line = format!(
                            "Failed {}: File too large ({} bytes > {} bytes)\n",
                            path.display(),
                            meta.len(),
                            args.max_file_size
                        );
                        summary_lines.push(line.clone());
                        ext_files.push(serde_json::json!({
                            "path": path.display().to_string(),
                            "success": false,
                            "error": format!(
                                "File too large ({} bytes > {} bytes)",
                                meta.len(),
                                args.max_file_size
                            ),
                            "size": meta.len()
                        }));
                        continue;
                    }

                    if is_probably_binary(&path) {
                        let line =
                            format!("Failed {}: file appears to be binary\n", path.display());
                        summary_lines.push(line.clone());
                        ext_files.push(serde_json::json!({
                            "path": path.display().to_string(),
                            "success": false,
                            "error": "File appears to be binary"
                        }));
                        continue;
                    }

                    match fs::read_to_string(&path).await {
                        Ok(raw_content) => {
                            tracker
                                .track_file_operation(FileOperationRecord::new(
                                    path.as_path(),
                                    FileRecordSource::ReadTool,
                                ))
                                .await?;

                            let mut lines: Vec<String> =
                                raw_content.split('\n').map(|s| s.to_string()).collect();
                            let total_lines = lines.len();
                            let mut was_truncated = false;

                            if lines.len() > MAX_LINES_PER_FILE {
                                lines.truncate(MAX_LINES_PER_FILE);
                                was_truncated = true;
                            }

                            for line in &mut lines {
                                if line.len() > MAX_LINE_LENGTH {
                                    *line = format!("{}... [truncated]", &line[..MAX_LINE_LENGTH]);
                                    was_truncated = true;
                                }
                            }

                            if args.show_line_numbers {
                                for (idx, line) in lines.iter_mut().enumerate() {
                                    *line = format!("{:>4}  {}", idx + 1, line);
                                }
                            }

                            let mut content = lines.join("\n");
                            if was_truncated {
                                content = format!(
                                    "Important note: File content has been truncated.\nStatus: Showing first {} lines out of {} total lines.\nSuggestion: Use read_file tool with offset and limit parameters to read complete content.\n\n{}",
                                    MAX_LINES_PER_FILE,
                                    total_lines,
                                    content
                                );
                            }

                            summary_lines.push(format!(
                                "Success {} ({} bytes, {} lines)\n{}\n\n",
                                path.display(),
                                meta.len(),
                                total_lines,
                                "─".repeat(50)
                            ));
                            summary_lines.push(content.clone());
                            summary_lines.push(String::from("\n"));

                            ext_files.push(serde_json::json!({
                                "path": path.display().to_string(),
                                "success": true,
                                "size": meta.len(),
                                "lines": total_lines,
                                "truncated": was_truncated,
                                "content": content
                            }));
                        }
                        Err(err) => {
                            let line = format!("Failed {}: {}\n", path.display(), err);
                            summary_lines.push(line.clone());
                            ext_files.push(serde_json::json!({
                                "path": path.display().to_string(),
                                "success": false,
                                "error": err.to_string()
                            }));
                        }
                    }
                }
                Err(_) => {
                    let line = format!("Failed {}: File not found\n", path.display());
                    summary_lines.push(line.clone());
                    ext_files.push(serde_json::json!({
                        "path": path.display().to_string(),
                        "success": false,
                        "error": "File not found"
                    }));
                }
            }
        }

        let text_output = summary_lines.join("");

        Ok(ToolResult {
            content: vec![ToolResultContent::Text {
                text: text_output.clone(),
            }],
            is_error: false,
            execution_time_ms: None,
            ext_info: Some(serde_json::json!({
                "files": ext_files,
                "showLineNumbers": args.show_line_numbers,
                "maxFileSize": args.max_file_size,
            })),
        })
    }
}
