use std::path::{Path, PathBuf};
use std::time::Instant;

use async_trait::async_trait;
use grep_regex::RegexMatcher;
use grep_searcher::sinks::UTF8;
use grep_searcher::Searcher;
use ignore::WalkBuilder;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;

use super::file_utils::{ensure_absolute, normalize_path};
use crate::agent::context::FileOperationRecord;
use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::persistence::FileRecordSource;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};

const DEFAULT_MAX_RESULTS: usize = 10;
const MAX_RESULTS_LIMIT: usize = 50;
const SNIPPET_MAX_LEN: usize = 200;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GrepArgs {
    pattern: String,
    max_results: Option<usize>,
    path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrepResultEntry {
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub snippet: String,
    pub language: String,
}

pub struct GrepTool;

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GrepTool {
    pub fn new() -> Self {
        Self
    }

    async fn grep_search(
        &self,
        path: &Path,
        pattern: &str,
        max_results: usize,
    ) -> Result<Vec<GrepResultEntry>, String> {
        let path = path.to_path_buf();
        let pattern = pattern.to_string();

        tokio::task::spawn_blocking(move || Self::grep_search_sync(&path, &pattern, max_results))
            .await
            .map_err(|e| format!("Search task failed: {e}"))?
    }

    fn grep_search_sync(
        path: &Path,
        pattern: &str,
        max_results: usize,
    ) -> Result<Vec<GrepResultEntry>, String> {
        use std::cell::RefCell;

        let matcher = RegexMatcher::new_line_matcher(pattern)
            .map_err(|e| format!("Invalid regex pattern: {e}"))?;

        let results = RefCell::new(Vec::with_capacity(max_results));

        let walker = WalkBuilder::new(path)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();

        'outer: for entry in walker.flatten() {
            if results.borrow().len() >= max_results {
                break;
            }

            let entry_path = entry.path();
            if entry_path.is_dir() {
                continue;
            }

            // 跳过大文件 (> 1MB)
            if let Ok(meta) = entry_path.metadata() {
                if meta.len() > 1024 * 1024 {
                    continue;
                }
            }

            let file_path_str = entry_path.display().to_string();
            let language = language_from_path(entry_path);

            let mut searcher = Searcher::new();
            let _ = searcher.search_path(
                &matcher,
                entry_path,
                UTF8(|line_num, line| {
                    let mut res = results.borrow_mut();
                    if res.len() >= max_results {
                        return Ok(false);
                    }

                    res.push(GrepResultEntry {
                        file_path: file_path_str.clone(),
                        start_line: line_num as usize,
                        end_line: line_num as usize,
                        snippet: truncate_snippet(line.trim()),
                        language: language.clone(),
                    });

                    Ok(res.len() < max_results)
                }),
            );

            if results.borrow().len() >= max_results {
                break 'outer;
            }
        }

        Ok(results.into_inner())
    }
}

#[async_trait]
impl RunnableTool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        r#"Fast regex code search powered by ripgrep. Use instead of shell grep/rg.

Usage:
- Provide a regex pattern to search across all code files
- Automatically respects .gitignore and skips binary files
- Returns file paths, line numbers, and matching snippets

Search Strategy:
1. Use grep to find code patterns, function calls, or definitions
2. Use read_file to examine the matching files in detail
3. Use list_files if you need to explore directory structure first

When to Use vs Other Tools:
- Use grep for finding specific patterns (function names, imports, TODOs)
- Use semantic_search for finding code by describing what it does
- Use read_file with mode="symbol" to read specific functions after finding them

Examples:
- Find function definitions: {"pattern": "fn main", "path": "/project/src"}
- Find TODOs: {"pattern": "TODO|FIXME"}
- Find imports: {"pattern": "^import.*react"}
- Find usages: {"pattern": "getUserById"}

Tips:
- Keep patterns simple, avoid over-escaping special characters
- Use maxResults to limit output for broad patterns"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "maxResults": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 50,
                    "description": "Max results (default: 10)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search (default: workspace root)"
                }
            },
            "required": ["pattern"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::CodeAnalysis, ToolPriority::Standard).with_tags(vec![
            "search".into(),
            "grep".into(),
            "regex".into(),
        ])
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: GrepArgs = serde_json::from_value(args)?;

        let pattern = args.pattern.trim();
        if pattern.is_empty() {
            return Ok(validation_error("Pattern cannot be empty"));
        }

        let max_results = args
            .max_results
            .unwrap_or(DEFAULT_MAX_RESULTS)
            .min(MAX_RESULTS_LIMIT);

        let search_path = match resolve_to_absolute(args.path.as_deref(), &context.cwd) {
            Ok(path) => path,
            Err(result) => return Ok(result),
        };

        if !search_path.exists() {
            return Ok(tool_error(format!(
                "Path does not exist: {}",
                search_path.display()
            )));
        }

        let started = Instant::now();
        let result = self.grep_search(&search_path, pattern, max_results).await;
        let elapsed_ms = started.elapsed().as_millis() as u64;

        match result {
            Ok(entries) => {
                for entry in &entries {
                    if let Ok(p) = PathBuf::from(&entry.file_path).canonicalize() {
                        let _ = context
                            .file_tracker()
                            .track_file_operation(FileOperationRecord::new(
                                p.as_path(),
                                FileRecordSource::FileMentioned,
                            ))
                            .await;
                    }
                }

                if entries.is_empty() {
                    return Ok(ToolResult {
                        content: vec![ToolResultContent::Success(format!(
                            "No matches for pattern \"{pattern}\""
                        ))],
                        status: ToolResultStatus::Success,
                        cancel_reason: None,
                        execution_time_ms: Some(elapsed_ms),
                        ext_info: Some(json!({ "totalFound": 0, "pattern": pattern })),
                    });
                }

                let summary = format!("Found {} matches ({}ms)", entries.len(), elapsed_ms);
                let details = format_result_details(&entries);

                Ok(ToolResult {
                    content: vec![ToolResultContent::Success(format!(
                        "{summary}\n\n{details}"
                    ))],
                    status: ToolResultStatus::Success,
                    cancel_reason: None,
                    execution_time_ms: Some(elapsed_ms),
                    ext_info: Some(json!({
                        "results": entries,
                        "totalFound": entries.len(),
                        "pattern": pattern,
                    })),
                })
            }
            Err(err_msg) => Ok(tool_error(err_msg)),
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

fn language_from_path(path: &Path) -> String {
    use crate::vector_db::core::Language;
    match Language::from_path(path) {
        Some(Language::Rust) => "rust".into(),
        Some(Language::TypeScript) => "typescript".into(),
        Some(Language::JavaScript) => "javascript".into(),
        Some(Language::Python) => "python".into(),
        Some(Language::Go) => "go".into(),
        Some(Language::Java) => "java".into(),
        Some(Language::C) => "c".into(),
        Some(Language::Cpp) => "cpp".into(),
        Some(Language::CSharp) => "csharp".into(),
        Some(Language::Ruby) => "ruby".into(),
        Some(Language::Php) => "php".into(),
        Some(Language::Swift) => "swift".into(),
        Some(Language::Kotlin) => "kotlin".into(),
        None => "text".into(),
    }
}

fn truncate_snippet(snippet: &str) -> String {
    crate::agent::utils::truncate_with_ellipsis(snippet, SNIPPET_MAX_LEN)
}

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
        let path = Path::new(base);
        if !path.is_absolute() {
            return Err(tool_error("Working directory must be an absolute path"));
        }
        Ok(normalize_path(path))
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

fn format_result_details(results: &[GrepResultEntry]) -> String {
    results
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let snippet = entry.snippet.replace('\n', "\n   ");
            format!(
                "{}. {}:{}\n   {}",
                idx + 1,
                entry.file_path,
                entry.start_line,
                snippet
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
