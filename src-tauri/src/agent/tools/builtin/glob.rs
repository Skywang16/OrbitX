use std::path::{Path, PathBuf};
use std::time::Instant;

use async_trait::async_trait;
use ignore::WalkBuilder;
use serde::Deserialize;
use serde_json::json;

use super::file_utils::{ensure_absolute, lenient, normalize_path};
use crate::agent::context::FileOperationRecord;
use crate::agent::context::FileRecordSource;
use crate::agent::core::context::AgentRunContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};

const DEFAULT_MAX_RESULTS: usize = 100;
const MAX_RESULTS_LIMIT: usize = 500;

/// Built-in ignored directories
const BUILTIN_SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "dist",
    "build",
    "target",
    ".next",
    ".nuxt",
    ".output",
    ".cache",
    ".turbo",
    "__pycache__",
    ".pytest_cache",
    "venv",
    ".venv",
    "vendor",
    "coverage",
    ".nyc_output",
    "bower_components",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GlobArgs {
    pattern: String,
    path: Option<String>,
    #[serde(
        alias = "max_results",
        default,
        deserialize_with = "lenient::deserialize_opt_usize"
    )]
    max_results: Option<usize>,
}

pub struct GlobTool;

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        r#"Search for files matching a glob pattern. Works fast with codebases of any size.

- Returns matching file paths sorted by modification time (most recent first)
- Automatically respects .gitignore and skips common build directories
- Patterns not starting with "**/" are automatically prepended with "**/" to enable recursive searching

When to use:
- Use glob to find files by name pattern (e.g. all .vue files, all test files)
- Use list_files to explore a specific directory's structure
- Use grep to search file contents by text/regex

Examples:
- Find all TypeScript files: {"pattern": "*.ts"}
- Find all test files: {"pattern": "**/test_*.ts"}
- Find Vue components: {"pattern": "src/components/**/*.vue"}
- Find config files: {"pattern": "*.{json,yaml,toml}"}
- Search in specific directory: {"pattern": "*.rs", "path": "/project/src"}"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match files against. Patterns not starting with \"**/\" are automatically prepended with \"**/\" for recursive search. Examples: \"*.ts\", \"src/**/*.vue\", \"**/test_*.py\""
                },
                "path": {
                    "type": "string",
                    "description": "Optional directory to search in (default: workspace root)"
                },
                "maxResults": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 500,
                    "description": "Max results to return (default: 100, max: 500)"
                }
            },
            "required": ["pattern"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileSystem, ToolPriority::Standard).with_tags(vec![
            "filesystem".into(),
            "glob".into(),
            "search".into(),
        ])
    }

    async fn run(
        &self,
        context: &AgentRunContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: GlobArgs = serde_json::from_value(args)?;

        let pattern = args.pattern.trim();
        if pattern.is_empty() {
            return Ok(validation_error("Pattern cannot be empty"));
        }

        let max_results = args
            .max_results
            .unwrap_or(DEFAULT_MAX_RESULTS)
            .min(MAX_RESULTS_LIMIT);

        let search_path = match resolve_to_absolute(args.path.as_deref(), &context.cwd) {
            Ok(p) => p,
            Err(result) => return Ok(result),
        };

        if !search_path.exists() {
            return Ok(tool_error(format!(
                "Path does not exist: {}",
                search_path.display()
            )));
        }

        // Auto-prepend **/ if pattern doesn't start with **/ or a path separator
        let effective_pattern = if pattern.starts_with("**/") || pattern.contains('/') {
            pattern.to_string()
        } else {
            format!("**/{pattern}")
        };

        let search_path_clone = search_path.clone();
        let effective_pattern_clone = effective_pattern.clone();

        let started = Instant::now();
        let result = tokio::task::spawn_blocking(move || {
            glob_search_sync(&search_path_clone, &effective_pattern_clone, max_results)
        })
        .await
        .map_err(
            |e| crate::agent::error::ToolExecutorError::ExecutionFailed {
                tool_name: "glob".to_string(),
                error: format!("Glob search task failed: {e}"),
            },
        )?;

        let elapsed_ms = started.elapsed().as_millis() as u64;

        match result {
            Ok(mut entries) => {
                // Sort by modification time (most recent first)
                entries.sort_by(|a, b| b.1.cmp(&a.1));

                let paths: Vec<String> = entries.iter().map(|(p, _)| p.clone()).collect();

                // Track found files
                for file_path in &paths {
                    match PathBuf::from(file_path).canonicalize() {
                        Ok(p) => {
                            if let Err(err) = context
                                .file_tracker()
                                .track_file_operation(FileOperationRecord::new(
                                    p.as_path(),
                                    FileRecordSource::FileMentioned,
                                ))
                                .await
                            {
                                tracing::warn!(
                                    path = %p.display(),
                                    "Failed to track glob-discovered file: {}",
                                    err
                                );
                            }
                        }
                        Err(err) => {
                            tracing::warn!(
                                "Failed to canonicalize glob-discovered file '{}': {}",
                                file_path,
                                err
                            );
                        }
                    }
                }

                if paths.is_empty() {
                    return Ok(ToolResult {
                        content: vec![ToolResultContent::Success(format!(
                            "No files matching pattern \"{effective_pattern}\""
                        ))],
                        status: ToolResultStatus::Success,
                        cancel_reason: None,
                        execution_time_ms: Some(elapsed_ms),
                        ext_info: Some(json!({ "totalFound": 0, "pattern": effective_pattern })),
                    });
                }

                let listing = paths.join("\n");

                Ok(ToolResult {
                    content: vec![ToolResultContent::Success(listing)],
                    status: ToolResultStatus::Success,
                    cancel_reason: None,
                    execution_time_ms: Some(elapsed_ms),
                    ext_info: Some(json!({
                        "files": paths,
                        "totalFound": paths.len(),
                        "pattern": effective_pattern,
                    })),
                })
            }
            Err(err_msg) => Ok(tool_error(err_msg)),
        }
    }
}

/// Synchronous glob search using ignore::WalkBuilder + glob::Pattern matching
fn glob_search_sync(
    path: &Path,
    pattern: &str,
    max_results: usize,
) -> Result<Vec<(String, u64)>, String> {
    let matcher = glob::Pattern::new(pattern).map_err(|e| format!("Invalid glob pattern: {e}"))?;

    let mut builder = WalkBuilder::new(path);
    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            let Some(file_type) = entry.file_type() else {
                return false;
            };
            let is_dir = file_type.is_dir();
            if is_dir {
                let name = entry.file_name().to_string_lossy();
                if BUILTIN_SKIP_DIRS.contains(&name.as_ref()) {
                    return false;
                }
            }
            true
        });

    let mut results: Vec<(String, u64)> = Vec::new();

    for entry in builder.build().flatten() {
        if results.len() >= max_results {
            break;
        }

        let entry_path = entry.path();
        if entry_path.is_dir() {
            continue;
        }

        // Match against relative path from search root
        let rel = entry_path.strip_prefix(path).map_err(|err| {
            format!(
                "Failed to compute relative path for {}: {err}",
                entry_path.display()
            )
        })?;
        let rel_str = rel.to_string_lossy();

        // Also try matching just the filename
        let file_name = entry_path
            .file_name()
            .ok_or_else(|| format!("Path has no filename: {}", entry_path.display()))?
            .to_string_lossy()
            .to_string();

        if matcher.matches(&rel_str) || matcher.matches(&file_name) {
            let metadata = entry_path.metadata().map_err(|err| {
                format!(
                    "Failed to read metadata for {}: {err}",
                    entry_path.display()
                )
            })?;
            let modified = metadata.modified().map_err(|err| {
                format!(
                    "Failed to read modified time for {}: {err}",
                    entry_path.display()
                )
            })?;
            let mtime = modified
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|err| {
                    format!("Invalid modified time for {}: {err}", entry_path.display())
                })?
                .as_secs();

            results.push((entry_path.display().to_string(), mtime));
        }
    }

    Ok(results)
}

// ============================================================================
// Helpers
// ============================================================================

fn resolve_to_absolute(path: Option<&str>, cwd: &str) -> Result<PathBuf, ToolResult> {
    if let Some(raw) = path {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(validation_error("Path cannot be empty"));
        }
        ensure_absolute(trimmed, cwd).map_err(|err| validation_error(err.to_string()))
    } else {
        let base = cwd.trim();
        if base.is_empty() {
            return Err(tool_error("Working directory is not available"));
        }
        let p = Path::new(base);
        if !p.is_absolute() {
            return Err(tool_error("Working directory must be an absolute path"));
        }
        Ok(normalize_path(p))
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
