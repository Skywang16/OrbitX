use std::path::{Path, PathBuf};
use std::time::Instant;

use async_trait::async_trait;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::sinks::UTF8;
use grep_searcher::Searcher;
use ignore::WalkBuilder;
use serde::Deserialize;
use serde::Serialize;
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

const DEFAULT_MAX_RESULTS: usize = 20;
const MAX_RESULTS_LIMIT: usize = 200;
const SNIPPET_MAX_LEN: usize = 300;

/// Built-in ignored directories - automatically skip these large directories during search
/// Note: If user directly specifies these directories as search paths, they can still be searched
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
struct GrepArgs {
    pattern: String,
    #[serde(
        alias = "max_results",
        default,
        deserialize_with = "lenient::deserialize_opt_usize"
    )]
    max_results: Option<usize>,
    path: Option<String>,
    include: Option<String>,
    #[serde(
        alias = "context_lines",
        default,
        deserialize_with = "lenient::deserialize_opt_usize"
    )]
    context_lines: Option<usize>,
    #[serde(alias = "case_insensitive", default)]
    ignore_case: Option<bool>,
    #[serde(alias = "output_mode")]
    output_mode: Option<String>,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrepCountEntry {
    pub file_path: String,
    pub count: usize,
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

    /// files_with_matches mode: only return file paths that contain a match
    async fn grep_files_only(
        &self,
        path: &Path,
        pattern: &str,
        max_results: usize,
        include: Option<&str>,
        ignore_case: bool,
    ) -> Result<Vec<String>, String> {
        let path = path.to_path_buf();
        let pattern = pattern.to_string();
        let include = include.map(|s| s.to_string());

        tokio::task::spawn_blocking(move || {
            Self::grep_files_only_sync(
                &path,
                &pattern,
                max_results,
                include.as_deref(),
                ignore_case,
            )
        })
        .await
        .map_err(|e| format!("Search task failed: {e}"))?
    }

    fn grep_files_only_sync(
        path: &Path,
        pattern: &str,
        max_results: usize,
        include: Option<&str>,
        ignore_case: bool,
    ) -> Result<Vec<String>, String> {
        let matcher = RegexMatcherBuilder::new()
            .case_insensitive(ignore_case)
            .build(pattern)
            .map_err(|e| format!("Invalid regex pattern: {e}"))?;

        let walker = build_walker(path, include)?;
        let mut files = Vec::with_capacity(max_results);

        for entry in walker.flatten() {
            if files.len() >= max_results {
                break;
            }
            let entry_path = entry.path();
            if entry_path.is_dir() {
                continue;
            }
            match entry_path.metadata() {
                Ok(meta) => {
                    if meta.len() > 1024 * 1024 {
                        continue;
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        path = %entry_path.display(),
                        "Failed to read metadata in grep files mode: {}",
                        err
                    );
                }
            }

            let mut found = false;
            let mut searcher = Searcher::new();
            if let Err(err) = searcher.search_path(
                &matcher,
                entry_path,
                UTF8(|_line_num, _line| {
                    found = true;
                    Ok(false) // stop after first match
                }),
            ) {
                tracing::warn!(
                    path = %entry_path.display(),
                    "grep file scan failed in files mode: {}",
                    err
                );
            }
            if found {
                files.push(entry_path.display().to_string());
            }
        }

        Ok(files)
    }

    /// count mode: return match counts per file
    async fn grep_count(
        &self,
        path: &Path,
        pattern: &str,
        max_results: usize,
        include: Option<&str>,
        ignore_case: bool,
    ) -> Result<Vec<GrepCountEntry>, String> {
        let path = path.to_path_buf();
        let pattern = pattern.to_string();
        let include = include.map(|s| s.to_string());

        tokio::task::spawn_blocking(move || {
            Self::grep_count_sync(
                &path,
                &pattern,
                max_results,
                include.as_deref(),
                ignore_case,
            )
        })
        .await
        .map_err(|e| format!("Search task failed: {e}"))?
    }

    fn grep_count_sync(
        path: &Path,
        pattern: &str,
        max_results: usize,
        include: Option<&str>,
        ignore_case: bool,
    ) -> Result<Vec<GrepCountEntry>, String> {
        let matcher = RegexMatcherBuilder::new()
            .case_insensitive(ignore_case)
            .build(pattern)
            .map_err(|e| format!("Invalid regex pattern: {e}"))?;

        let walker = build_walker(path, include)?;
        let mut entries = Vec::with_capacity(max_results);

        for entry in walker.flatten() {
            if entries.len() >= max_results {
                break;
            }
            let entry_path = entry.path();
            if entry_path.is_dir() {
                continue;
            }
            match entry_path.metadata() {
                Ok(meta) => {
                    if meta.len() > 1024 * 1024 {
                        continue;
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        path = %entry_path.display(),
                        "Failed to read metadata in grep count mode: {}",
                        err
                    );
                }
            }

            let mut count = 0usize;
            let mut searcher = Searcher::new();
            if let Err(err) = searcher.search_path(
                &matcher,
                entry_path,
                UTF8(|_line_num, _line| {
                    count += 1;
                    Ok(true)
                }),
            ) {
                tracing::warn!(
                    path = %entry_path.display(),
                    "grep file scan failed in count mode: {}",
                    err
                );
            }
            if count > 0 {
                entries.push(GrepCountEntry {
                    file_path: entry_path.display().to_string(),
                    count,
                });
            }
        }

        Ok(entries)
    }

    async fn grep_search(
        &self,
        path: &Path,
        pattern: &str,
        max_results: usize,
        include: Option<&str>,
        context_lines: usize,
        ignore_case: bool,
    ) -> Result<Vec<GrepResultEntry>, String> {
        let path = path.to_path_buf();
        let pattern = pattern.to_string();
        let include = include.map(|s| s.to_string());

        tokio::task::spawn_blocking(move || {
            Self::grep_search_sync(
                &path,
                &pattern,
                max_results,
                include.as_deref(),
                context_lines,
                ignore_case,
            )
        })
        .await
        .map_err(|e| format!("Search task failed: {e}"))?
    }

    fn grep_search_sync(
        path: &Path,
        pattern: &str,
        max_results: usize,
        include: Option<&str>,
        context_lines: usize,
        ignore_case: bool,
    ) -> Result<Vec<GrepResultEntry>, String> {
        use std::cell::RefCell;
        use std::collections::BTreeMap;

        let matcher = RegexMatcherBuilder::new()
            .case_insensitive(ignore_case)
            .build(pattern)
            .map_err(|e| format!("Invalid regex pattern: {e}"))?;

        // When context_lines > 0, we collect per-file matches first, then build snippets with context.
        // When context_lines == 0, we collect individual line matches directly.
        let results = RefCell::new(Vec::with_capacity(max_results));

        let walker = build_walker(path, include)?;

        'outer: for entry in walker.flatten() {
            if results.borrow().len() >= max_results {
                break;
            }

            let entry_path = entry.path();
            if entry_path.is_dir() {
                continue;
            }

            // Skip large files (> 1MB)
            match entry_path.metadata() {
                Ok(meta) => {
                    if meta.len() > 1024 * 1024 {
                        continue;
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        path = %entry_path.display(),
                        "Failed to read metadata in grep content mode: {}",
                        err
                    );
                }
            }

            let file_path_str = entry_path.display().to_string();
            let language = language_from_path(entry_path);

            if context_lines > 0 {
                // Collect all matches in this file first, then build context snippets
                let file_matches: RefCell<BTreeMap<usize, String>> = RefCell::new(BTreeMap::new());

                let mut searcher = Searcher::new();
                if let Err(err) = searcher.search_path(
                    &matcher,
                    entry_path,
                    UTF8(|line_num, line| {
                        file_matches
                            .borrow_mut()
                            .insert(line_num as usize, line.to_string());
                        Ok(true)
                    }),
                ) {
                    tracing::warn!(
                        path = %entry_path.display(),
                        "grep file scan failed while collecting context matches: {}",
                        err
                    );
                }

                let matches = file_matches.into_inner();
                if matches.is_empty() {
                    continue;
                }

                // Read full file for context
                let file_lines: Vec<String> = match std::fs::read_to_string(entry_path) {
                    Ok(content) => content.lines().map(|l| l.to_string()).collect(),
                    Err(_) => continue,
                };

                let mut res = results.borrow_mut();
                for &match_line in matches.keys() {
                    if res.len() >= max_results {
                        break;
                    }
                    let start = match_line.saturating_sub(context_lines).max(1);
                    let end = (match_line + context_lines).min(file_lines.len());

                    let mut snippet_parts = Vec::new();
                    for ln in start..=end {
                        if ln <= file_lines.len() {
                            let prefix = if ln == match_line { ":" } else { "-" };
                            let line_content = &file_lines[ln - 1];
                            snippet_parts.push(format!(
                                "{}{}{}",
                                ln,
                                prefix,
                                truncate_snippet(line_content.trim_end())
                            ));
                        }
                    }

                    res.push(GrepResultEntry {
                        file_path: file_path_str.clone(),
                        start_line: start,
                        end_line: end,
                        snippet: snippet_parts.join("\n"),
                        language: language.clone(),
                    });
                }
            } else {
                // Simple mode: one entry per match line
                let mut searcher = Searcher::new();
                if let Err(err) = searcher.search_path(
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
                ) {
                    tracing::warn!(
                        path = %entry_path.display(),
                        "grep file scan failed in content mode: {}",
                        err
                    );
                }
            }

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
        r#"Fast regex content search powered by ripgrep. Use this instead of shell grep/rg.

- Searches file contents using regular expressions
- Supports full regex syntax (e.g., "log.*Error", "function\s+\w+")
- Automatically respects .gitignore and skips binary files
- Filter files by glob pattern with the include parameter (e.g., "*.js", "*.{ts,tsx}")
- Use ignore_case for case-insensitive search
- Use context_lines to include surrounding lines for each match
- Use maxResults to limit output for broad patterns (default: 20, max: 200)

Output modes (outputMode parameter):
- "content" (default): Returns matching lines with snippets — use when you need to see the actual code
- "files_with_matches": Returns only file paths — much faster, saves tokens. Use this FIRST to find which files are relevant, then read_file to inspect them
- "count": Returns match counts per file — useful for gauging how spread out a pattern is

Recommended search workflow:
1. Start with outputMode="files_with_matches" to find relevant files
2. Use read_file with mode="outline" on promising files to understand structure
3. Use read_file with mode="symbol" or grep with outputMode="content" for specific code

When to use grep vs other tools:
- Use grep for exact matches: symbol names, strings, error messages, imports
- Use semantic_search for conceptual questions: "how does X work", "where is Y handled"
- Use list_files for finding files by name/location
- Use glob for finding files by name pattern
- When doing an open-ended search that may require multiple rounds of grepping, use the Task tool with the explore agent instead

Examples:
- Find which files use a symbol: {"pattern": "AgentRunExecutor", "outputMode": "files_with_matches"}
- Find function definitions: {"pattern": "fn main", "path": "/project/src"}
- Count matches: {"pattern": "TODO|FIXME", "outputMode": "count"}
- Find imports: {"pattern": "^import.*react", "ignore_case": true}
- Find with file filter: {"pattern": "Session", "include": "*.ts"}
- Find with context: {"pattern": "handleError", "context_lines": 3}"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "outputMode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"],
                    "description": "Output mode: 'content' (default) shows matching lines, 'files_with_matches' shows only file paths (fastest, saves tokens), 'count' shows match counts per file"
                },
                "maxResults": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 200,
                    "description": "Max results (default: 20, max: 200). In 'count' and 'files_with_matches' modes, limits the number of files returned."
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search (default: workspace root)"
                },
                "include": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. \"*.js\", \"*.{ts,tsx}\")"
                },
                "contextLines": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 10,
                    "description": "Number of context lines before and after each match (default: 0). Only used in 'content' mode."
                },
                "ignoreCase": {
                    "type": "boolean",
                    "description": "Case insensitive search (default: false)"
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
        context: &AgentRunContext,
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

        let ignore_case = args.ignore_case.unwrap_or(false);
        let output_mode = args.output_mode.as_deref().unwrap_or("content");
        if !matches!(output_mode, "content" | "files_with_matches" | "count") {
            return Ok(validation_error(format!(
                "Unsupported outputMode '{output_mode}'. Expected one of: content, files_with_matches, count"
            )));
        }

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

        // Execute search — all modes produce (file_paths, details, ext_info)
        let outcome: Result<GrepOutcome, String> = match output_mode {
            "files_with_matches" => self
                .grep_files_only(&search_path, pattern, max_results, args.include.as_deref(), ignore_case)
                .await
                .map(|files| {
                    let details = files.join("\n");
                    let ext = json!({ "files": &files, "totalFound": files.len(), "pattern": pattern });
                    GrepOutcome { file_paths: files, details, ext_info: ext }
                }),
            "count" => self
                .grep_count(&search_path, pattern, max_results, args.include.as_deref(), ignore_case)
                .await
                .map(|entries| {
                    let total: usize = entries.iter().map(|e| e.count).sum();
                    let file_paths: Vec<String> = entries.iter().map(|e| e.file_path.clone()).collect();
                    let details = format_count_details(&entries);
                    let ext = json!({ "counts": entries, "totalMatches": total, "totalFiles": entries.len(), "pattern": pattern });
                    GrepOutcome { file_paths, details, ext_info: ext }
                }),
            _ => {
                let context_lines = args.context_lines.unwrap_or(0).min(10);
                self.grep_search(&search_path, pattern, max_results, args.include.as_deref(), context_lines, ignore_case)
                    .await
                    .map(|entries| {
                        let file_paths: Vec<String> = entries.iter().map(|e| e.file_path.clone()).collect();
                        let details = format_result_details(&entries);
                        let ext = json!({ "results": entries, "totalFound": entries.len(), "pattern": pattern });
                        GrepOutcome { file_paths, details, ext_info: ext }
                    })
            }
        };

        let elapsed_ms = started.elapsed().as_millis() as u64;

        match outcome {
            Ok(outcome) => {
                // Track all matched files
                for file_path in &outcome.file_paths {
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
                                    "Failed to track grep file mention '{}': {}",
                                    p.display(),
                                    err
                                );
                            }
                        }
                        Err(err) => {
                            tracing::warn!(
                                "Failed to canonicalize grep file mention '{}': {}",
                                file_path,
                                err
                            );
                        }
                    }
                }

                if outcome.file_paths.is_empty() {
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

                Ok(ToolResult {
                    content: vec![ToolResultContent::Success(outcome.details)],
                    status: ToolResultStatus::Success,
                    cancel_reason: None,
                    execution_time_ms: Some(elapsed_ms),
                    ext_info: Some(outcome.ext_info),
                })
            }
            Err(err_msg) => Ok(tool_error(err_msg)),
        }
    }
}

struct GrepOutcome {
    file_paths: Vec<String>,
    details: String,
    ext_info: serde_json::Value,
}

// ============================================================================
// Helper functions
// ============================================================================

/// Shared walker builder used by all grep modes
fn build_walker(path: &Path, include: Option<&str>) -> Result<ignore::Walk, String> {
    let mut builder = WalkBuilder::new(path);
    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true);

    if let Some(glob_pattern) = include {
        let mut types_builder = ignore::types::TypesBuilder::new();
        types_builder
            .add("custom", glob_pattern)
            .map_err(|err| format!("Invalid include glob '{glob_pattern}': {err}"))?;
        types_builder.select("custom");
        let types = types_builder
            .build()
            .map_err(|err| format!("Failed to build include matcher '{glob_pattern}': {err}"))?;
        builder.types(types);
    }

    builder.filter_entry(|entry| {
        if entry.depth() == 0 {
            return true;
        }
        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        if is_dir {
            let name = entry.file_name().to_string_lossy();
            if BUILTIN_SKIP_DIRS.contains(&name.as_ref()) {
                return false;
            }
        }
        true
    });

    Ok(builder.build())
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

fn format_count_details(entries: &[GrepCountEntry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}:{}", entry.file_path, entry.count))
        .collect::<Vec<_>>()
        .join("\n")
}
