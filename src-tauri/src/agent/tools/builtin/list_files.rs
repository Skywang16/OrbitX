use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::agent::context::FileOperationRecord;
use crate::agent::context::FileRecordSource;
use crate::agent::core::context::AgentRunContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};
use crate::filesystem::commands::fs_list_directory;

use super::file_utils::ensure_absolute;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListFilesArgs {
    #[serde(alias = "directory_path", alias = "directoryPath")]
    path: String,
    recursive: Option<bool>,
    /// Optional array of glob patterns to ignore (e.g. ["*.test.ts", "node_modules/**"])
    #[serde(alias = "ignore_globs")]
    ignore_globs: Option<Vec<String>>,
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
        r#"Lists files and directories in a given path. Use this for understanding project structure and finding files by location.

Usage:
- The path parameter must be an absolute path to a directory
- Supports recursive listing with recursive=true
- Automatically respects .gitignore patterns and skips common build directories
- Hidden files (starting with .) are included by default
- Returns relative file/directory paths, organized with directories first, then files, sorted alphabetically
- Use ignore_globs to filter out files matching specific patterns

When to use:
- Use list_files for directory structure exploration and finding files by location
- Use grep for searching file contents by pattern
- Use glob for finding files by name pattern across the project
- Use semantic_search for conceptual questions about code

Examples:
- List directory: {"path": "/Users/user/project/src"}
- Recursive listing: {"path": "/Users/user/project/src", "recursive": true}
- With ignore patterns: {"path": "/Users/user/project/src", "recursive": true, "ignoreGlobs": ["*.test.ts", "*.spec.ts"]}"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute path to the directory to list. Will return an error if the path doesn't exist or is not a directory."
                },
                "recursive": {
                    "type": "boolean",
                    "description": "If true, lists all files and directories recursively. If false or omitted, lists only immediate children. Default: false."
                },
                "ignoreGlobs": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional array of glob patterns to ignore (e.g. [\"*.test.ts\", \"dist/**\"]). Patterns are matched against relative paths."
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
        context: &AgentRunContext,
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

        // Prohibit listing root directory and system sensitive directories
        let path_str = path.to_string_lossy();
        if path_str == "/" {
            return Ok(validation_error(
                "Cannot list root directory '/'. Please specify a more specific directory path.",
            ));
        }

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
            let message = match api_response.message {
                Some(message) => message,
                None => format!(
                    "Directory listing failed with code {} and no error message",
                    api_response.code
                ),
            };
            return Ok(tool_error(message));
        }

        let mut entries = match api_response.data {
            Some(entries) => entries,
            None => {
                return Ok(tool_error(format!(
                    "Directory listing response missing data for {}",
                    path.display()
                )));
            }
        };

        // Apply ignore_globs filtering
        let ignore_globs = args.ignore_globs.unwrap_or_default();
        if !ignore_globs.is_empty() {
            let mut matchers = Vec::with_capacity(ignore_globs.len());
            for glob in &ignore_globs {
                let matcher = match glob::Pattern::new(glob) {
                    Ok(matcher) => matcher,
                    Err(err) => {
                        return Ok(validation_error(format!(
                            "Invalid ignore glob '{glob}': {err}"
                        )));
                    }
                };
                matchers.push(matcher);
            }

            if !matchers.is_empty() {
                entries.retain(|entry| {
                    let clean = entry.trim_end_matches('/');
                    !matchers
                        .iter()
                        .any(|m| m.matches(clean) || m.matches(entry))
                });
            }
        }

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
                "ignoredPatterns": ignore_globs,
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
