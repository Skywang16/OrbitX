use notify::{
    event::{CreateKind, ModifyKind, RemoveKind, RenameMode},
    Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Runtime};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info};

use super::config::FileWatcherConfig;
use super::events::{
    now_timestamp_ms, FileWatcherEvent, FileWatcherEventBatch, FsEventType, GitChangeType,
};

const CHANNEL_CAPACITY: usize = 2048;

#[derive(Debug, Clone)]
struct GitPaths {
    git_dir: PathBuf,
    common_dir: PathBuf,
}

#[derive(Debug, Clone)]
struct CompiledIgnore {
    gitignore: ignore::gitignore::Gitignore,
}

impl CompiledIgnore {
    fn new(workspace_root: &Path, patterns: &[String]) -> Self {
        let mut builder = ignore::gitignore::GitignoreBuilder::new(workspace_root);
        for pattern in patterns {
            let _ = builder.add_line(None, pattern);
        }

        let gitignore = builder.build().unwrap_or_else(|_| {
            ignore::gitignore::GitignoreBuilder::new(workspace_root)
                .build()
                .expect("GitignoreBuilder must build for empty set")
        });

        Self { gitignore }
    }

    fn is_ignored_abs(&self, abs_path: &Path) -> bool {
        let file_match = self
            .gitignore
            .matched_path_or_any_parents(abs_path, false)
            .is_ignore();
        if file_match {
            return true;
        }
        self.gitignore
            .matched_path_or_any_parents(abs_path, true)
            .is_ignore()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatcherStatus {
    pub running: bool,
    pub workspace_root: Option<String>,
    pub repo_root: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ObservedFsChange {
    pub abs_path: String,
    pub old_abs_path: Option<String>,
    pub event_type: FsEventType,
    pub observed_at_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ObservedFsChangeBatch {
    pub workspace_key: Arc<str>,
    pub workspace_root: PathBuf,
    pub changes: Vec<ObservedFsChange>,
}

struct WatcherState {
    watcher: RecommendedWatcher,
    watched_paths: Vec<PathBuf>,
    workspace_root: PathBuf,
    repo_root: Option<PathBuf>,
    shutdown: Arc<AtomicBool>,
}

pub struct UnifiedFileWatcher {
    state: Arc<RwLock<Option<WatcherState>>>,
    fs_sink: Option<mpsc::Sender<ObservedFsChangeBatch>>,
}

impl UnifiedFileWatcher {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(None)),
            fs_sink: None,
        }
    }

    pub fn with_fs_sink(mut self, sink: mpsc::Sender<ObservedFsChangeBatch>) -> Self {
        self.fs_sink = Some(sink);
        self
    }

    pub async fn start<R: Runtime + 'static>(
        &self,
        emitter: tauri::AppHandle<R>,
        root: String,
        config: FileWatcherConfig,
    ) -> Result<WatcherStatus, String> {
        let requested_root = PathBuf::from(&root);
        if !requested_root.exists() {
            return Err("Path does not exist".to_string());
        }

        self.stop().await;

        let workspace_root = tokio::fs::canonicalize(&requested_root)
            .await
            .map_err(|e| format!("Failed to canonicalize path: {}", e))?;

        let repo_root = find_git_root(&workspace_root).await;
        let git_paths = match &repo_root {
            Some(repo) if config.enable_git_watcher => Some(resolve_git_paths(repo).await?),
            _ => None,
        };

        let ignore = CompiledIgnore::new(&workspace_root, &config.ignore_patterns);
        let (tx, mut rx) = mpsc::channel::<notify::Event>(CHANNEL_CAPACITY);
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown);

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.try_send(event);
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(500)),
        )
        .map_err(|e| e.to_string())?;

        let mut watched_paths = Vec::new();

        if config.enable_fs_watcher {
            watcher
                .watch(&workspace_root, RecursiveMode::Recursive)
                .map_err(|e| e.to_string())?;
            watched_paths.push(workspace_root.clone());
        }

        if let Some(git) = &git_paths {
            watcher
                .watch(&git.git_dir, RecursiveMode::NonRecursive)
                .map_err(|e| e.to_string())?;
            watched_paths.push(git.git_dir.clone());

            if git.common_dir != git.git_dir {
                watcher
                    .watch(&git.common_dir, RecursiveMode::NonRecursive)
                    .map_err(|e| e.to_string())?;
                watched_paths.push(git.common_dir.clone());
            }

            let refs_dir = git.common_dir.join("refs");
            if refs_dir.exists() {
                watcher
                    .watch(&refs_dir, RecursiveMode::Recursive)
                    .map_err(|e| e.to_string())?;
                watched_paths.push(refs_dir);
            }
        }

        let state = WatcherState {
            watcher,
            watched_paths,
            workspace_root: workspace_root.clone(),
            repo_root: repo_root.clone(),
            shutdown: Arc::clone(&shutdown),
        };

        let status = WatcherStatus {
            running: true,
            workspace_root: Some(workspace_root.to_string_lossy().to_string()),
            repo_root: repo_root.as_ref().map(|p| p.to_string_lossy().to_string()),
        };
        *self.state.write().await = Some(state);

        let workspace_root_for_task = workspace_root.clone();
        let workspace_key: Arc<str> = Arc::from(workspace_root.to_string_lossy().to_string());
        let workspace_root_str = workspace_root.to_string_lossy().to_string();
        let fs_sink = self.fs_sink.clone();
        let repo_root_str = repo_root.as_ref().map(|p| p.to_string_lossy().to_string());
        let git_paths_for_task = git_paths;
        let ignore_for_task = ignore;

        let debounce_ms = config.debounce_ms;
        let throttle_ms = config.throttle_ms;

        tokio::spawn(async move {
            let mut seq: u64 = 0;
            let mut pending_git: HashSet<GitChangeType> = HashSet::new();
            let mut pending_fs: HashMap<String, (FsEventType, Option<String>)> = HashMap::new();
            let mut last_emit_time: Option<tokio::time::Instant> = None;
            let debounce = tokio::time::sleep(Duration::from_secs(3600));
            tokio::pin!(debounce);

            loop {
                if shutdown_clone.load(Ordering::SeqCst) {
                    break;
                }

                tokio::select! {
                    Some(event) = rx.recv() => {
                        if let Some(git_paths) = &git_paths_for_task {
                            if let Some(change_type) = classify_git_event(
                                &event,
                                git_paths,
                                &workspace_root_for_task,
                                &ignore_for_task,
                            ) {
                                pending_git.insert(change_type);
                            }
                        }

                        if let Some(fs_change) = classify_fs_event(&event) {
                            for (path, event_type, old_path) in fs_change {
                                let abs = PathBuf::from(&path);
                                if ignore_for_task.is_ignored_abs(&abs) {
                                    continue;
                                }
                                pending_fs.insert(path, (event_type, old_path));
                            }
                        }

                        debounce.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(debounce_ms));
                    }
                    _ = &mut debounce => {
                        if pending_git.is_empty() && pending_fs.is_empty() {
                            debounce.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(3600));
                            continue;
                        }

                        if let Some(last) = last_emit_time {
                            let elapsed = last.elapsed();
                            if elapsed < Duration::from_millis(throttle_ms) {
                                tokio::time::sleep(Duration::from_millis(throttle_ms) - elapsed).await;
                            }
                        }

                        let now_ms = now_timestamp_ms();
                        let mut events: Vec<FileWatcherEvent> = Vec::new();
                        if let Some(repo_root) = &repo_root_str {
                            if !pending_git.is_empty() {
                                events.push(FileWatcherEvent::git_changed(
                                    repo_root.clone(),
                                    summarize_git_change(&pending_git),
                                ));
                            }
                        }

                        let mut observed: Vec<ObservedFsChange> = Vec::new();
                        for (path, (event_type, old_path)) in pending_fs.drain() {
                            observed.push(ObservedFsChange {
                                abs_path: path.clone(),
                                old_abs_path: old_path.clone(),
                                event_type,
                                observed_at_ms: now_ms,
                            });

                            events.push(FileWatcherEvent::fs_changed(
                                workspace_root_str.clone(),
                                path,
                                event_type,
                                old_path,
                            ));
                        }

                        pending_git.clear();

                        if let Some(sink) = &fs_sink {
                            if !observed.is_empty() {
                                let _ = sink.try_send(ObservedFsChangeBatch {
                                    workspace_key: Arc::clone(&workspace_key),
                                    workspace_root: workspace_root_for_task.clone(),
                                    changes: observed,
                                });
                            }
                        }

                        if !events.is_empty() {
                            seq = seq.saturating_add(1);
                            let batch = FileWatcherEventBatch { seq, events };
                            let _ = emitter.emit("file-watcher:event", &batch);
                            last_emit_time = Some(tokio::time::Instant::now());
                        }

                        debounce.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(3600));
                    }
                }
            }

            debug!("Unified file watcher stopped");
        });

        info!("Unified file watcher started for {:?}", workspace_root);
        Ok(status)
    }

    pub async fn stop(&self) {
        if let Some(mut state) = self.state.write().await.take() {
            state.shutdown.store(true, Ordering::SeqCst);
            for path in &state.watched_paths {
                let _ = state.watcher.unwatch(path);
            }
        }
    }

    pub async fn status(&self) -> WatcherStatus {
        self.state
            .read()
            .await
            .as_ref()
            .map(|s| WatcherStatus {
                running: true,
                workspace_root: Some(s.workspace_root.to_string_lossy().to_string()),
                repo_root: s
                    .repo_root
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string()),
            })
            .unwrap_or(WatcherStatus {
                running: false,
                workspace_root: None,
                repo_root: None,
            })
    }
}

impl Default for UnifiedFileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

// No mapping here: watcher is an observer only.

fn summarize_git_change(changes: &HashSet<GitChangeType>) -> GitChangeType {
    if changes.contains(&GitChangeType::Head) {
        return GitChangeType::Head;
    }
    if changes.contains(&GitChangeType::Refs) {
        return GitChangeType::Refs;
    }
    if changes.contains(&GitChangeType::Index) {
        return GitChangeType::Index;
    }
    GitChangeType::Worktree
}

fn classify_git_event(
    event: &notify::Event,
    git_paths: &GitPaths,
    workspace_root: &Path,
    ignore: &CompiledIgnore,
) -> Option<GitChangeType> {
    match &event.kind {
        EventKind::Create(CreateKind::File)
        | EventKind::Create(CreateKind::Any)
        | EventKind::Modify(ModifyKind::Data(_))
        | EventKind::Modify(ModifyKind::Any)
        | EventKind::Remove(RemoveKind::File)
        | EventKind::Remove(RemoveKind::Any)
        | EventKind::Modify(ModifyKind::Name(RenameMode::Any))
        | EventKind::Modify(ModifyKind::Name(RenameMode::From))
        | EventKind::Modify(ModifyKind::Name(RenameMode::To))
        | EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {}
        _ => return None,
    }

    for path in &event.paths {
        if path.starts_with(&git_paths.git_dir) {
            if let Ok(relative) = path.strip_prefix(&git_paths.git_dir) {
                let rel = relative.to_string_lossy();
                if rel == "index" {
                    return Some(GitChangeType::Index);
                }
                if rel == "HEAD" {
                    return Some(GitChangeType::Head);
                }
            }
            continue;
        }

        if path.starts_with(&git_paths.common_dir) {
            if let Ok(relative) = path.strip_prefix(&git_paths.common_dir) {
                let rel = relative.to_string_lossy();
                if rel.starts_with("refs/") || rel == "packed-refs" {
                    return Some(GitChangeType::Refs);
                }
                if rel == "COMMIT_EDITMSG"
                    || rel == "MERGE_HEAD"
                    || rel == "REBASE_HEAD"
                    || rel == "CHERRY_PICK_HEAD"
                {
                    return Some(GitChangeType::Index);
                }
            }
            continue;
        }

        // Worktree change: only signal if not ignored (prevents node_modules spam etc.)
        if path.starts_with(workspace_root) && ignore.is_ignored_abs(path) {
            continue;
        }
        return Some(GitChangeType::Worktree);
    }

    None
}

fn classify_fs_event(event: &notify::Event) -> Option<Vec<(String, FsEventType, Option<String>)>> {
    let event_type = match &event.kind {
        EventKind::Create(CreateKind::File) | EventKind::Create(CreateKind::Any) => {
            FsEventType::Created
        }
        EventKind::Modify(ModifyKind::Data(_)) | EventKind::Modify(ModifyKind::Any) => {
            FsEventType::Modified
        }
        EventKind::Remove(RemoveKind::File) | EventKind::Remove(RemoveKind::Any) => {
            FsEventType::Deleted
        }
        EventKind::Modify(ModifyKind::Name(_)) => FsEventType::Renamed,
        _ => return None,
    };

    if event.paths.is_empty() {
        return None;
    }

    if event_type == FsEventType::Renamed {
        if event.paths.len() >= 2 {
            let from = event.paths[0].to_string_lossy().to_string();
            let to = event.paths[1].to_string_lossy().to_string();
            return Some(vec![(to, FsEventType::Renamed, Some(from))]);
        }
        return Some(vec![(
            event.paths[0].to_string_lossy().to_string(),
            FsEventType::Renamed,
            None,
        )]);
    }

    Some(
        event
            .paths
            .iter()
            .map(|p| (p.to_string_lossy().to_string(), event_type, None))
            .collect(),
    )
}

async fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start.to_path_buf());
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }
    None
}

async fn resolve_git_paths(worktree: &Path) -> Result<GitPaths, String> {
    let dot_git = worktree.join(".git");

    let git_dir = if dot_git.is_dir() {
        dot_git
    } else if dot_git.is_file() {
        let content = tokio::fs::read_to_string(&dot_git)
            .await
            .map_err(|e| format!("Failed to read .git file: {}", e))?;
        let content = content.trim();
        let gitdir = content
            .strip_prefix("gitdir:")
            .map(|v| v.trim())
            .ok_or_else(|| "Invalid .git file format".to_string())?;

        let gitdir_path = PathBuf::from(gitdir);
        if gitdir_path.is_absolute() {
            gitdir_path
        } else {
            worktree.join(gitdir_path)
        }
    } else {
        return Err("Not a git repository".to_string());
    };

    if !git_dir.exists() {
        return Err("Resolved git dir does not exist".to_string());
    }

    let common_dir = {
        let commondir = git_dir.join("commondir");
        if commondir.is_file() {
            let content = tokio::fs::read_to_string(&commondir)
                .await
                .map_err(|e| format!("Failed to read commondir: {}", e))?;
            let value = content.trim();
            let common_path = PathBuf::from(value);
            if common_path.is_absolute() {
                common_path
            } else {
                git_dir.join(common_path)
            }
        } else {
            git_dir.clone()
        }
    };

    Ok(GitPaths {
        git_dir,
        common_dir,
    })
}
