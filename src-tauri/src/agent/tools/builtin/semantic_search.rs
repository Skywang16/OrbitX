use std::path::{Path, PathBuf};
use std::time::Instant;

use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use tokio::fs;

use super::file_utils::{ensure_absolute, normalize_path};
use crate::agent::context::FileOperationRecord;
use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::persistence::FileRecordSource;
use crate::agent::tools::{
    RunnableTool, ToolAvailabilityContext, ToolCategory, ToolMetadata, ToolPriority, ToolResult,
    ToolResultContent, ToolResultStatus,
};
use crate::vector_db::search::SemanticSearchEngine;
use std::sync::Arc;

const DEFAULT_MAX_RESULTS: usize = 10;
const MAX_RESULTS_LIMIT: usize = 50;
const SNIPPET_MAX_LEN: usize = 200;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SemanticSearchArgs {
    query: String,
    max_results: Option<usize>,
    path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticResultEntry {
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub snippet: String,
    pub score: f32,
    pub language: String,
}

pub struct SemanticSearchTool {
    search_engine: Arc<SemanticSearchEngine>,
}

impl SemanticSearchTool {
    pub fn new(search_engine: Arc<SemanticSearchEngine>) -> Self {
        Self { search_engine }
    }

    async fn vector_search(
        &self,
        path: &Path,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<SemanticResultEntry>, String> {
        let search_options = crate::vector_db::search::SearchOptions {
            top_k: max_results,
            threshold: 0.3,
            include_snippet: true,
            filter_languages: vec![],
        };

        let results = self
            .search_engine
            .search_in_workspace(path, query, search_options)
            .await
            .map_err(|e| format!("Vector search failed: {}", e))?;

        let mut entries: Vec<SemanticResultEntry> = Vec::new();
        for r in results.into_iter().take(max_results) {
            if !path.as_os_str().is_empty() && !r.file_path.starts_with(path) {
                continue;
            }
            let span = r.span.clone();
            let snippet = extract_content_from_span(&r.file_path, &span).await;
            let language = language_from_path(&r.file_path);
            entries.push(SemanticResultEntry {
                file_path: r.file_path.display().to_string(),
                start_line: span.line_start,
                end_line: span.line_end,
                snippet,
                score: r.score,
                language,
            });
        }

        Ok(entries)
    }
}

#[async_trait]
impl RunnableTool for SemanticSearchTool {
    fn name(&self) -> &str {
        "semantic_search"
    }

    fn is_available(&self, ctx: &ToolAvailabilityContext) -> bool {
        ctx.has_vector_index
    }

    fn description(&self) -> &str {
        r#"AI-powered semantic code search. Find code by describing what it does.

Usage:
- Describe the functionality you're looking for in natural language
- Returns relevant code snippets ranked by semantic similarity

Search Strategy:
1. Use semantic_search when you don't know exact patterns or names
2. Use read_file to examine the matching files in detail
3. Use grep if you need exact pattern matching

When to Use vs Other Tools:
- Use semantic_search for "find code that handles X" queries
- Use grep for exact patterns like function names or imports
- Use list_files to explore directory structure

Examples:
- {"query": "error handling and retry logic"}
- {"query": "database connection pooling"}
- {"query": "user authentication flow"}
- {"query": "file upload validation"}"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural language description of the code you're looking for"
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
            "required": ["query"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::CodeAnalysis, ToolPriority::Expensive).with_tags(vec![
            "search".into(),
            "semantic".into(),
            "ai".into(),
        ])
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: SemanticSearchArgs = serde_json::from_value(args)?;

        let query = args.query.trim();
        if query.is_empty() {
            return Ok(validation_error("Query cannot be empty"));
        }
        if query.len() < 3 {
            return Ok(validation_error("Query must be at least 3 characters"));
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
        let result = self.vector_search(&search_path, query, max_results).await;
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
                            "No code found matching \"{}\"",
                            query
                        ))],
                        status: ToolResultStatus::Success,
                        cancel_reason: None,
                        execution_time_ms: Some(elapsed_ms),
                        ext_info: Some(json!({ "totalFound": 0, "query": query })),
                    });
                }

                let summary = format!("Found {} results ({}ms)", entries.len(), elapsed_ms);
                let details = format_result_details(&entries);

                Ok(ToolResult {
                    content: vec![ToolResultContent::Success(format!(
                        "{}\n\n{}",
                        summary, details
                    ))],
                    status: ToolResultStatus::Success,
                    cancel_reason: None,
                    execution_time_ms: Some(elapsed_ms),
                    ext_info: Some(json!({
                        "results": entries,
                        "totalFound": entries.len(),
                        "query": query,
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

async fn extract_content_from_span(file: &Path, span: &crate::vector_db::core::Span) -> String {
    match fs::read_to_string(file).await {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            if span.line_start == 0 || span.line_start > lines.len() {
                return String::new();
            }
            let start_idx = span.line_start.saturating_sub(1);
            let end_idx = span
                .line_end
                .saturating_sub(1)
                .min(lines.len().saturating_sub(1));
            let snippet = if start_idx <= end_idx && end_idx < lines.len() {
                lines
                    .get(start_idx..=end_idx)
                    .map(|l| l.join("\n"))
                    .unwrap_or_default()
            } else if start_idx < lines.len() {
                lines
                    .get(start_idx)
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            truncate_snippet(&snippet)
        }
        Err(_) => String::new(),
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

fn format_result_details(results: &[SemanticResultEntry]) -> String {
    results
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let score_text = format!(" ({:.0}%)", entry.score * 100.0);
            let snippet = entry.snippet.replace('\n', "\n   ");
            format!(
                "{}. {}:{}{}\n   {}",
                idx + 1,
                entry.file_path,
                entry.start_line,
                score_text,
                snippet
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
