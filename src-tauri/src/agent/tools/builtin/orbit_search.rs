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
use tokio::fs;

use super::file_utils::{ensure_absolute, normalize_path};
use crate::agent::context::FileOperationRecord;
use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::persistence::FileRecordSource;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolDescriptionContext, ToolMetadata, ToolPermission, ToolPriority,
    ToolResult, ToolResultContent, ToolResultStatus,
};

#[derive(Clone, Debug, PartialEq)]
enum SearchMode {
    Semantic,
    Grep,
}

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

#[derive(Debug, Clone, Serialize)]
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
        "Code search tool with two modes: grep and semantic.

Modes:
  - 'grep': Regex pattern search via ripgrep. Always available.
    Examples: 'fn main', 'class.*Controller', 'import.*from', 'TODO|FIXME'
  - 'semantic': AI-powered conceptual search. Requires vector index.
    Examples: 'authentication logic', 'error handling', 'database connection'

Usage:
  - Use grep for exact patterns (function names, class names, imports)
  - Use semantic for conceptual searches
  - Pattern syntax: ripgrep regex (same as grep -E)"
    }

    fn description_with_context(&self, context: &ToolDescriptionContext) -> Option<String> {
        let path = Path::new(&context.cwd);
        let has_index = is_index_ready(path);

        if has_index {
            Some("Code search: grep or semantic (index ready).".to_string())
        } else {
            Some("Code search: grep available. Semantic unavailable (no index).".to_string())
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search pattern. For grep: regex pattern (e.g., 'fn.*async', 'class\\s+\\w+'). For semantic: natural language (e.g., 'error handling'). Min 3 chars."
                },
                "maxResults": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 50,
                    "description": "Max results to return (default: 10)."
                },
                "path": {
                    "type": "string",
                    "description": "Optional path to scope search. Defaults to workspace root."
                },
                "mode": {
                    "type": "string",
                    "enum": ["grep", "semantic"],
                    "description": "Search mode: 'grep' (regex/ripgrep) or 'semantic' (AI search, requires index). Default: 'grep'."
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
            Some("grep") | None => SearchMode::Grep, // 默认 grep
            Some("semantic") => SearchMode::Semantic,
            Some(other) => {
                return Ok(validation_error(format!(
                    "Unsupported mode: '{}'. Use 'grep' or 'semantic'.",
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

        let started = Instant::now();
        let has_index = is_index_ready(&search_path);

        // 根据 mode 和索引状态选择搜索方式
        let result = match mode {
            SearchMode::Grep => {
                // Grep 模式：使用内嵌 ripgrep crate
                self.grep_search(&search_path, query, max_results).await
            }
            SearchMode::Semantic => {
                if has_index {
                    // 有索引：使用向量搜索
                    self.vector_search(&search_path, query, max_results).await
                } else {
                    // 无索引：自动降级到 grep 模式
                    tracing::info!(
                        "No index for {:?}, falling back to grep",
                        search_path
                    );
                    self.grep_search(&search_path, query, max_results).await
                }
            }
        };

        let elapsed_ms = started.elapsed().as_millis() as u64;

        match result {
            Ok(entries) => {
                // 记录文件操作
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

                let actual_mode = if !has_index && mode == SearchMode::Semantic {
                    "grep (fallback)"
                } else {
                    mode_as_str(&mode)
                };

                if entries.is_empty() {
                    return Ok(ToolResult {
                        content: vec![ToolResultContent::Success(format!(
                            "No code found matching \"{}\". Try different keywords or patterns.",
                            query
                        ))],
                        status: ToolResultStatus::Success,
                        cancel_reason: None,
                        execution_time_ms: Some(elapsed_ms),
                        ext_info: Some(json!({
                            "results": entries,
                            "totalFound": 0,
                            "query": query,
                            "path": search_path.display().to_string(),
                            "mode": actual_mode,
                            "hasIndex": has_index,
                            "searchTimeMs": elapsed_ms,
                        })),
                    });
                }

                let summary = format!(
                    "Found {} match{} for \"{}\" [{}] ({}ms)",
                    entries.len(),
                    if entries.len() == 1 { "" } else { "es" },
                    query,
                    actual_mode,
                    elapsed_ms
                );
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
                        "path": search_path.display().to_string(),
                        "mode": actual_mode,
                        "hasIndex": has_index,
                        "searchTimeMs": elapsed_ms,
                    })),
                })
            }
            Err(err_msg) => Ok(tool_error(err_msg)),
        }
    }
}

impl OrbitSearchTool {
    /// 使用内嵌的 ripgrep crate 进行正则搜索
    async fn grep_search(
        &self,
        path: &Path,
        pattern: &str,
        max_results: usize,
    ) -> Result<Vec<OrbitSearchResultEntry>, String> {
        let path = path.to_path_buf();
        let pattern = pattern.to_string();

        // 在阻塞线程中执行搜索（grep-searcher 是同步的）
        tokio::task::spawn_blocking(move || {
            Self::grep_search_sync(&path, &pattern, max_results)
        })
        .await
        .map_err(|e| format!("Search task failed: {}", e))?
    }

    /// 同步版本的 grep 搜索
    fn grep_search_sync(
        path: &Path,
        pattern: &str,
        max_results: usize,
    ) -> Result<Vec<OrbitSearchResultEntry>, String> {
        use std::cell::RefCell;

        // 构建正则匹配器
        let matcher = RegexMatcher::new_line_matcher(pattern)
            .map_err(|e| format!("Invalid regex pattern: {}", e))?;

        let results = RefCell::new(Vec::with_capacity(max_results));

        // 使用 ignore crate 遍历目录（自动尊重 .gitignore）
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

                    res.push(OrbitSearchResultEntry {
                        file_path: file_path_str.clone(),
                        start_line: line_num as usize,
                        end_line: line_num as usize,
                        snippet: truncate_snippet(line.trim()),
                        score: None,
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

    /// 使用向量索引进行语义搜索
    async fn vector_search(
        &self,
        path: &Path,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<OrbitSearchResultEntry>, String> {
        let global = crate::vector_db::commands::get_global_state()
            .ok_or_else(|| "Vector DB not initialized".to_string())?;

        let search_options = crate::vector_db::search::SearchOptions {
            top_k: max_results,
            threshold: 0.3,
            include_snippet: true,
            filter_languages: vec![],
        };

        let results = global
            .search_engine
            .search_in_workspace(path, query, search_options)
            .await
            .map_err(|e| format!("Vector search failed: {}", e))?;

        let mut entries: Vec<OrbitSearchResultEntry> = Vec::new();
        for r in results.into_iter().take(max_results) {
            if !path.as_os_str().is_empty() && !r.file_path.starts_with(path) {
                continue;
            }
            let span = r.span.clone();
            let snippet = extract_content_from_span(&r.file_path, &span).await;
            let language = language_from_path(&r.file_path);
            entries.push(OrbitSearchResultEntry {
                file_path: r.file_path.display().to_string(),
                start_line: span.line_start,
                end_line: span.line_end,
                snippet,
                score: Some(r.score),
                language,
            });
        }

        Ok(entries)
    }
}

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

fn mode_as_str(mode: &SearchMode) -> &'static str {
    match mode {
        SearchMode::Semantic => "semantic",
        SearchMode::Grep => "grep",
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
