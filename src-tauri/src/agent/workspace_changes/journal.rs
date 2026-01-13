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
        let _ = self
            .cmd_tx
            .send(Command::BeginAgentWrite {
                workspace_key,
                abs_path,
            })
            .await;
    }

    pub async fn update_snapshot_from_read(
        &self,
        workspace_key: Arc<str>,
        abs_path: PathBuf,
        content: &str,
    ) {
        if content.as_bytes().len() as u64 > MAX_SNAPSHOT_BYTES {
            return;
        }

        let _ = self
            .cmd_tx
            .send(Command::UpdateSnapshot {
                workspace_key,
                abs_path,
                content: content.to_string(),
            })
            .await;
    }

    pub async fn take_pending_by_key(&self, workspace_key: Arc<str>) -> Vec<PendingChange> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .cmd_tx
            .send(Command::TakePending {
                workspace_key,
                reply: tx,
            })
            .await;
        rx.await.unwrap_or_default()
    }

    pub async fn take_pending(&self, workspace_root: &Path) -> Vec<PendingChange> {
        let workspace_key: Arc<str> = Arc::from(
            std::fs::canonicalize(workspace_root)
                .unwrap_or_else(|_| workspace_root.to_path_buf())
                .to_string_lossy()
                .to_string(),
        );
        self.take_pending_by_key(workspace_key).await
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
            let state = workspaces.entry(workspace_key).or_insert_with(|| {
                WorkspaceState::new(PathBuf::from(".")) // placeholder; fs batch will override
            });
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
            let state = workspaces.entry(workspace_key).or_insert_with(|| {
                WorkspaceState::new(PathBuf::from(".")) // placeholder; fs batch will override
            });
            state.snapshots.put(normalize_abs_path(&abs_path), content);
        }
        Command::TakePending {
            workspace_key,
            reply,
        } => {
            let pending = workspaces
                .get_mut(&workspace_key)
                .map(|s| s.pending.drain(..).collect())
                .unwrap_or_else(Vec::new);
            let _ = reply.send(pending);
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

    let Some(relative) = relative_path(&state.workspace_root, &change.abs_path) else {
        return;
    };

    let (patch, large_change) = match change.kind {
        ChangeKind::Modified | ChangeKind::Created | ChangeKind::Renamed => {
            compute_patch_from_snapshot(state, &change.abs_path)
                .await
                .unwrap_or((None, true))
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
    let rel = path.strip_prefix(workspace_root).ok()?;
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
    let old = state.snapshots.get(abs_path).cloned()?;
    let path = PathBuf::from(abs_path);
    let meta = tokio::fs::metadata(&path).await.ok()?;
    if !meta.is_file() {
        return Some((None, false));
    }
    if meta.len() > MAX_SNAPSHOT_BYTES {
        return Some((None, true));
    }
    let bytes = tokio::fs::read(&path).await.ok()?;
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

    let mut trimmed = patch_text;
    if trimmed.len() > MAX_PATCH_CHARS {
        trimmed.truncate(MAX_PATCH_CHARS.saturating_sub(3));
        trimmed.push_str("...");
    }
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
        let cap = NonZeroUsize::new(MAX_SNAPSHOTS).expect("MAX_SNAPSHOTS must be non-zero");
        Self {
            workspace_root,
            pending: VecDeque::new(),
            agent_suppress_until_ms: HashMap::new(),
            snapshots: lru::LruCache::new(cap),
        }
    }
}
