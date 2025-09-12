use crate::{api_error, api_success};
use crate::utils::TauriApiResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct CkSearchParams {
    pub query: String,
    pub max_results: Option<usize>,
    pub min_score: Option<f32>,
    pub directory: Option<String>,
    pub language_filter: Option<String>,
    pub mode: Option<String>,
    pub full_section: Option<bool>,
    pub reindex: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CkSearchResultItem {
    pub file_path: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub language: String,
    pub chunk_type: String,
    pub score: f32,
}

fn language_to_str(lang: &Option<ck_core::Language>) -> String {
    lang.map(|l| l.to_string()).unwrap_or_else(|| "text".to_string())
}

async fn extract_content_from_span(file: &std::path::Path, span: &ck_core::Span) -> String {
    match tokio::fs::read_to_string(file).await {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            if span.line_start == 0 || span.line_start > lines.len() {
                return String::new();
            }
            let start_idx = span.line_start - 1;
            let end_idx = (span.line_end.saturating_sub(1)).min(lines.len().saturating_sub(1));
            if start_idx <= end_idx {
                lines[start_idx..=end_idx].join("\n")
            } else {
                lines[start_idx].to_string()
            }
        }
        Err(_) => String::new(),
    }
}

#[tauri::command]
pub async fn code_search(params: CkSearchParams) -> TauriApiResult<Vec<CkSearchResultItem>> {
    // Validate
    if params.query.trim().len() < 3 {
        return Ok(api_error!("common.invalid_params"));
    }

    // Resolve search path
    let search_path = if let Some(dir) = params.directory.as_deref() {
        PathBuf::from(dir)
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    // Build search options
    let mode = match params.mode.as_deref() {
        Some("regex") => ck_core::SearchMode::Regex,
        Some("lexical") => ck_core::SearchMode::Lexical,
        Some("hybrid") => ck_core::SearchMode::Hybrid,
        _ => ck_core::SearchMode::Semantic,
    };

    let options = ck_core::SearchOptions {
        mode,
        query: params.query.trim().to_string(),
        path: search_path.clone(),
        top_k: params.max_results,
        threshold: params.min_score,
        case_insensitive: true,
        whole_word: false,
        fixed_string: false,
        line_numbers: false,
        context_lines: 0,
        before_context_lines: 0,
        after_context_lines: 0,
        recursive: true,
        json_output: false,
        reindex: params.reindex.unwrap_or(false),
        show_scores: false,
        show_filenames: true,
        files_with_matches: false,
        files_without_matches: false,
        exclude_patterns: ck_core::get_default_exclude_patterns(),
        respect_gitignore: true,
        full_section: params.full_section.unwrap_or(false),
    };

    // Execute search (auto-updates index for semantic/hybrid)
    let raw_results = match ck_engine::search(&options).await {
        Ok(v) => v,
        Err(_e) => {
            return Ok(api_error!("common.operation_failed"));
        }
    };

    // Optional language filter (post-filter)
    let lang_filter_norm = params
        .language_filter
        .as_ref()
        .map(|s| s.trim().to_lowercase());

    let mut out = Vec::with_capacity(raw_results.len());

    for r in raw_results {
        if let Some(ref lf) = lang_filter_norm {
            let lang_str = language_to_str(&r.lang);
            if &lang_str != lf {
                continue;
            }
        }

        let file_path = r.file;
        let span = r.span.clone();
        let content = extract_content_from_span(&file_path, &span).await;
        let language = language_to_str(&r.lang);

        out.push(CkSearchResultItem {
            file_path: file_path.display().to_string(),
            content,
            start_line: span.line_start,
            end_line: span.line_end,
            language,
            chunk_type: if options.full_section { "section".into() } else { "text".into() },
            score: r.score,
        });
    }

    Ok(api_success!(out))
}
