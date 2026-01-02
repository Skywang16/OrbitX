use notify::{
    event::{CreateKind, ModifyKind, RemoveKind, RenameMode},
    Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Runtime};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

// VSCode-style: treat bursts as "one logical change".
const DEBOUNCE_MS: u64 = 300;
const CHANNEL_CAPACITY: usize = 1024;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitChangeEvent {
    pub path: String,
    pub change_type: GitChangeType,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[derive(Hash, Eq, PartialEq)]
pub enum GitChangeType {
    Index,
    Head,
    Refs,
    Worktree,
}

#[derive(Debug, Clone)]
struct GitPaths {
    git_dir: PathBuf,
    common_dir: PathBuf,
}

struct WatcherState {
    watcher: RecommendedWatcher,
    watched_paths: Vec<PathBuf>,
    watched_path: PathBuf,
}

pub struct GitWatcher {
    state: Arc<RwLock<Option<WatcherState>>>,
    shutdown: Arc<AtomicBool>,
}

impl GitWatcher {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(None)),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start<R: Runtime, E: Emitter<R> + Clone + Send + Sync + 'static>(
        &self,
        emitter: E,
        repo_path: String,
    ) -> Result<(), String> {
        let repo_path = PathBuf::from(&repo_path);
        if !repo_path.exists() {
            return Err("Path does not exist".to_string());
        }

        let git_paths = resolve_git_paths(&repo_path).await?;

        // Stop existing watcher if any
        self.stop().await;

        self.shutdown.store(false, Ordering::SeqCst);

        let (tx, mut rx) = mpsc::channel::<notify::Event>(CHANNEL_CAPACITY);
        let shutdown_clone = self.shutdown.clone();

        // Create watcher with channel
        let watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    // We only need "at least one" signal per burst; dropping extra events is fine.
                    let _ = tx.try_send(event);
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(500)),
        )
        .map_err(|e| e.to_string())?;

        let mut state = WatcherState {
            watcher,
            watched_paths: Vec::new(),
            watched_path: repo_path.clone(),
        };

        // Watch git dir (index/HEAD etc) - non-recursive keeps noise low.
        state
            .watcher
            .watch(&git_paths.git_dir, RecursiveMode::NonRecursive)
            .map_err(|e| e.to_string())?;
        state.watched_paths.push(git_paths.git_dir.clone());

        // Watch common git dir (packed-refs and refs dir discovery).
        if git_paths.common_dir != git_paths.git_dir {
            state
                .watcher
                .watch(&git_paths.common_dir, RecursiveMode::NonRecursive)
                .map_err(|e| e.to_string())?;
            state.watched_paths.push(git_paths.common_dir.clone());
        }

        // Watch refs recursively if present (branch/tag updates).
        let refs_dir = git_paths.common_dir.join("refs");
        if refs_dir.exists() {
            state
                .watcher
                .watch(&refs_dir, RecursiveMode::Recursive)
                .map_err(|e| e.to_string())?;
            state.watched_paths.push(refs_dir);
        }

        // Watch working directory (exclude .git via classifier).
        state
            .watcher
            .watch(&repo_path, RecursiveMode::Recursive)
            .map_err(|e| e.to_string())?;
        state.watched_paths.push(repo_path.clone());

        *self.state.write().await = Some(state);

        let repo_path_str = repo_path.to_string_lossy().to_string();
        let git_paths_for_task = git_paths.clone();

        // Spawn debounced event processor
        tokio::spawn(async move {
            let mut pending_changes: HashSet<GitChangeType> = HashSet::new();
            let debounce = tokio::time::sleep(Duration::from_secs(3600));
            tokio::pin!(debounce);

            loop {
                if shutdown_clone.load(Ordering::SeqCst) {
                    break;
                }

                tokio::select! {
                    Some(event) = rx.recv() => {
                        if let Some(change_type) = classify_event(&event, &git_paths_for_task) {
                            pending_changes.insert(change_type);
                            debounce.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(DEBOUNCE_MS));
                        }
                    }
                    _ = &mut debounce, if !pending_changes.is_empty() => {
                        let change_type = summarize_change(&pending_changes);
                        pending_changes.clear();

                        let event = GitChangeEvent {
                            path: repo_path_str.clone(),
                            change_type,
                        };
                        debug!("Emitting git change event: {:?}", event);
                        if let Err(e) = emitter.emit("git:changed", &event) {
                            error!("Failed to emit git change event: {}", e);
                        }
                    }
                }
            }

            info!("Git watcher stopped for {}", repo_path_str);
        });

        info!("Git watcher started for {:?}", repo_path);
        Ok(())
    }

    pub async fn stop(&self) {
        self.shutdown.store(true, Ordering::SeqCst);

        let mut state = self.state.write().await;
        if let Some(mut s) = state.take() {
            for watched in s.watched_paths {
                let _ = s.watcher.unwatch(&watched);
            }
            info!("Git watcher stopped");
        }
    }

    pub async fn is_watching(&self) -> bool {
        self.state.read().await.is_some()
    }

    pub async fn watched_path(&self) -> Option<String> {
        self.state
            .read()
            .await
            .as_ref()
            .map(|s| s.watched_path.to_string_lossy().to_string())
    }
}

impl Default for GitWatcher {
    fn default() -> Self {
        Self::new()
    }
}

fn summarize_change(changes: &HashSet<GitChangeType>) -> GitChangeType {
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

fn classify_event(event: &notify::Event, git_paths: &GitPaths) -> Option<GitChangeType> {
    // Only care about create, modify, remove, rename events
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
            let relative = path.strip_prefix(&git_paths.git_dir).ok()?;
            let relative_str = relative.to_string_lossy();

            // .git/index - staging area changes
            if relative_str == "index" || relative_str == "index.lock" {
                return Some(GitChangeType::Index);
            }

            // .git/HEAD - branch switch
            if relative_str == "HEAD" || relative_str == "HEAD.lock" {
                return Some(GitChangeType::Head);
            }

            // Skip other git_dir files (logs, objects, etc.)
            continue;
        }

        if path.starts_with(&git_paths.common_dir) {
            let relative = path.strip_prefix(&git_paths.common_dir).ok()?;
            let relative_str = relative.to_string_lossy();

            // .git/refs/* - branch/tag changes
            if relative_str.starts_with("refs/") {
                return Some(GitChangeType::Refs);
            }

            // packed refs is used by many operations (fetch, tag, branch updates)
            if relative_str == "packed-refs" || relative_str == "packed-refs.lock" {
                return Some(GitChangeType::Refs);
            }

            // .git/COMMIT_EDITMSG, .git/MERGE_HEAD etc - commit related
            if relative_str == "COMMIT_EDITMSG"
                || relative_str == "MERGE_HEAD"
                || relative_str == "REBASE_HEAD"
                || relative_str == "CHERRY_PICK_HEAD"
            {
                return Some(GitChangeType::Index);
            }

            // Skip other common dir files
            continue;
        }

        // Working tree change
        let path_str = path.to_string_lossy();

        // Skip common non-essential paths
        if path_str.contains("node_modules")
            || path_str.contains(".git")
            || path_str.contains("target/")
            || path_str.contains("dist/")
            || path_str.contains(".DS_Store")
            || path_str.ends_with(".log")
        {
            continue;
        }

        return Some(GitChangeType::Worktree);
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
