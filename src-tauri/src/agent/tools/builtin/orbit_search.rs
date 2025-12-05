use std::path::{Path, PathBuf};
use std::time::Instant;

use async_trait::async_trait;

#[derive(Clone, Debug)]
enum SearchMode {
    Semantic,
    Hybrid,
    Regex,
}
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
    ToolResult, ToolResultContent,
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
        "A powerful semantic and pattern-based code search tool.

Usage:
  - ALWAYS use orbit_search for finding code in the codebase. NEVER invoke `grep` or `find` as a shell command for code searching.
  - Supports three search modes: 'semantic' (AI-powered, requires index), 'hybrid' (semantic + keyword, requires index), 'regex' (pattern-based, always available)
  - Returns file paths, line ranges, code snippets, and relevance scores
  - Automatically respects .gitignore patterns
  - Use semantic/hybrid modes for conceptual searches (e.g., 'authentication logic', 'database connection')
  - Use regex mode for exact patterns (e.g., 'function\\s+\\w+', 'class.*Controller')
  - Pattern syntax: Uses ripgrep regex (not grep) - literal braces need escaping
  - You have the capability to call multiple tools in a single response. It is always better to speculatively perform multiple searches as a batch that are potentially useful."
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
                    "description": "The search query. For semantic/hybrid modes: use natural language (e.g., 'file upload handler', 'authentication middleware'). For regex mode: use ripgrep regex syntax (e.g., 'function\\s+\\w+', 'class.*Component'). Query must be at least 3 characters."
                },
                "maxResults": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 50,
                    "description": "Maximum number of results to return (default: 10, max: 50). Use lower numbers (5-10) for focused searches, higher numbers (20-50) when exploring."
                },
                "path": {
                    "type": "string",
                    "description": "Optional absolute path to scope the search to a specific directory or file. For example: '/Users/user/project/src/components'. If omitted, searches the entire workspace."
                },
                "mode": {
                    "type": "string",
                    "enum": ["semantic", "hybrid", "regex"],
                    "description": "Search mode: 'semantic' for AI-powered concept search (requires index), 'hybrid' for combined semantic+keyword (requires index), 'regex' for pattern matching (always available). Default: 'semantic'. Use 'hybrid' for best results."
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

        let started = Instant::now();

        // 使用工作区路径加载向量索引进行搜索
        let global = match crate::vector_db::commands::get_global_state() {
            Some(g) => g,
            None => return Ok(tool_error("Vector DB not initialized")),
        };

        let config = global.search_engine.config().clone();

        // 为工作区创建 IndexManager 并加载向量索引
        let index_manager =
            match crate::vector_db::storage::IndexManager::new(&search_path, config.clone()) {
                Ok(manager) => manager,
                Err(e) => return Ok(tool_error(format!("Failed to load index: {}", e))),
            };

        // 获取所有 chunk 元数据并加载向量到内存
        let chunk_metadata_vec = index_manager.get_all_chunk_metadata();
        if chunk_metadata_vec.is_empty() {
            return Ok(tool_error(format!(
                "No code found matching \"{}\". Try using different keywords or ensure the directory is indexed.",
                query
            )));
        }

        // 转换为HashMap
        let chunk_metadata: std::collections::HashMap<_, _> = chunk_metadata_vec.into_iter().collect();

        let vector_index = match crate::vector_db::index::VectorIndex::load(
            index_manager.store(),
            &chunk_metadata,
            config.embedding.dimension,
        )
        .await
        {
            Ok(index) => index,
            Err(e) => return Ok(tool_error(format!("Failed to load vectors: {}", e))),
        };

        // 生成查询向量
        let embedder = global.search_engine.embedder();
        let query_embedding = match embedder.embed(&[query]).await {
            Ok(embeddings) if !embeddings.is_empty() => embeddings.into_iter().next().unwrap(),
            Ok(_) => return Ok(tool_error("Failed to generate query embedding")),
            Err(e) => return Ok(tool_error(format!("Embedding failed: {}", e))),
        };

        // 执行向量搜索
        let results = match vector_index.search(&query_embedding, max_results, 0.3) {
            Ok(r) => r,
            Err(e) => return Ok(tool_error(format!("Search failed: {}", e))),
        };

        // 转换结果
        let results: Vec<crate::vector_db::core::SearchResult> = results
            .into_iter()
            .filter_map(|(chunk_id, score)| {
                vector_index.get_chunk_metadata(&chunk_id).map(|metadata| {
                    crate::vector_db::core::SearchResult::new(
                        metadata.file_path.clone(),
                        metadata.span.clone(),
                        score,
                        format!("Chunk {:?}", metadata.chunk_type),
                        None,
                        Some(metadata.chunk_type),
                    )
                })
            })
            .collect();

        let mut entries: Vec<OrbitSearchResultEntry> = Vec::new();
        for r in results.into_iter().take(max_results) {
            // Scope to path if provided
            if !search_path.as_os_str().is_empty() && !r.file_path.starts_with(&search_path) {
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
                content: vec![ToolResultContent::Success(format!(
                    "No code found matching \"{}\". Try using different keywords or ensure the directory is indexed.",
                    query
                ))],
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
            content: vec![ToolResultContent::Success(format!(
                "{}\n\n{}",
                summary, details
            ))],
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
        SearchMode::Hybrid => "hybrid",
        SearchMode::Regex => "regex",
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
        is_error: true,
        execution_time_ms: None,
        ext_info: None,
    }
}

fn tool_error(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
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
