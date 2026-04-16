use std::collections::{BTreeMap, HashMap, VecDeque};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use crate::file_watcher::ObservedFsChangeBatch;

use super::types::{ChangeKind, ObservedChange, PendingChange};

const MAX_PENDING_PER_WORKSPACE: usize = 256;
const MAX_SNAPSHOT_BYTES: u64 = 256 * 1024;
const MAX_PATCH_CHANGED_LINES: usize = 10;
const MAX_PATCH_CHARS: usize = 2000;
const MAX_SNAPSHOTS: usize = 256;
const AGENT_WRITE_SUPPRESS_MS: u64 = 5_000;

enum Command {
    BeginAgentWrite {
        workspace_key: Arc<str>,
        abs_path: PathBuf,
    },
    UpdateSnapshot {
        workspace_key: Arc<str>,
        abs_path: PathBuf,
        content: String,
    },
    TakePending {
        workspace_key: Arc<str>,
        reply: oneshot::Sender<Vec<PendingChange>>,
    },
}

struct WorkspaceState {
    workspace_root: PathBuf,
    pending: VecDeque<PendingChange>,
    agent_suppress_until_ms: HashMap<String, u64>,
    snapshots: lru::LruCache<String, String>,
}

pub struct WorkspaceChangeJournal {
    cmd_tx: mpsc::Sender<Command>,
    fs_tx: mpsc::Sender<ObservedFsChangeBatch>,
}

impl WorkspaceChangeJournal {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(1024);
        let (fs_tx, fs_rx) = mpsc::channel::<ObservedFsChangeBatch>(2048);
        tauri::async_runtime::spawn(actor_loop(cmd_rx, fs_rx));
        Self { cmd_tx, fs_tx }
    }

    pub fn fs_sender(&self) -> mpsc::Sender<ObservedFsChangeBatch> {
        self.fs_tx.clone()
    }

    pub async fn begin_agent_write(&self, workspace_key: Arc<str>, abs_path: PathBuf) {
        if let Err(err) = self
            .cmd_tx
            .send(Command::BeginAgentWrite {
                workspace_key,
                abs_path,
            })
            .await
        {
            tracing::warn!("Failed to send begin_agent_write command: {}", err);
        }
    }

    pub async fn update_snapshot_from_read(
        &self,
        workspace_key: Arc<str>,
        abs_path: PathBuf,
        content: &str,
    ) {
        if content.len() as u64 > MAX_SNAPSHOT_BYTES {
            return;
        }

        if let Err(err) = self
            .cmd_tx
            .send(Command::UpdateSnapshot {
                workspace_key,
                abs_path,
                content: content.to_string(),
            })
            .await
        {
            tracing::warn!("Failed to send workspace snapshot update command: {}", err);
        }
    }

    pub async fn take_pending_by_key(&self, workspace_key: Arc<str>) -> Vec<PendingChange> {
        let (tx, rx) = oneshot::channel();
        if let Err(err) = self
            .cmd_tx
            .send(Command::TakePending {
                workspace_key,
                reply: tx,
            })
            .await
        {
            tracing::warn!("Failed to request pending workspace changes: {}", err);
            return Vec::new();
        }

        match rx.await {
            Ok(pending) => pending,
            Err(err) => {
                tracing::warn!("Failed to receive pending workspace changes: {}", err);
                Vec::new()
            }
        }
    }
}

impl Default for WorkspaceChangeJournal {
    fn default() -> Self {
        Self::new()
    }
}

async fn actor_loop(
    mut cmd_rx: mpsc::Receiver<Command>,
    mut fs_rx: mpsc::Receiver<ObservedFsChangeBatch>,
) {
    let mut workspaces: BTreeMap<Arc<str>, WorkspaceState> = BTreeMap::new();

    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                handle_command(&mut workspaces, cmd).await;
            }
            Some(batch) = fs_rx.recv() => {
                handle_fs_batch(&mut workspaces, batch).await;
            }
            else => break,
        }
    }
}

async fn handle_command(workspaces: &mut BTreeMap<Arc<str>, WorkspaceState>, cmd: Command) {
    match cmd {
        Command::BeginAgentWrite {
            workspace_key,
            abs_path,
        } => {
            let state = workspaces
                .entry(workspace_key)
                .or_insert_with(|| WorkspaceState::new(infer_workspace_root_from_path(&abs_path)));
            let key = normalize_abs_path(&abs_path);
            state.agent_suppress_until_ms.insert(
                key,
                crate::file_watcher::now_timestamp_ms().saturating_add(AGENT_WRITE_SUPPRESS_MS),
            );
        }
        Command::UpdateSnapshot {
            workspace_key,
            abs_path,
            content,
        } => {
            let state = workspaces
                .entry(workspace_key)
                .or_insert_with(|| WorkspaceState::new(infer_workspace_root_from_path(&abs_path)));
            state.snapshots.put(normalize_abs_path(&abs_path), content);
        }
        Command::TakePending {
            workspace_key,
            reply,
        } => {
            let pending = match workspaces.get_mut(&workspace_key) {
                Some(state) => state.pending.drain(..).collect(),
                None => Vec::new(),
            };
            if reply.send(pending).is_err() {
                tracing::warn!("Failed to send pending workspace changes to requester");
            }
        }
    }
}

async fn handle_fs_batch(
    workspaces: &mut BTreeMap<Arc<str>, WorkspaceState>,
    batch: ObservedFsChangeBatch,
) {
    let state = workspaces
        .entry(batch.workspace_key)
        .or_insert_with(|| WorkspaceState::new(batch.workspace_root.clone()));
    state.workspace_root = batch.workspace_root;

    prune_agent_suppression(state);

    for fs_change in batch.changes {
        let change = ObservedChange {
            abs_path: normalize_abs_path_str(&fs_change.abs_path),
            old_abs_path: fs_change
                .old_abs_path
                .as_ref()
                .map(|v| normalize_abs_path_str(v)),
            kind: match fs_change.event_type {
                crate::file_watcher::FsEventType::Created => ChangeKind::Created,
                crate::file_watcher::FsEventType::Modified => ChangeKind::Modified,
                crate::file_watcher::FsEventType::Deleted => ChangeKind::Deleted,
                crate::file_watcher::FsEventType::Renamed => ChangeKind::Renamed,
            },
            observed_at_ms: fs_change.observed_at_ms,
        };

        record_observed_change(state, change).await;
    }
}

async fn record_observed_change(state: &mut WorkspaceState, change: ObservedChange) {
    if is_suppressed_agent_write(state, &change.abs_path) {
        return;
    }

    if should_ignore_observed_path(&change.abs_path) {
        return;
    }

    let Some(relative) = relative_path(&state.workspace_root, &change.abs_path) else {
        return;
    };

    let (patch, large_change) = match change.kind {
        ChangeKind::Modified | ChangeKind::Created | ChangeKind::Renamed => {
            match compute_patch_from_snapshot(state, &change.abs_path).await {
                Some(result) => result,
                None => {
                    tracing::warn!(
                        "Failed to compute workspace change patch for '{}'",
                        change.abs_path
                    );
                    (None, true)
                }
            }
        }
        ChangeKind::Deleted => (None, false),
    };

    state.pending.push_back(PendingChange {
        relative_path: relative,
        kind: change.kind,
        observed_at_ms: change.observed_at_ms,
        patch,
        large_change,
        note: if large_change {
            Some("Large change detected; re-read before editing.".to_string())
        } else {
            None
        },
    });

    while state.pending.len() > MAX_PENDING_PER_WORKSPACE {
        state.pending.pop_front();
    }
}

fn relative_path(workspace_root: &Path, abs_path: &str) -> Option<String> {
    let path = PathBuf::from(abs_path);
    let rel = match path.strip_prefix(workspace_root) {
        Ok(relative) => relative,
        Err(err) => {
            tracing::warn!(
                "Observed path '{}' is outside workspace '{}': {}",
                abs_path,
                workspace_root.display(),
                err
            );
            return None;
        }
    };
    let relative = rel
        .components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .trim_start_matches(std::path::MAIN_SEPARATOR)
        .replace('\\', "/");
    if relative.is_empty() {
        None
    } else {
        Some(relative)
    }
}

fn normalize_abs_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn normalize_abs_path_str(path: &str) -> String {
    path.replace('\\', "/")
}

fn should_ignore_observed_path(abs_path: &str) -> bool {
    let path = Path::new(abs_path);

    if path.components().any(|component| match component {
        std::path::Component::Normal(value) => value == "__pycache__",
        _ => false,
    }) {
        return true;
    }

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "pyc" | "pyo"))
        .unwrap_or(false)
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains(".pyc."))
}

fn infer_workspace_root_from_path(abs_path: &Path) -> PathBuf {
    if abs_path.is_dir() {
        return abs_path.to_path_buf();
    }

    match abs_path.parent() {
        Some(parent) => parent.to_path_buf(),
        None => abs_path.to_path_buf(),
    }
}

fn prune_agent_suppression(state: &mut WorkspaceState) {
    let now = crate::file_watcher::now_timestamp_ms();
    state
        .agent_suppress_until_ms
        .retain(|_, until| *until > now);
}

fn is_suppressed_agent_write(state: &mut WorkspaceState, abs_path: &str) -> bool {
    let now = crate::file_watcher::now_timestamp_ms();
    state
        .agent_suppress_until_ms
        .get(abs_path)
        .is_some_and(|until| *until > now)
}

async fn compute_patch_from_snapshot(
    state: &mut WorkspaceState,
    abs_path: &str,
) -> Option<(Option<String>, bool)> {
    let Some(old) = state.snapshots.get(abs_path).cloned() else {
        return Some((None, false));
    };
    let path = PathBuf::from(abs_path);
    let meta = match tokio::fs::metadata(&path).await {
        Ok(metadata) => metadata,
        Err(err) => {
            tracing::warn!(
                "Failed to stat workspace change path '{}': {}",
                path.display(),
                err
            );
            return None;
        }
    };
    if !meta.is_file() {
        return Some((None, false));
    }
    if meta.len() > MAX_SNAPSHOT_BYTES {
        return Some((None, true));
    }
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::warn!(
                "Failed to read workspace change path '{}': {}",
                path.display(),
                err
            );
            return None;
        }
    };
    let new = String::from_utf8_lossy(&bytes).to_string();
    if new == old {
        return Some((None, false));
    }

    let patch = diffy::create_patch(&old, &new);
    let patch_text = patch.to_string();
    let changed_lines = count_changed_lines(&patch_text);
    if changed_lines > MAX_PATCH_CHANGED_LINES {
        return Some((None, true));
    }

    let trimmed = crate::agent::common::truncate_chars(&patch_text, MAX_PATCH_CHARS);
    Some((Some(trimmed), false))
}

fn count_changed_lines(patch: &str) -> usize {
    patch
        .lines()
        .filter(|line| {
            if line.starts_with("+++") || line.starts_with("---") || line.starts_with("@@") {
                return false;
            }
            line.starts_with('+') || line.starts_with('-')
        })
        .count()
}

impl WorkspaceState {
    fn new(workspace_root: PathBuf) -> Self {
        let cap = NonZeroUsize::new(MAX_SNAPSHOTS).unwrap_or(NonZeroUsize::MIN);
        Self {
            workspace_root,
            pending: VecDeque::new(),
            agent_suppress_until_ms: HashMap::new(),
            snapshots: lru::LruCache::new(cap),
        }
    }
}
