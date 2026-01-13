use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::agent::context::FileOperationRecord;
use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::persistence::FileRecordSource;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
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

impl Default for ListFilesTool {
    fn default() -> Self {
        Self::new()
    }
}

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
        r#"Lists files and directories in a given path with detailed information and smart filtering.

Usage:
- The path parameter must be an absolute path to a directory (e.g., '/Users/user/project/src')
- Returns a list of files and directories with their relative paths, sizes, and modification times
- Supports recursive listing to show all nested files and directories
- Automatically respects .gitignore patterns to avoid listing ignored files
- Hidden files (starting with .) are included by default
- Results are organized with directories first, then files, sorted alphabetically

Directory Exploration Strategy:
1. Use list_files to see what's available in a directory
2. Use read_file with mode="outline" to understand file structure
3. Use read_file with mode="symbol" to read specific functions/classes
4. Use grep to find specific patterns across multiple files

When to Use vs Other Tools:
- Use list_files for exploring directory structure and finding files
- Use grep if you know what code patterns to search for
- Use read_file after identifying interesting files to examine

Examples:
- List project root: {"path": "/Users/user/project"}
- List source directory: {"path": "/Users/user/project/src"}
- Recursive listing: {"path": "/Users/user/project/src", "recursive": true}

Common Use Cases:
- Exploring unfamiliar codebases to understand structure
- Finding configuration files, tests, or documentation
- Locating specific file types before reading them
- Understanding project organization before making changes"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute path to the directory to list. For example: '/Users/user/project/src'. Will return an error if the path doesn't exist or is not a directory."
                },
                "recursive": {
                    "type": "boolean",
                    "description": "If true, lists all files and directories recursively in the entire directory tree. If false or omitted, lists only the immediate children of the directory. Default: false."
                }
            },
            "required": ["path"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileSystem, ToolPriority::Standard)
            .with_tags(vec!["filesystem".into(), "list".into()])
            .with_summary_key_arg("path")
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: ListFilesArgs = serde_json::from_value(args)?;
        let trimmed = args.path.trim();
        if trimmed.is_empty() {
            return Ok(validation_error("Directory path cannot be empty"));
        }

        let path = match ensure_absolute(trimmed, &context.cwd) {
            Ok(resolved) => resolved,
            Err(err) => return Ok(validation_error(err.to_string())),
        };

        // 禁止列出根目录和系统敏感目录
        let path_str = path.to_string_lossy();
        if path_str == "/" {
            return Ok(validation_error(
                "Cannot list root directory '/'. Please specify a more specific directory path.",
            ));
        }

        // 禁止列出系统敏感目录
        let forbidden_paths = [
            "/System", "/Library", "/private", "/bin", "/sbin", "/usr", "/var", "/etc", "/dev",
            "/proc", "/sys",
        ];

        for forbidden in &forbidden_paths {
            if path_str == *forbidden || path_str.starts_with(&format!("{forbidden}/")) {
                return Ok(validation_error(format!(
                    "Cannot list system directory '{forbidden}'. Please specify a user directory path."
                )));
            }
        }

        let recursive = args.recursive.unwrap_or(false);
        let request_path = path.to_string_lossy().to_string();

        let response = fs_list_directory(request_path.clone(), recursive).await;
        let api_response = match response {
            Ok(resp) => resp,
            Err(err) => {
                return Ok(tool_error(format!("Directory listing failed: {err}")));
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

        context
            .file_tracker()
            .track_file_operation(FileOperationRecord::new(
                path.as_path(),
                FileRecordSource::FileMentioned,
            ))
            .await?;

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(text)],
            status: ToolResultStatus::Success,
            cancel_reason: None,
            execution_time_ms: None,
            ext_info: Some(json!({
                "path": path.display().to_string(),
                "count": entries.len(),
                "recursive": recursive,
                "entries": entries,
                "respectGitIgnore": true,
                "includeHidden": true,
                "ignoredPatterns": Vec::<String>::new(),
            })),
        })
    }
}

fn validation_error(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        status: ToolResultStatus::Error,
        cancel_reason: None,
        execution_time_ms: None,
        ext_info: None,
    }
}

fn tool_error(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        status: ToolResultStatus::Error,
        cancel_reason: None,
        execution_time_ms: None,
        ext_info: None,
    }
}
