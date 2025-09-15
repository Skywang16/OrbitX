/*
 * Copyright (C) 2025 OrbitX Contributors
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

/*!
 * CK 代码搜索引擎 Tauri 命令接口
 *
 * 提供语义搜索、索引管理等功能的前端调用接口
 */

use crate::terminal::commands::TerminalContextState;
use crate::utils::TauriApiResult;
use crate::{api_error, api_success};
use ck_index::EmbeddingProgress;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::State;
use tokio::task::JoinHandle;
use tracing::debug;

// 检查给定路径的索引是否就绪
fn is_index_ready(search_path: &Path) -> bool {
    let ck_dir = search_path.join(".ck");
    let embeddings = ck_dir.join("embeddings.json");
    let ann = ck_dir.join("ann_index.bin");
    let building_lock = ck_dir.join("building.lock");
    let ready_marker = ck_dir.join("ready.marker");

    ck_dir.exists()
        && embeddings.exists()
        && ann.exists()
        && ready_marker.exists()
        && !building_lock.exists()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CkSearchParams {
    pub query: String,
    pub path: String, // 搜索时路径是必需的
    pub max_results: Option<usize>,
    pub min_score: Option<f32>,
    pub language_filter: Option<String>,
    pub mode: Option<String>,
    pub full_section: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
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
    lang.map(|l| l.to_string())
        .unwrap_or_else(|| "text".to_string())
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

/// 执行代码搜索 (ck_search)
///
/// 搜索接口强制要求提供 `path` 参数。
#[tauri::command]
pub async fn ck_search(
    params: CkSearchParams,
    _terminal_state: State<'_, TerminalContextState>,
) -> TauriApiResult<Vec<CkSearchResultItem>> {
    debug!("代码搜索: query={}, path={}", params.query, params.path);

    if params.query.trim().len() < 3 {
        return Ok(api_error!("ck.invalid_query"));
    }

    let search_path = PathBuf::from(params.path);

    let mode = match params.mode.as_deref() {
        Some("regex") => ck_core::SearchMode::Regex,
        Some("lexical") => ck_core::SearchMode::Lexical,
        Some("hybrid") => ck_core::SearchMode::Hybrid,
        _ => ck_core::SearchMode::Semantic,
    };

    // 对于需要索引的模式，检查索引是否就绪
    if !matches!(
        mode,
        ck_core::SearchMode::Regex | ck_core::SearchMode::Lexical
    ) && !is_index_ready(&search_path)
    {
        debug!("索引未就绪，无法执行语义/混合搜索: {:?}", search_path);
        return Ok(api_error!("ck.index_not_found"));
    }

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
        jsonl_output: false,
        no_snippet: false,
        reindex: false, // 搜索接口不再触发重新索引
        show_scores: false,
        show_filenames: true,
        files_with_matches: false,
        files_without_matches: false,
        exclude_patterns: ck_core::get_default_exclude_patterns(),
        respect_gitignore: true,
        full_section: params.full_section.unwrap_or(false),
        rerank: false,
        rerank_model: None,
    };

    let raw_results = match ck_engine::search(&options).await {
        Ok(v) => v,
        Err(_) => return Ok(api_error!("ck.search_failed")),
    };

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
            chunk_type: if options.full_section {
                "section".into()
            } else {
                "text".into()
            },
            score: r.score,
        });
    }

    Ok(api_success!(out))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CkIndexStatusResult {
    pub is_ready: bool,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CkBuildProgress {
    pub current_file: Option<String>,
    pub files_completed: usize,
    pub total_files: usize,
    pub current_file_chunks: Option<usize>,
    pub total_chunks: usize,
    pub is_complete: bool,
    pub error: Option<String>,
}

// 全局状态管理
static BUILD_PROGRESS_STORE: OnceLock<Arc<Mutex<HashMap<String, CkBuildProgress>>>> =
    OnceLock::new();
static BUILD_TASKS: OnceLock<Arc<Mutex<HashMap<Arc<str>, JoinHandle<()>>>>> = OnceLock::new();

fn get_tasks_store() -> &'static Arc<Mutex<HashMap<Arc<str>, JoinHandle<()>>>> {
    BUILD_TASKS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

fn get_progress_store() -> &'static Arc<Mutex<HashMap<String, CkBuildProgress>>> {
    BUILD_PROGRESS_STORE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

/// 更新并持久化构建进度
fn update_build_progress(path: &str, progress: CkBuildProgress) {
    let store = get_progress_store();
    if let Ok(mut map) = store.lock() {
        map.insert(path.to_string(), progress.clone());
    }

    let ck_dir = Path::new(path).join(".ck");
    if fs::create_dir_all(&ck_dir).is_ok() {
        let progress_path = ck_dir.join("progress.json");
        if let Ok(json) = serde_json::to_string(&progress) {
            let _ = fs::write(progress_path, json);
        }
    }
}

/// 获取CK索引构建进度
///
/// 根据提供的pane_id获取对应终端的路径进行查询。
#[tauri::command]
pub async fn ck_get_build_progress(
    path: String,
    _terminal_state: State<'_, TerminalContextState>,
) -> TauriApiResult<CkBuildProgress> {
    let search_path = PathBuf::from(path);
    let path_key = search_path.display().to_string();

    let store = get_progress_store();
    let progress = store.lock().unwrap().get(&path_key).cloned();

    let final_progress = progress.unwrap_or_else(|| {
        let progress_path = search_path.join(".ck").join("progress.json");
        fs::read_to_string(progress_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_else(|| CkBuildProgress {
                current_file: None,
                files_completed: 0,
                total_files: 0,
                current_file_chunks: None,
                total_chunks: 0,
                is_complete: false,
                error: Some("progress_unavailable".into()),
            })
    });

    Ok(api_success!(final_progress))
}

/// 获取CK索引状态
///
/// 此命令会根据提供的pane_id获取对应终端的路径进行检查。
#[tauri::command]
pub async fn ck_index_status(
    path: String,
    _terminal_state: State<'_, TerminalContextState>,
) -> TauriApiResult<CkIndexStatusResult> {
    debug!("🔍 开始获取CK索引状态，路径: {}", path);

    let search_path = PathBuf::from(path);

    let is_ready = is_index_ready(&search_path);
    let path_str = search_path.display().to_string();

    debug!(
        "📊 索引状态检查结果: path={}, is_ready={}",
        path_str, is_ready
    );

    Ok(api_success!(CkIndexStatusResult {
        is_ready,
        path: path_str,
    }))
}

/// 异步构建CK索引
///
/// 根据提供的pane_id获取对应终端的路径进行构建，并立即返回。
#[tauri::command]
pub async fn ck_build_index(
    path: String,
    _terminal_state: State<'_, TerminalContextState>,
) -> TauriApiResult<()> {
    let search_path = PathBuf::from(path);
    let path_key: Arc<str> = search_path.display().to_string().into();

    if let Some(existing_task) = get_tasks_store().lock().unwrap().remove(path_key.as_ref()) {
        existing_task.abort();
        debug!("取消了正在进行的构建任务: {}", path_key);
    }

    update_build_progress(
        path_key.as_ref(),
        CkBuildProgress {
            current_file: None,
            files_completed: 0,
            total_files: 1,
            current_file_chunks: None,
            total_chunks: 0,
            is_complete: false,
            error: None,
        },
    );

    let ck_dir = search_path.join(".ck");
    let _ = fs::create_dir_all(&ck_dir);
    let building_lock = ck_dir.join("building.lock");
    let ready_marker = ck_dir.join("ready.marker");
    let _ = fs::remove_file(&ready_marker);
    let _ = fs::write(&building_lock, b"building");

    let path_key_for_task = Arc::clone(&path_key);

    let build_task = tokio::spawn(async move {
        let path_key = path_key_for_task;
        let options = ck_core::SearchOptions {
            mode: ck_core::SearchMode::Semantic,
            query: "".to_string(),
            path: search_path,
            reindex: true,
            exclude_patterns: ck_core::get_default_exclude_patterns(),
            respect_gitignore: true,
            ..Default::default()
        };

        let progress_cb_path = path_key.clone();
        let detailed_cb = Some(Box::new(move |ep: EmbeddingProgress| {
            update_build_progress(
                &progress_cb_path,
                CkBuildProgress {
                    current_file: Some(ep.file_name),
                    files_completed: ep.file_index.saturating_sub(1),
                    total_files: ep.total_files,
                    current_file_chunks: Some(ep.chunk_index),
                    total_chunks: ep.total_chunks,
                    is_complete: false,
                    error: None,
                },
            );
        }) as ck_engine::DetailedIndexingProgressCallback);

        match ck_engine::search_enhanced_with_indexing_progress(&options, None, None, detailed_cb)
            .await
        {
            Ok(_) => {
                debug!("✅ 索引构建成功: {}", path_key);
                update_build_progress(
                    &path_key,
                    CkBuildProgress {
                        current_file: None,
                        files_completed: 1,
                        total_files: 1,
                        current_file_chunks: None,
                        total_chunks: 0,
                        is_complete: true,
                        error: None,
                    },
                );
                let _ = fs::write(&ready_marker, b"ready");
            }
            Err(e) => {
                debug!("❌ 索引构建失败: {}, Error: {}", path_key, e);
                update_build_progress(
                    &path_key,
                    CkBuildProgress {
                        current_file: None,
                        files_completed: 0,
                        total_files: 0,
                        current_file_chunks: None,
                        total_chunks: 0,
                        is_complete: true,
                        error: Some(e.to_string()),
                    },
                );
            }
        }

        let _ = fs::remove_file(&building_lock);
        get_tasks_store().lock().unwrap().remove(path_key.as_ref());
        debug!("构建任务结束，已清理: {}", path_key);
    });

    get_tasks_store()
        .lock()
        .unwrap()
        .insert(path_key, build_task);

    Ok(api_success!(()))
}

/// 取消CK索引构建
///
/// 根据提供的pane_id获取对应终端的路径进行操作。
#[tauri::command]
pub async fn ck_cancel_build(
    path: String,
    _terminal_state: State<'_, TerminalContextState>,
) -> TauriApiResult<()> {
    let search_path = PathBuf::from(path);
    let path_key = search_path.display().to_string();

    if let Some(task) = get_tasks_store().lock().unwrap().remove(path_key.as_str()) {
        task.abort();
        debug!("请求中止构建任务: {}", path_key);

        update_build_progress(
            &path_key,
            CkBuildProgress {
                current_file: None,
                files_completed: 0,
                total_files: 0,
                current_file_chunks: None,
                total_chunks: 0,
                is_complete: true,
                error: Some("canceled".into()),
            },
        );

        let ck_dir = search_path.join(".ck");
        let _ = fs::remove_file(ck_dir.join("building.lock"));
        let _ = fs::remove_file(ck_dir.join("ready.marker"));
    }

    Ok(api_success!(()))
}

/// 删除CK索引
///
/// 根据提供的pane_id获取对应终端的路径进行操作。
#[tauri::command]
pub async fn ck_delete_index(
    path: String,
    _terminal_state: State<'_, TerminalContextState>,
) -> TauriApiResult<()> {
    let search_path = PathBuf::from(path);

    let path_key = search_path.display().to_string();
    if let Some(task) = get_tasks_store().lock().unwrap().remove(path_key.as_str()) {
        task.abort();
        debug!("删除索引前，取消了正在进行的构建任务: {}", &path_key);
    }

    let ck_dir = search_path.join(".ck");
    if ck_dir.exists() {
        match tokio::fs::remove_dir_all(&ck_dir).await {
            Ok(_) => {
                get_progress_store().lock().unwrap().remove(&path_key);
                debug!("✅ 成功删除CK索引: {}", path_key);
                Ok(api_success!(()))
            }
            Err(e) => {
                debug!("❌ 删除CK索引失败: {}, Error: {}", path_key, e);
                Ok(api_error!("ck.delete_failed"))
            }
        }
    } else {
        Ok(api_success!(()))
    }
}
