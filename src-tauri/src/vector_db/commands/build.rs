use crate::utils::{EmptyData, TauriApiResult};
use crate::vector_db::commands::VectorDbState;
use crate::{api_error, api_success};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{ipc::Channel, State};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorBuildPhase {
    Pending,
    CollectingFiles,
    Chunking,
    Embedding,
    Writing,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VectorBuildProgress {
    pub phase: VectorBuildPhase,
    pub root: String,

    pub total_files: usize,
    pub files_done: usize,
    pub files_failed: usize,

    pub current_file: Option<String>,
    pub current_file_chunks_total: usize,
    pub current_file_chunks_done: usize,

    pub is_done: bool,
    pub error: Option<String>,
}

impl VectorBuildProgress {
    fn new(root: String) -> Self {
        Self {
            phase: VectorBuildPhase::Pending,
            root,
            total_files: 0,
            files_done: 0,
            files_failed: 0,
            current_file: None,
            current_file_chunks_total: 0,
            current_file_chunks_done: 0,
            is_done: false,
            error: None,
        }
    }
}

struct BuildState {
    progress: Mutex<VectorBuildProgress>,
    tx: broadcast::Sender<VectorBuildProgress>,
}

impl BuildState {
    fn new(root: String) -> Self {
        let (tx, _rx) = broadcast::channel::<VectorBuildProgress>(64);
        let progress = VectorBuildProgress::new(root);
        let _ = tx.send(progress.clone());
        Self {
            progress: Mutex::new(progress),
            tx,
        }
    }

    fn snapshot(&self) -> VectorBuildProgress {
        self.progress.lock().clone()
    }

    fn subscribe(&self) -> broadcast::Receiver<VectorBuildProgress> {
        self.tx.subscribe()
    }

    fn update(&self, f: impl FnOnce(&mut VectorBuildProgress)) {
        let mut p = self.progress.lock();
        f(&mut p);
        let _ = self.tx.send(p.clone());
    }
}

struct BuildEntry {
    token: CancellationToken,
    handle: JoinHandle<()>,
    state: Arc<BuildState>,
}

static BUILD_TASKS: once_cell::sync::OnceCell<Arc<Mutex<HashMap<String, BuildEntry>>>> =
    once_cell::sync::OnceCell::new();

fn build_tasks() -> &'static Arc<Mutex<HashMap<String, BuildEntry>>> {
    BUILD_TASKS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

fn send_progress(channel: &Channel<VectorBuildProgress>, p: VectorBuildProgress) -> bool {
    if let Err(e) = channel.send(p) {
        warn!("Failed to send vector build progress: {}", e);
        return false;
    }
    true
}

fn start_build_locked(
    store: &mut HashMap<String, BuildEntry>,
    path: String,
    state: Arc<crate::vector_db::SemanticSearchEngine>,
) {
    if let Some(existing) = store.remove(&path) {
        existing.token.cancel();
        existing.handle.abort();
    }

    let token = CancellationToken::new();
    let task_state = Arc::new(BuildState::new(path.clone()));

    let root = PathBuf::from(&path);
    let config = state.config().clone();
    let embedder = state.embedder();
    let token_for_task = token.clone();
    let task_state_for_task = Arc::clone(&task_state);

    let handle = tokio::spawn(async move {
        state.invalidate_workspace_index(&root);

        task_state_for_task.update(|p| {
            p.phase = VectorBuildPhase::CollectingFiles;
            p.error = None;
            p.is_done = false;
        });

        let manager = match crate::vector_db::storage::IndexManager::new(&root, config.clone()) {
            Ok(m) => Arc::new(m),
            Err(e) => {
                error!("创建工作区索引管理器失败: {}", e);
                task_state_for_task.update(|p| {
                    p.phase = VectorBuildPhase::Failed;
                    p.is_done = true;
                    p.error = Some("index_manager_create_failed".into());
                });
                return;
            }
        };

        let file_list_res = tokio::task::spawn_blocking({
            let root = root.clone();
            let max = config.max_file_size;
            move || crate::vector_db::utils::collect_source_files(&root, max)
        })
        .await;

        let files = match file_list_res {
            Ok(list) => list,
            Err(e) => {
                error!("收集文件列表失败: {}", e);
                task_state_for_task.update(|p| {
                    p.phase = VectorBuildPhase::Failed;
                    p.is_done = true;
                    p.error = Some("collect_failed".into());
                });
                return;
            }
        };

        task_state_for_task.update(|p| {
            p.total_files = files.len();
            p.files_done = 0;
            p.files_failed = 0;
            p.current_file = None;
            p.current_file_chunks_total = 0;
            p.current_file_chunks_done = 0;
            p.phase = VectorBuildPhase::Chunking;
        });

        for file_path in files {
            if token_for_task.is_cancelled() {
                task_state_for_task.update(|p| {
                    p.phase = VectorBuildPhase::Cancelled;
                    p.is_done = true;
                    p.current_file = None;
                    p.current_file_chunks_total = 0;
                    p.current_file_chunks_done = 0;
                });
                return;
            }

            task_state_for_task.update(|p| {
                p.phase = VectorBuildPhase::Chunking;
                p.current_file = Some(file_path.display().to_string());
                p.current_file_chunks_total = 0;
                p.current_file_chunks_done = 0;
                p.error = None;
            });

            let res = manager
                .index_file_with_progress(&file_path, &*embedder, |done, total| {
                    task_state_for_task.update(|p| {
                        p.phase = VectorBuildPhase::Embedding;
                        p.current_file_chunks_total = total;
                        p.current_file_chunks_done = done;
                    });
                })
                .await;

            match res {
                Ok(outcome) => {
                    task_state_for_task.update(|p| {
                        p.phase = VectorBuildPhase::Writing;
                        p.current_file_chunks_total = outcome.indexed_chunks;
                        p.current_file_chunks_done = outcome.indexed_chunks;
                        p.files_done += 1;
                    });
                }
                Err(e) => {
                    task_state_for_task.update(|p| {
                        p.files_failed += 1;
                        p.files_done += 1;
                        p.error = Some(e.to_string());
                    });
                }
            }
        }

        task_state_for_task.update(|p| {
            p.phase = if p.files_failed > 0 {
                VectorBuildPhase::Failed
            } else {
                VectorBuildPhase::Completed
            };
            p.is_done = true;
            p.current_file = None;
            p.current_file_chunks_total = 0;
            p.current_file_chunks_done = 0;
        });
    });

    store.insert(
        path.clone(),
        BuildEntry {
            token,
            handle,
            state: task_state,
        },
    );
}

#[tauri::command]
pub async fn vector_build_index_start(
    path: String,
    state: State<'_, VectorDbState>,
) -> TauriApiResult<EmptyData> {
    let mut store = build_tasks().lock();
    start_build_locked(&mut store, path, state.search_engine.clone());
    Ok(api_success!(EmptyData::default()))
}

#[tauri::command]
pub async fn vector_build_index_status(
    path: String,
) -> TauriApiResult<Option<VectorBuildProgress>> {
    let store = build_tasks().lock();
    Ok(api_success!(store.get(&path).map(|e| e.state.snapshot())))
}

#[tauri::command]
pub async fn vector_build_index_subscribe(
    path: String,
    channel: Channel<VectorBuildProgress>,
) -> TauriApiResult<EmptyData> {
    let (mut rx, initial) = {
        let store = build_tasks().lock();
        let Some(entry) = store.get(&path) else {
            return Ok(api_error!("vector_db.progress_unavailable"));
        };
        (entry.state.subscribe(), entry.state.snapshot())
    };

    if !send_progress(&channel, initial) {
        return Ok(api_success!(EmptyData::default()));
    }

    loop {
        match rx.recv().await {
            Ok(p) => {
                if !send_progress(&channel, p.clone()) {
                    break;
                }
                if p.is_done {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Closed) => break,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
        }
    }

    Ok(api_success!(EmptyData::default()))
}

#[tauri::command]
pub async fn vector_build_index_cancel(path: String) -> TauriApiResult<EmptyData> {
    let mut store = build_tasks().lock();
    if let Some(entry) = store.get_mut(&path) {
        entry.token.cancel();
        entry.handle.abort();
        entry.state.update(|p| {
            p.phase = VectorBuildPhase::Cancelled;
            p.is_done = true;
            p.current_file = None;
            p.current_file_chunks_total = 0;
            p.current_file_chunks_done = 0;
        });
    }
    Ok(api_success!(EmptyData::default()))
}
