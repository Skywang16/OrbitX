use std::path::{Path, PathBuf};
use std::time::Instant;

use async_trait::async_trait;
use ck_core::{self, SearchMode};
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use tokio::fs;

use super::file_utils::{ensure_absolute, normalize_path};
use crate::agent::context::FileOperationRecord;
use crate::agent::core::context::TaskContext;
use crate::agent::persistence::FileRecordSource;
use crate::agent::error::ToolExecutorResult;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPermission, ToolPriority,
    ToolResult, ToolResultContent, ToolDescriptionContext,
};

const DEFAULT_MAX_RESULTS: usize = 10;
const MAX_RESULTS_LIMIT: usize = 50;
const SNIPPET_MAX_LEN: usize = 200;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrbitSearchArgs {
    query: String,
    max_results: Option<usize>,
    path: Option<String>,
    mode: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrbitSearchResultEntry {
    file_path: String,
    start_line: usize,
    end_line: usize,
    snippet: String,
    score: Option<f32>,
    language: String,
}

pub struct OrbitSearchTool;

impl OrbitSearchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for OrbitSearchTool {
    fn name(&self) -> &str {
        "orbit_search"
    }

    fn description(&self) -> &str {
        "Search for code snippets in the current project using semantic or hybrid matching."
    }
    
    fn description_with_context(&self, context: &ToolDescriptionContext) -> Option<String> {
        let path = Path::new(&context.cwd);
        let has_index = is_index_ready(path);
        
        if has_index {
            Some("Search for code snippets in the current project. Supports three modes: 'semantic' (AI-powered understanding of code semantics, recommended), 'hybrid' (combines semantic and keyword matching), and 'regex' (pattern-based search). The project index is ready - use semantic or hybrid mode for best results.".to_string())
        } else {
            Some("Search for code snippets in the current project. Currently, only 'regex' mode (pattern-based search) is available because no index has been built yet. To use 'semantic' and 'hybrid' intelligent search modes, please build the index first using the CK index button in the interface.".to_string())
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural language description of the desired code"
                },
                "maxResults": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 50,
                    "description": "Maximum number of results to return (default 10)"
                },
                "path": {
                    "type": "string",
                    "description": "Optional path to scope the search"
                },
                "mode": {
                    "type": "string",
                    "enum": ["semantic", "hybrid", "regex"],
                    "description": "Search mode"
                }
            },
            "required": ["query"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::CodeAnalysis, ToolPriority::Expensive)
            .with_tags(vec!["search".into(), "code".into()])
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: OrbitSearchArgs = serde_json::from_value(args)?;

        let query = args.query.trim();
        if query.is_empty() {
            return Ok(validation_error("Query cannot be empty"));
        }
        if query.len() < 3 {
            return Ok(validation_error("Query must be at least 3 characters"));
        }

        let max_results = args.max_results.unwrap_or(DEFAULT_MAX_RESULTS);
        if max_results == 0 {
            return Ok(validation_error("maxResults must be at least 1"));
        }
        if max_results > MAX_RESULTS_LIMIT {
            return Ok(validation_error("maxResults must be between 1 and 50"));
        }

        let mode = match args.mode.as_deref() {
            Some("regex") => SearchMode::Regex,
            Some("hybrid") => SearchMode::Hybrid,
            Some("semantic") | None => SearchMode::Semantic,
            Some(other) => {
                return Ok(validation_error(format!(
                    "Unsupported search mode: {}",
                    other
                )));
            }
        };

        let search_path = match resolve_to_absolute(args.path.as_deref(), &context.cwd) {
            Ok(path) => path,
            Err(result) => return Ok(result),
        };

        if !search_path.exists() {
            return Ok(tool_error(format!(
                "Search path does not exist: {}",
                search_path.display()
            )));
        }

        if requires_index(&mode) && !is_index_ready(&search_path) {
            return Ok(tool_error(
                "No semantic index found. Please build an index first using the CK index button in the chat interface.",
            ));
        }

        let options = build_search_options(&search_path, query, &mode, max_results);
        let started = Instant::now();
        let raw_results = match ck_engine::search(&options).await {
            Ok(results) => results,
            Err(err) => {
                return Ok(tool_error(format!("Search failed: {}", err)));
            }
        };

        let mut entries: Vec<OrbitSearchResultEntry> = Vec::new();
        for result in raw_results.into_iter().take(max_results) {
            let span = result.span.clone();
            let snippet = extract_content_from_span(&result.file, &span).await;
            let language = language_to_string(&result.lang);
            entries.push(OrbitSearchResultEntry {
                file_path: result.file.display().to_string(),
                start_line: span.line_start,
                end_line: span.line_end,
                snippet,
                score: if result.score > 0.0 {
                    Some(result.score)
                } else {
                    None
                },
                language,
            });
        }

        let elapsed_ms = started.elapsed().as_millis() as u64;

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
                content: vec![ToolResultContent::Text {
                    text: format!(
                        "No code found matching \"{}\". Try using different keywords or ensure the directory is indexed.",
                        query
                    ),
                }],
                is_error: false,
                execution_time_ms: Some(elapsed_ms),
                ext_info: Some(json!({
                    "results": entries,
                    "totalFound": 0,
                    "query": query,
                    "path": search_path.display().to_string(),
                    "mode": mode_as_str(&mode),
                    "searchTimeMs": elapsed_ms,
                })),
            });
        }

        let summary = format!(
            "Found {} code snippet{} matching \"{}\" ({}ms)",
            entries.len(),
            if entries.len() == 1 { "" } else { "s" },
            query,
            elapsed_ms
        );
        let details = format_result_details(&entries);

        Ok(ToolResult {
            content: vec![ToolResultContent::Text {
                text: format!("{}\n\n{}", summary, details),
            }],
            is_error: false,
            execution_time_ms: Some(elapsed_ms),
            ext_info: Some(json!({
                "results": entries,
                "totalFound": entries.len(),
                "query": query,
                "path": search_path.display().to_string(),
                "mode": mode_as_str(&mode),
                "searchTimeMs": elapsed_ms,
            })),
        })
    }
}

fn build_search_options(
    path: &Path,
    query: &str,
    mode: &SearchMode,
    max_results: usize,
) -> ck_core::SearchOptions {
    ck_core::SearchOptions {
        mode: mode.clone(),
        query: query.to_string(),
        path: path.to_path_buf(),
        top_k: Some(max_results),
        threshold: None,
        case_insensitive: true,
        whole_word: false,
        fixed_string: false,
        line_numbers: false,
        context_lines: 0,
        before_context_lines: 0,
        after_context_lines: 0,
        recursive: true,
        json_output: false,
        jsonl_output: false,
        no_snippet: false,
        reindex: false,
        show_scores: false,
        show_filenames: true,
        files_with_matches: false,
        files_without_matches: false,
        exclude_patterns: ck_core::get_default_exclude_patterns(),
        respect_gitignore: true,
        full_section: false,
        rerank: false,
        rerank_model: None,
        embedding_model: None,
    }
}

fn requires_index(mode: &SearchMode) -> bool {
    !matches!(mode, SearchMode::Regex | SearchMode::Lexical)
}

fn mode_as_str(mode: &SearchMode) -> &'static str {
    match mode {
        SearchMode::Semantic => "semantic",
        SearchMode::Hybrid => "hybrid",
        SearchMode::Regex => "regex",
        SearchMode::Lexical => "lexical",
    }
}

fn language_to_string(lang: &Option<ck_core::Language>) -> String {
    lang.map(|l| l.to_string())
        .unwrap_or_else(|| "text".to_string())
}

async fn extract_content_from_span(file: &Path, span: &ck_core::Span) -> String {
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
            let snippet = if start_idx <= end_idx {
                lines[start_idx..=end_idx].join("\n")
            } else {
                lines[start_idx].to_string()
            };
            truncate_snippet(&snippet)
        }
        Err(_) => String::new(),
    }
}

fn truncate_snippet(snippet: &str) -> String {
    if snippet.len() <= SNIPPET_MAX_LEN {
        return snippet.to_string();
    }
    format!("{}...", &snippet[..SNIPPET_MAX_LEN])
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

fn resolve_index_dir(base: &Path) -> PathBuf {
    let oxi = base.join(".oxi");
    if oxi.exists() {
        return oxi;
    }
    base.join(".ck")
}

fn is_index_ready(search_path: &Path) -> bool {
    let idx_dir = resolve_index_dir(search_path);
    if !idx_dir.exists() {
        return false;
    }

    let building_lock = idx_dir.join("building.lock");
    if building_lock.exists() {
        return false;
    }

    let ready_marker = idx_dir.join("ready.marker");
    ready_marker.exists()
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

fn format_result_details(results: &[OrbitSearchResultEntry]) -> String {
    results
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let score_text = entry
                .score
                .map(|s| format!(" ({:.1}%)", s * 100.0))
                .unwrap_or_default();
            let snippet = entry.snippet.replace('\n', "\n   ");
            format!(
                "{}. {}:{}-{}{}\n   {}",
                idx + 1,
                entry.file_path,
                entry.start_line,
                entry.end_line,
                score_text,
                snippet
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
