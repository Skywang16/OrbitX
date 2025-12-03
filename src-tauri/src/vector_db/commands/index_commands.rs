use crate::utils::{EmptyData, TauriApiResult};
use crate::vector_db::commands::VectorDbState;
use crate::{api_error, api_success};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tokio::task::JoinHandle;
use tracing::warn;

#[derive(Debug, serde::Serialize)]
pub struct IndexResult {
    pub indexed_files: usize,
    pub total_files: usize,
    pub total_chunks: usize,
}

#[tauri::command]
pub async fn index_files(
    paths: Vec<String>,
    state: State<'_, VectorDbState>,
) -> TauriApiResult<IndexResult> {
    let before = state.index_manager.get_status();

    let file_paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let embedder = state.search_engine.embedder();

    match state
        .index_manager
        .index_files_with(&file_paths, &*embedder, &state.vector_index)
        .await
    {
        Ok(_) => {
            let after = state.index_manager.get_status();
            Ok(api_success!(IndexResult {
                indexed_files: after.total_files.saturating_sub(before.total_files),
                total_files: after.total_files,
                total_chunks: after.total_chunks,
            }))
        }
        Err(e) => {
            warn!(error = %e, "索引文件失败");
            Ok(api_error!("vector_db.index_failed"))
        }
    }
}

#[tauri::command]
pub async fn update_file_index(
    path: String,
    state: State<'_, VectorDbState>,
) -> TauriApiResult<EmptyData> {
    let p = PathBuf::from(&path);
    let embedder = state.search_engine.embedder();
    match state
        .index_manager
        .update_index(&p, &*embedder, &state.vector_index)
        .await
    {
        Ok(_) => Ok(api_success!(EmptyData::default())),
        Err(e) => {
            warn!(error = %e, path = %path, "更新文件索引失败");
            Ok(api_error!("vector_db.update_failed"))
        }
    }
}

#[tauri::command]
pub async fn remove_file_index(
    path: String,
    state: State<'_, VectorDbState>,
) -> TauriApiResult<EmptyData> {
    let p = PathBuf::from(&path);
    match state.index_manager.remove_file(&p) {
        Ok(_) => Ok(api_success!(EmptyData::default())),
        Err(e) => {
            warn!(error = %e, path = %path, "移除文件索引失败");
            Ok(api_error!("vector_db.remove_failed"))
        }
    }
}

// ---------------- Progress + Cancel Support ----------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VectorBuildProgress {
    pub current_file: Option<String>,
    pub files_completed: usize,
    pub total_files: usize,
    pub current_file_chunks: Option<usize>,
    pub total_chunks: usize,
    pub is_complete: bool,
    pub error: Option<String>,
}

static BUILD_PROGRESS: once_cell::sync::OnceCell<Arc<Mutex<HashMap<String, VectorBuildProgress>>>> =
    once_cell::sync::OnceCell::new();
static BUILD_TASKS: once_cell::sync::OnceCell<Arc<Mutex<HashMap<String, JoinHandle<()>>>>> =
    once_cell::sync::OnceCell::new();

fn progress_store() -> &'static Arc<Mutex<HashMap<String, VectorBuildProgress>>> {
    BUILD_PROGRESS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

fn tasks_store() -> &'static Arc<Mutex<HashMap<String, JoinHandle<()>>>> {
    BUILD_TASKS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

fn set_progress(path: &str, p: VectorBuildProgress) {
    let store = progress_store();
    store.lock().insert(path.to_string(), p);
}

#[tauri::command]
pub async fn vector_get_build_progress(path: String) -> TauriApiResult<VectorBuildProgress> {
    let store = progress_store();
    if let Some(p) = store.lock().get(&path).cloned() {
        Ok(api_success!(p))
    } else {
        Ok(api_success!(VectorBuildProgress {
            current_file: None,
            files_completed: 0,
            total_files: 0,
            current_file_chunks: None,
            total_chunks: 0,
            is_complete: true,
            error: Some("progress_unavailable".into()),
        }))
    }
}

#[tauri::command]
pub async fn vector_cancel_build(path: String) -> TauriApiResult<EmptyData> {
    let task = tasks_store().lock().remove(&path);
    if let Some(handle) = task {
        handle.abort();
    }
    progress_store().lock().remove(&path);
    Ok(api_success!(EmptyData::default()))
}

#[tauri::command]
pub async fn vector_build_index(
    path: String,
    state: State<'_, VectorDbState>,
) -> TauriApiResult<EmptyData> {
    let root = PathBuf::from(&path);
    let embedder = state.search_engine.embedder();

    // 为当前工作区创建独立的 IndexManager
    let workspace_index_manager = match crate::vector_db::storage::IndexManager::new(
        &root,
        state.search_engine.config().clone(),
    ) {
        Ok(manager) => std::sync::Arc::new(manager),
        Err(e) => {
            tracing::error!("创建工作区索引管理器失败: {}", e);
            return Ok(api_error!("vector_db.index_failed"));
        }
    };

    // Abort existing
    if let Some(existing) = tasks_store().lock().remove(&path) {
        existing.abort();
    }

    set_progress(
        &path,
        VectorBuildProgress {
            current_file: None,
            files_completed: 0,
            total_files: 0,
            current_file_chunks: None,
            total_chunks: 0,
            is_complete: false,
            error: None,
        },
    );

    // Spawn background task
    let path_key = path.clone();
    let index_manager = workspace_index_manager.clone();
    let vector_index = state.vector_index.clone();
    let config = state.search_engine.config().clone();

    let handle = tokio::spawn(async move {
        // Collect files under root
        let files = crate::vector_db::utils::collect_source_files(&root, config.max_file_size);
        set_progress(
            &path_key,
            VectorBuildProgress {
                current_file: None,
                files_completed: 0,
                total_files: files.len(),
                current_file_chunks: None,
                total_chunks: 0,
                is_complete: false,
                error: None,
            },
        );

        let mut completed = 0usize;
        for f in files {
            let name = f
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            set_progress(
                &path_key,
                VectorBuildProgress {
                    current_file: Some(name),
                    files_completed: completed,
                    total_files: completed + 1,
                    current_file_chunks: None,
                    total_chunks: 0,
                    is_complete: false,
                    error: None,
                },
            );
            if let Err(e) = index_manager
                .index_file_with(&f, &*embedder, &vector_index)
                .await
            {
                set_progress(
                    &path_key,
                    VectorBuildProgress {
                        current_file: Some(f.display().to_string()),
                        files_completed: completed,
                        total_files: completed + 1,
                        current_file_chunks: None,
                        total_chunks: 0,
                        is_complete: false,
                        error: Some(e.to_string()),
                    },
                );
            }
            completed += 1;
        }

        set_progress(
            &path_key,
            VectorBuildProgress {
                current_file: None,
                files_completed: completed,
                total_files: completed,
                current_file_chunks: None,
                total_chunks: 0,
                is_complete: true,
                error: None,
            },
        );
        tasks_store().lock().remove(&path_key);
    });

    tasks_store().lock().insert(path, handle);
    Ok(api_success!(EmptyData::default()))
}
