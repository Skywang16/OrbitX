use crate::utils::{EmptyData, TauriApiResult};
use crate::vector_db::commands::VectorDbState;
use crate::{api_error, api_success};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tokio::task::JoinHandle;
use tracing::{error, warn};

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
pub async fn get_index_status(
    path: String,
    state: State<'_, VectorDbState>,
) -> TauriApiResult<crate::vector_db::storage::IndexStatus> {
    let workspace_path = PathBuf::from(&path);

    if !workspace_path.join(".oxi").exists() {
        return Ok(api_success!(crate::vector_db::storage::IndexStatus {
            total_files: 0,
            total_chunks: 0,
            embedding_model: String::new(),
            vector_dimension: 0,
        }));
    }

    let config = state.search_engine.config().clone();
    match crate::vector_db::storage::IndexManager::new(&workspace_path, config) {
        Ok(manager) => Ok(api_success!(manager.get_status())),
        Err(e) => {
            warn!(error = %e, path = %path, "获取索引状态失败");
            Ok(api_error!("vector_db.status_failed"))
        }
    }
}

#[tauri::command]
pub async fn delete_workspace_index(
    path: String,
    state: State<'_, VectorDbState>,
) -> TauriApiResult<EmptyData> {
    let root = PathBuf::from(&path);
    let index_dir = root.join(".oxi");

    state.search_engine.invalidate_workspace_index(&root);

    if index_dir.exists() {
        let dir = index_dir.clone();
        match tokio::task::spawn_blocking(move || std::fs::remove_dir_all(dir)).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                warn!(error = %e, path = %path, "删除索引失败");
                return Ok(api_error!("vector_db.delete_failed"));
            }
            Err(e) => {
                error!("删除索引任务 join 失败: {}", e);
                return Ok(api_error!("vector_db.delete_failed"));
            }
        }
    }
    Ok(api_success!(EmptyData::default()))
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
    let config = state.search_engine.config().clone();
    let embedder = state.search_engine.embedder();
    state.search_engine.invalidate_workspace_index(&root);

    let workspace_index_manager =
        match crate::vector_db::storage::IndexManager::new(&root, config.clone()) {
            Ok(manager) => std::sync::Arc::new(manager),
            Err(e) => {
                tracing::error!("创建工作区索引管理器失败: {}", e);
                return Ok(api_error!("vector_db.index_failed"));
            }
        };

    if let Some(existing) = tasks_store().lock().remove(&path) {
        existing.abort();
    }

    let embed_config = config.clone();

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

    let path_key = path.clone();
    let index_manager = workspace_index_manager.clone();
    let handle = tokio::spawn({
        let embed_config = embed_config.clone();
        async move {
            let file_list_res = tokio::task::spawn_blocking({
                let root = root.clone();
                let cfg = embed_config.clone();
                move || crate::vector_db::utils::collect_source_files(&root, cfg.max_file_size)
            })
            .await;

            let files = match file_list_res {
                Ok(list) => list,
                Err(e) => {
                    error!("收集文件列表失败: {}", e);
                    set_progress(
                        &path_key,
                        VectorBuildProgress {
                            current_file: None,
                            files_completed: 0,
                            total_files: 0,
                            current_file_chunks: None,
                            total_chunks: 0,
                            is_complete: true,
                            error: Some("collect_failed".into()),
                        },
                    );
                    return;
                }
            };

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
                if let Err(e) = index_manager.index_file_with(&f, &*embedder).await {
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
        }
    });

    tasks_store().lock().insert(path, handle);
    Ok(api_success!(EmptyData::default()))
}
