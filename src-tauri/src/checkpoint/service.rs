//! Checkpoint 服务层（重构版）

use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use diffy::{create_patch, PatchFormatter};
use tokio::fs;

use super::blob_store::BlobStore;
use super::config::CheckpointConfig;
use super::models::{
    Checkpoint, CheckpointError, CheckpointResult, CheckpointSummary, FileChangeType, FileDiff,
    NewCheckpoint, NewFileSnapshot, RollbackResult,
};
use super::storage::CheckpointStorage;

/// Checkpoint 服务
pub struct CheckpointService {
    storage: Arc<CheckpointStorage>,
    blob_store: Arc<BlobStore>,
    config: CheckpointConfig,
}

impl CheckpointService {
    pub fn new(storage: Arc<CheckpointStorage>, blob_store: Arc<BlobStore>) -> Self {
        Self {
            storage,
            blob_store,
            config: CheckpointConfig::default(),
        }
    }

    pub fn with_config(
        storage: Arc<CheckpointStorage>,
        blob_store: Arc<BlobStore>,
        config: CheckpointConfig,
    ) -> Self {
        Self {
            storage,
            blob_store,
            config,
        }
    }

    /// 创建空 checkpoint，实际文件快照在修改发生前捕获
    pub async fn create_empty(
        &self,
        session_id: i64,
        message_id: i64,
        workspace_path: &Path,
    ) -> CheckpointResult<Checkpoint> {
        let workspace_root = canonicalize_workspace(workspace_path).await?;
        let workspace_key = workspace_root.to_string_lossy().to_string();

        let parent = self
            .storage
            .find_latest_by_session(session_id, &workspace_key)
            .await?;
        let parent_id = parent.as_ref().map(|cp| cp.id);

        let checkpoint_id = self
            .storage
            .insert(&NewCheckpoint {
                workspace_path: workspace_key.clone(),
                session_id,
                message_id,
                parent_id,
            })
            .await?;

        tracing::info!(
            "Created empty checkpoint {} for session {} message {}",
            checkpoint_id,
            session_id,
            message_id
        );

        self.storage
            .find_by_id(checkpoint_id)
            .await?
            .ok_or(CheckpointError::NotFound(checkpoint_id))
    }

    /// 在文件被修改前记录原始内容
    pub async fn snapshot_file_before_edit(
        &self,
        checkpoint_id: i64,
        file_path: &Path,
        workspace_root: &Path,
    ) -> CheckpointResult<()> {
        let resolved = resolve_file_path(file_path, workspace_root).await?;

        // 检查是否应该忽略这个文件
        if self.config.should_ignore_file(&resolved.relative) {
            tracing::debug!("Ignoring file: {}", resolved.relative);
            return Ok(());
        }

        if self
            .storage
            .has_file_snapshot(checkpoint_id, &resolved.relative)
            .await?
        {
            return Ok(()); // 幂等：同一 checkpoint 内只记录一次
        }

        match fs::read(&resolved.absolute).await {
            Ok(content) => {
                // 检查文件大小限制
                if self.config.is_file_too_large(content.len() as u64) {
                    tracing::warn!(
                        "Skipping large file: {} ({} bytes)",
                        resolved.relative,
                        content.len()
                    );
                    return Ok(());
                }

                let blob_hash = self.blob_store.store(&content).await?;
                let snapshot = NewFileSnapshot {
                    checkpoint_id,
                    file_path: resolved.relative,
                    blob_hash,
                    change_type: FileChangeType::Modified,
                    file_size: content.len() as i64,
                };
                self.storage.insert_file_snapshot(&snapshot).await?;
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {
                // 文件不存在，稍后会被新建
                let snapshot = NewFileSnapshot {
                    checkpoint_id,
                    file_path: resolved.relative,
                    blob_hash: String::new(),
                    change_type: FileChangeType::Added,
                    file_size: 0,
                };
                self.storage.insert_file_snapshot(&snapshot).await?;
            }
            Err(e) => return Err(CheckpointError::Io(e)),
        }

        Ok(())
    }

    /// 获取 checkpoint
    pub async fn get(&self, id: i64) -> CheckpointResult<Option<Checkpoint>> {
        self.storage.find_by_id(id).await
    }

    /// 通过 message_id 查找 checkpoint
    pub async fn find_by_message_id(
        &self,
        message_id: i64,
    ) -> CheckpointResult<Option<Checkpoint>> {
        self.storage.find_by_message_id(message_id).await
    }

    /// 获取会话的 checkpoint 列表
    pub async fn list_by_session(
        &self,
        session_id: i64,
        workspace_path: &str,
    ) -> CheckpointResult<Vec<CheckpointSummary>> {
        self.storage
            .list_summaries_by_session(session_id, workspace_path)
            .await
    }

    /// 回滚到指定 checkpoint：按从最新到目标的顺序依次恢复
    pub async fn rollback(&self, checkpoint_id: i64) -> CheckpointResult<RollbackResult> {
        let target = self
            .storage
            .find_by_id(checkpoint_id)
            .await?
            .ok_or(CheckpointError::NotFound(checkpoint_id))?;

        let workspace_root = canonicalize_workspace(Path::new(&target.workspace_path)).await?;
        let checkpoints = self.collect_descendants(&target).await?;

        let mut restored = HashSet::new();
        let mut failed = Vec::new();

        for checkpoint in checkpoints {
            let snapshots = self.storage.find_file_snapshots(checkpoint.id).await?;
            for snapshot in snapshots {
                let relative_path = snapshot.file_path.clone();
                let abs_path = workspace_root.join(&relative_path);

                match snapshot.change_type {
                    FileChangeType::Added => match fs::remove_file(&abs_path).await {
                        Ok(_) => {
                            restored.insert(relative_path.clone());
                        }
                        Err(e) if e.kind() == ErrorKind::NotFound => {
                            restored.insert(relative_path.clone());
                        }
                        Err(e) => {
                            failed.push((relative_path.clone(), e.to_string()));
                        }
                    },
                    FileChangeType::Modified | FileChangeType::Deleted => {
                        if snapshot.blob_hash.is_empty() {
                            failed.push((
                                relative_path.clone(),
                                "Missing blob hash for snapshot".to_string(),
                            ));
                            continue;
                        }

                        let content = match self.blob_store.get(&snapshot.blob_hash).await? {
                            Some(c) => c,
                            None => {
                                failed.push((
                                    relative_path.clone(),
                                    format!("Blob not found: {}", snapshot.blob_hash),
                                ));
                                continue;
                            }
                        };

                        if let Some(parent) = abs_path.parent() {
                            if let Err(e) = fs::create_dir_all(parent).await {
                                failed.push((relative_path.clone(), e.to_string()));
                                continue;
                            }
                        }

                        match fs::write(&abs_path, &content).await {
                            Ok(_) => {
                                restored.insert(relative_path.clone());
                            }
                            Err(e) => {
                                failed.push((relative_path.clone(), e.to_string()));
                            }
                        }
                    }
                }
            }
        }

        tracing::info!(
            "Rollback checkpoint {} restored={} failed={}",
            checkpoint_id,
            restored.len(),
            failed.len()
        );

        Ok(RollbackResult {
            checkpoint_id,
            restored_files: restored.into_iter().collect(),
            failed_files: failed,
        })
    }

    /// 计算两个 checkpoint 之间的 diff
    ///
    /// 新设计只跟踪某个 checkpoint 捕获的原始内容，因此这里退化为返回 `from_id`（或 `to_id`）记录的文件列表。
    pub async fn diff(
        &self,
        from_id: Option<i64>,
        to_id: i64,
        workspace_path: &Path,
    ) -> CheckpointResult<Vec<FileDiff>> {
        let checkpoint_id = from_id.unwrap_or(to_id);
        self.diff_from_snapshots(checkpoint_id, workspace_path)
            .await
    }

    /// 计算 checkpoint 与当前工作区的 diff
    pub async fn diff_with_workspace(
        &self,
        checkpoint_id: i64,
        workspace_path: &Path,
    ) -> CheckpointResult<Vec<FileDiff>> {
        self.diff_from_snapshots(checkpoint_id, workspace_path)
            .await
    }

    /// 获取文件内容
    pub async fn get_file_content(
        &self,
        checkpoint_id: i64,
        file_path: &str,
    ) -> CheckpointResult<Option<Vec<u8>>> {
        let snapshot = self
            .storage
            .find_file_snapshot(checkpoint_id, file_path)
            .await?;
        match snapshot {
            Some(s) if !s.blob_hash.is_empty() => self.blob_store.get(&s.blob_hash).await,
            _ => Ok(None),
        }
    }

    /// 删除 checkpoint
    pub async fn delete(&self, checkpoint_id: i64) -> CheckpointResult<()> {
        let snapshots = self.storage.find_file_snapshots(checkpoint_id).await?;

        self.storage.delete(checkpoint_id).await?;

        for snapshot in snapshots {
            if snapshot.change_type != FileChangeType::Added && !snapshot.blob_hash.is_empty() {
                self.blob_store.decrement_ref(&snapshot.blob_hash).await?;
            }
        }

        self.blob_store.gc().await?;
        Ok(())
    }

    async fn collect_descendants(&self, target: &Checkpoint) -> CheckpointResult<Vec<Checkpoint>> {
        let mut chain = Vec::new();
        let mut current = match self
            .storage
            .find_latest_by_session(target.session_id, &target.workspace_path)
            .await?
        {
            Some(cp) => cp,
            None => return Err(CheckpointError::NotFound(target.id)),
        };

        loop {
            chain.push(current.clone());
            if current.id == target.id {
                break;
            }

            let parent_id = current
                .parent_id
                .ok_or_else(|| CheckpointError::NotFound(target.id))?;
            current = self
                .storage
                .find_by_id(parent_id)
                .await?
                .ok_or(CheckpointError::NotFound(parent_id))?;
        }

        Ok(chain)
    }

    async fn diff_from_snapshots(
        &self,
        checkpoint_id: i64,
        workspace_path: &Path,
    ) -> CheckpointResult<Vec<FileDiff>> {
        let workspace_root = canonicalize_workspace(workspace_path).await?;
        let snapshots = self.storage.find_file_snapshots(checkpoint_id).await?;
        let mut diffs = Vec::new();

        for snapshot in snapshots {
            let relative_path = snapshot.file_path.clone();
            let abs_path = workspace_root.join(&relative_path);

            match snapshot.change_type {
                FileChangeType::Added => {
                    if fs::metadata(&abs_path).await.is_ok() {
                        diffs.push(FileDiff {
                            file_path: relative_path,
                            change_type: FileChangeType::Added,
                            diff_content: None,
                        });
                    }
                }
                FileChangeType::Modified => {
                    let current = fs::read(&abs_path).await.unwrap_or_default();
                    let original = self
                        .blob_store
                        .get(&snapshot.blob_hash)
                        .await?
                        .unwrap_or_default();

                    if current != original {
                        diffs.push(FileDiff {
                            file_path: relative_path,
                            change_type: FileChangeType::Modified,
                            diff_content: Some(compute_diff(&original, &current)),
                        });
                    }
                }
                FileChangeType::Deleted => {
                    if fs::metadata(&abs_path).await.is_err() {
                        diffs.push(FileDiff {
                            file_path: relative_path,
                            change_type: FileChangeType::Deleted,
                            diff_content: None,
                        });
                    }
                }
            }
        }

        Ok(diffs)
    }
}

struct ResolvedPath {
    absolute: PathBuf,
    relative: String,
}

// === 辅助函数 ===

async fn canonicalize_workspace(path: &Path) -> CheckpointResult<PathBuf> {
    let canonical = fs::canonicalize(path)
        .await
        .map_err(|e| CheckpointError::InvalidWorkspace(format!("{} ({})", path.display(), e)))?;

    let metadata = fs::metadata(&canonical).await.map_err(|e| {
        CheckpointError::InvalidWorkspace(format!("{} ({})", canonical.display(), e))
    })?;

    if !metadata.is_dir() {
        return Err(CheckpointError::InvalidWorkspace(format!(
            "{} is not a directory",
            canonical.display()
        )));
    }

    Ok(canonical)
}

async fn resolve_file_path(path: &Path, workspace_root: &Path) -> CheckpointResult<ResolvedPath> {
    if path.as_os_str().is_empty() {
        return Err(CheckpointError::InvalidFilePath("empty path".to_string()));
    }

    if path.is_absolute() {
        let absolute = match fs::canonicalize(path).await {
            Ok(p) => p,
            Err(e) if e.kind() == ErrorKind::NotFound => path.components().collect(),
            Err(e) => {
                return Err(CheckpointError::InvalidFilePath(format!(
                    "{:?} ({})",
                    path, e
                )))
            }
        };

        if !absolute.starts_with(workspace_root) {
            return Err(CheckpointError::InvalidFilePath(format!(
                "{:?} is outside workspace {:?}",
                absolute, workspace_root
            )));
        }

        let relative = absolute
            .strip_prefix(workspace_root)
            .map_err(|_| {
                CheckpointError::InvalidFilePath(format!(
                    "{:?} is outside workspace {:?}",
                    absolute, workspace_root
                ))
            })?
            .to_path_buf();

        if relative.as_os_str().is_empty() {
            return Err(CheckpointError::InvalidFilePath(
                "workspace root cannot be snapshotted".to_string(),
            ));
        }

        return Ok(ResolvedPath {
            absolute,
            relative: path_to_unix_string(&relative),
        });
    }

    let normalized = normalize_relative_path(path)
        .ok_or_else(|| CheckpointError::InvalidFilePath(path.to_string_lossy().into_owned()))?;

    if normalized.as_os_str().is_empty() {
        return Err(CheckpointError::InvalidFilePath(
            path.to_string_lossy().into_owned(),
        ));
    }

    Ok(ResolvedPath {
        absolute: workspace_root.join(&normalized),
        relative: path_to_unix_string(&normalized),
    })
}

fn normalize_relative_path(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(normalized)
}

fn path_to_unix_string(path: &Path) -> String {
    path.components()
        .filter_map(|c| {
            let part = c.as_os_str();
            if part.is_empty() {
                None
            } else {
                Some(part.to_string_lossy().to_string())
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn compute_diff(from: &[u8], to: &[u8]) -> String {
    let from_str = String::from_utf8_lossy(from).into_owned();
    let to_str = String::from_utf8_lossy(to).into_owned();
    let patch = create_patch(&from_str, &to_str);
    let formatted = PatchFormatter::new().fmt_patch(&patch).to_string();
    formatted
}
