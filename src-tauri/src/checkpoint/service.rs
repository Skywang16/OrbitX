//! CheckpointService：核心服务层
//!
//! 提供 checkpoint 的创建、回滚、diff 等核心功能

use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use diffy::{create_patch, PatchFormatter};
use tokio::fs;

use super::blob_store::BlobStore;
use super::models::{
    Checkpoint, CheckpointError, CheckpointResult, CheckpointSummary, FileChangeType, FileDiff,
    NewCheckpoint, NewFileSnapshot, RollbackResult,
};
use super::storage::CheckpointStorage;

/// Checkpoint 核心服务
pub struct CheckpointService {
    storage: Arc<CheckpointStorage>,
    blob_store: Arc<BlobStore>,
}

impl CheckpointService {
    pub fn new(storage: Arc<CheckpointStorage>, blob_store: Arc<BlobStore>) -> Self {
        Self {
            storage,
            blob_store,
        }
    }

    /// 创建新的 checkpoint
    ///
    /// # Arguments
    /// * `conversation_id` - 会话 ID
    /// * `user_message` - 用户消息
    /// * `files` - 需要快照的文件路径列表
    /// * `workspace_path` - 工作区根路径
    pub async fn create_checkpoint(
        &self,
        conversation_id: i64,
        user_message: &str,
        files: Vec<PathBuf>,
        workspace_path: &Path,
    ) -> CheckpointResult<Checkpoint> {
        let workspace_root = Self::canonicalize_workspace(workspace_path).await?;

        // 获取父 checkpoint
        let parent = self.storage.get_latest_checkpoint(conversation_id).await?;
        let parent_id = parent.as_ref().map(|p| p.id);

        let parent_state = if let Some(ref p) = parent {
            self.build_checkpoint_state(p.id, &workspace_root).await?
        } else {
            AggregatedState::new()
        };

        // 创建 checkpoint 记录
        let new_checkpoint = NewCheckpoint {
            conversation_id,
            parent_id,
            user_message: user_message.to_string(),
        };
        let checkpoint_id = self.storage.insert_checkpoint(&new_checkpoint).await?;

        // 处理每个文件
        let mut file_snapshots = Vec::new();
        let mut seen_paths = HashSet::new();

        for raw_path in files {
            let resolved = match Self::resolve_input_path(&raw_path, &workspace_root).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(
                        "CheckpointService: invalid path {:?} for workspace {:?}: {}",
                        raw_path,
                        workspace_root,
                        e
                    );
                    continue;
                }
            };

            if !seen_paths.insert(resolved.relative.clone()) {
                continue;
            }

            match fs::read(&resolved.absolute).await {
                Ok(content) => {
                    let blob_hash = self.blob_store.store(&content).await?;
                    let file_size = content.len() as i64;

                    let change_type = match parent_state.files.get(&resolved.relative) {
                        None => FileChangeType::Added,
                        Some(existing) if existing.blob_hash != blob_hash => {
                            FileChangeType::Modified
                        }
                        Some(_) => {
                            self.blob_store.decrement_ref(&blob_hash).await?;
                            continue;
                        }
                    };

                    file_snapshots.push(NewFileSnapshot {
                        checkpoint_id,
                        file_path: resolved.relative.clone(),
                        blob_hash,
                        change_type,
                        file_size,
                    });
                }
                Err(err) if err.kind() == ErrorKind::NotFound => {
                    if parent_state.files.contains_key(&resolved.relative) {
                        file_snapshots.push(NewFileSnapshot {
                            checkpoint_id,
                            file_path: resolved.relative.clone(),
                            blob_hash: String::new(),
                            change_type: FileChangeType::Deleted,
                            file_size: 0,
                        });
                    } else {
                        tracing::warn!(
                            "CheckpointService: file {:?} missing and not tracked previously",
                            resolved.absolute
                        );
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        "CheckpointService: failed to read file {:?}: {}",
                        resolved.absolute,
                        err
                    );
                }
            }
        }

        // 批量插入文件快照
        if !file_snapshots.is_empty() {
            self.storage.insert_file_snapshots(&file_snapshots).await?;
        }

        // 返回创建的 checkpoint
        let checkpoint = self
            .storage
            .get_checkpoint(checkpoint_id)
            .await?
            .ok_or(CheckpointError::NotFound(checkpoint_id))?;

        tracing::info!(
            "CheckpointService: created checkpoint id={}, files={}",
            checkpoint_id,
            file_snapshots.len()
        );

        Ok(checkpoint)
    }

    /// 获取 checkpoint 列表
    pub async fn list_checkpoints(
        &self,
        conversation_id: i64,
    ) -> CheckpointResult<Vec<CheckpointSummary>> {
        self.storage
            .list_summaries_by_conversation(conversation_id)
            .await
    }

    /// 获取 checkpoint 详情
    pub async fn get_checkpoint(&self, checkpoint_id: i64) -> CheckpointResult<Option<Checkpoint>> {
        self.storage.get_checkpoint(checkpoint_id).await
    }

    /// 回滚到指定 checkpoint
    pub async fn rollback_to(
        &self,
        checkpoint_id: i64,
        workspace_path: &Path,
    ) -> CheckpointResult<RollbackResult> {
        let workspace_root = Self::canonicalize_workspace(workspace_path).await?;

        // 获取目标 checkpoint
        let checkpoint = self
            .storage
            .get_checkpoint(checkpoint_id)
            .await?
            .ok_or(CheckpointError::NotFound(checkpoint_id))?;

        let state = self
            .build_checkpoint_state(checkpoint_id, &workspace_root)
            .await?;

        let mut restored_files = Vec::new();
        let mut failed_files = Vec::new();
        let mut snapshot_targets = HashSet::new();

        // 恢复每个文件
        for (relative_path, file_state) in &state.files {
            let abs_path = workspace_root.join(relative_path);

            // 获取 blob 内容
            let content = match self.blob_store.get(&file_state.blob_hash).await? {
                Some(c) => c,
                None => {
                    failed_files.push((
                        relative_path.clone(),
                        format!("Blob not found: {}", file_state.blob_hash),
                    ));
                    continue;
                }
            };

            // 确保父目录存在
            if let Some(parent) = abs_path.parent() {
                if let Err(e) = fs::create_dir_all(parent).await {
                    failed_files.push((
                        relative_path.clone(),
                        format!("Create dir failed: {}", e),
                    ));
                    continue;
                }
            }

            // 写入文件
            match fs::write(&abs_path, &content).await {
                Ok(_) => {
                    restored_files.push(relative_path.clone());
                    snapshot_targets.insert(relative_path.clone());
                }
                Err(e) => {
                    failed_files.push((relative_path.clone(), e.to_string()));
                }
            }
        }

        // 删除应当不存在的文件
        for relative_path in &state.tombstones {
            let abs_path = workspace_root.join(relative_path);
            match fs::remove_file(&abs_path).await {
                Ok(_) => {
                    restored_files.push(relative_path.clone());
                    snapshot_targets.insert(relative_path.clone());
                }
                Err(err) if err.kind() == ErrorKind::NotFound => {
                    snapshot_targets.insert(relative_path.clone());
                }
                Err(err) => {
                    failed_files.push((relative_path.clone(), err.to_string()));
                }
            }
        }

        // 创建回滚后的新 checkpoint
        let files: Vec<PathBuf> = snapshot_targets.iter().map(PathBuf::from).collect();
        let new_checkpoint = self
            .create_checkpoint(
                checkpoint.conversation_id,
                &format!("Rollback to checkpoint #{}", checkpoint_id),
                files,
                &workspace_root,
            )
            .await?;

        tracing::info!(
            "CheckpointService: rollback to checkpoint {}, restored={}, failed={}",
            checkpoint_id,
            restored_files.len(),
            failed_files.len()
        );

        Ok(RollbackResult {
            checkpoint_id,
            new_checkpoint_id: new_checkpoint.id,
            restored_files,
            failed_files,
        })
    }

    /// 计算两个 checkpoint 之间的 diff
    pub async fn diff_checkpoints(
        &self,
        from_id: i64,
        to_id: i64,
        workspace_path: &Path,
    ) -> CheckpointResult<Vec<FileDiff>> {
        let workspace_root = Self::canonicalize_workspace(workspace_path).await?;

        let from_state = self
            .build_checkpoint_state(from_id, &workspace_root)
            .await?;
        let to_state = self
            .build_checkpoint_state(to_id, &workspace_root)
            .await?;

        let mut diffs = Vec::new();

        // 检查 to 中的文件
        for (path, to_entry) in &to_state.files {
            match from_state.files.get(path) {
                None => {
                    // 新增文件
                    diffs.push(FileDiff {
                        file_path: path.clone(),
                        change_type: FileChangeType::Added,
                        diff_content: None,
                    });
                }
                Some(from_entry) if from_entry.blob_hash != to_entry.blob_hash => {
                    // 修改文件，计算 diff
                    let from_content = self
                        .blob_store
                        .get(&from_entry.blob_hash)
                        .await?
                        .unwrap_or_default();
                    let to_content = self
                        .blob_store
                        .get(&to_entry.blob_hash)
                        .await?
                        .unwrap_or_default();

                    let diff_content = self.compute_diff(&from_content, &to_content);
                    diffs.push(FileDiff {
                        file_path: path.clone(),
                        change_type: FileChangeType::Modified,
                        diff_content: Some(diff_content),
                    });
                }
                _ => {} // 未变更
            }
        }

        // 检查删除的文件
        for path in from_state.files.keys() {
            if !to_state.files.contains_key(path) {
                diffs.push(FileDiff {
                    file_path: path.clone(),
                    change_type: FileChangeType::Deleted,
                    diff_content: None,
                });
            }
        }

        Ok(diffs)
    }

    /// 计算当前文件与 checkpoint 的 diff
    pub async fn diff_with_current(
        &self,
        checkpoint_id: i64,
        file_path: &str,
        workspace_path: &Path,
    ) -> CheckpointResult<Option<String>> {
        let workspace_root = Self::canonicalize_workspace(workspace_path).await?;
        let normalized = match Self::normalize_relative_str(file_path) {
            Some(p) => p,
            None => return Ok(None),
        };

        let state = self
            .build_checkpoint_state(checkpoint_id, &workspace_root)
            .await?;

        let entry = match state.files.get(&normalized) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        let checkpoint_content = self
            .blob_store
            .get(&entry.blob_hash)
            .await?
            .unwrap_or_default();

        let abs_path = workspace_root.join(Path::new(&normalized));
        let current_content = fs::read(&abs_path).await.unwrap_or_default();

        let diff = self.compute_diff(&checkpoint_content, &current_content);
        Ok(Some(diff))
    }

    /// 获取 checkpoint 中某个文件的内容
    pub async fn get_file_content(
        &self,
        checkpoint_id: i64,
        file_path: &str,
    ) -> CheckpointResult<Option<Vec<u8>>> {
        let snapshot = self
            .storage
            .get_file_snapshot(checkpoint_id, file_path)
            .await?;

        match snapshot {
            Some(s) => self.blob_store.get(&s.blob_hash).await,
            None => Ok(None),
        }
    }

    /// 计算 unified diff
    fn compute_diff(&self, from: &[u8], to: &[u8]) -> String {
        let from_str = String::from_utf8_lossy(from).into_owned();
        let to_str = String::from_utf8_lossy(to).into_owned();

        let patch = create_patch(&from_str, &to_str);
        let formatter = PatchFormatter::new();
        let result = formatter.fmt_patch(&patch).to_string();
        result
    }

    /// 删除 checkpoint 并清理 blob
    pub async fn delete_checkpoint(&self, checkpoint_id: i64) -> CheckpointResult<()> {
        // 获取 checkpoint 引用的所有 blob
        let blob_hashes = self.storage.get_blob_hashes(checkpoint_id).await?;

        // 删除 checkpoint（会级联删除 file_snapshots）
        self.storage.delete_checkpoint(checkpoint_id).await?;

        // 减少 blob 引用计数
        for hash in blob_hashes {
            self.blob_store.decrement_ref(&hash).await?;
        }

        // 执行垃圾回收
        self.blob_store.gc().await?;

        Ok(())
    }

    async fn canonicalize_workspace(workspace_path: &Path) -> CheckpointResult<PathBuf> {
        let canonical = fs::canonicalize(workspace_path)
            .await
            .map_err(|e| {
                CheckpointError::InvalidWorkspace(format!(
                    "{} ({})",
                    workspace_path.display(),
                    e
                ))
            })?;
        let metadata = fs::metadata(&canonical).await.map_err(|e| {
            CheckpointError::InvalidWorkspace(format!(
                "{} ({})",
                canonical.display(),
                e
            ))
        })?;

        if !metadata.is_dir() {
            return Err(CheckpointError::InvalidWorkspace(format!(
                "{} is not a directory",
                canonical.display()
            )));
        }

        Ok(canonical)
    }

    async fn resolve_input_path(
        path: &Path,
        workspace_root: &Path,
    ) -> CheckpointResult<ResolvedPath> {
        if path.as_os_str().is_empty() {
            return Err(CheckpointError::InvalidFilePath(
                "empty path".to_string(),
            ));
        }

        if path.is_absolute() {
            let absolute = match fs::canonicalize(path).await {
                Ok(p) => p,
                Err(err) if err.kind() == ErrorKind::NotFound => {
                    path.components().collect::<PathBuf>()
                }
                Err(err) => {
                    return Err(CheckpointError::InvalidFilePath(format!(
                        "{:?} ({})",
                        path, err
                    )))
                }
            };

            return Self::finalize_absolute(absolute, workspace_root);
        }

        let normalized = Self::normalize_relative_path(path).ok_or_else(|| {
            CheckpointError::InvalidFilePath(path.to_string_lossy().into_owned())
        })?;

        if normalized.as_os_str().is_empty() {
            return Err(CheckpointError::InvalidFilePath(
                path.to_string_lossy().into_owned(),
            ));
        }

        Ok(ResolvedPath {
            absolute: workspace_root.join(&normalized),
            relative: Self::path_to_unix_string(&normalized),
        })
    }

    fn finalize_absolute(
        absolute: PathBuf,
        workspace_root: &Path,
    ) -> CheckpointResult<ResolvedPath> {
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

        Ok(ResolvedPath {
            absolute,
            relative: Self::path_to_unix_string(&relative),
        })
    }

    fn normalize_relative_path(path: &Path) -> Option<PathBuf> {
        use std::path::Component;

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

    fn normalize_relative_str(input: &str) -> Option<String> {
        if input.trim().is_empty() {
            return None;
        }

        let path = Path::new(input);
        let normalized = Self::normalize_relative_path(path)?;
        if normalized.as_os_str().is_empty() {
            return None;
        }
        Some(Self::path_to_unix_string(&normalized))
    }

    fn path_to_unix_string(path: &Path) -> String {
        path.components()
            .filter_map(|component| {
                let part = component.as_os_str();
                if part.is_empty() {
                    None
                } else {
                    Some(part.to_string_lossy().to_string())
                }
            })
            .collect::<Vec<String>>()
            .join("/")
    }

    fn normalize_stored_path(stored: &str, workspace_root: &Path) -> Option<String> {
        if stored.trim().is_empty() {
            return None;
        }

        let candidate = PathBuf::from(stored);
        if candidate.is_absolute() {
            let cleaned = candidate.components().collect::<PathBuf>();
            if !cleaned.starts_with(workspace_root) {
                return None;
            }
            let relative = cleaned.strip_prefix(workspace_root).ok()?;
            if relative.as_os_str().is_empty() {
                return None;
            }
            Some(Self::path_to_unix_string(relative))
        } else {
            let normalized = Self::normalize_relative_path(&candidate)?;
            if normalized.as_os_str().is_empty() {
                return None;
            }
            Some(Self::path_to_unix_string(&normalized))
        }
    }

    async fn build_checkpoint_state(
        &self,
        checkpoint_id: i64,
        workspace_root: &Path,
    ) -> CheckpointResult<AggregatedState> {
        let lineage = self.collect_lineage(checkpoint_id).await?;
        let mut state = AggregatedState::new();

        for ancestor in lineage {
            let snapshots = self.storage.get_file_snapshots(ancestor).await?;
            for snapshot in snapshots {
                let normalized =
                    match Self::normalize_stored_path(&snapshot.file_path, workspace_root) {
                        Some(path) => path,
                        None => {
                            tracing::warn!(
                                "CheckpointService: skip path {:?} outside workspace {:?}",
                                snapshot.file_path,
                                workspace_root
                            );
                            continue;
                        }
                    };

                match snapshot.change_type {
                    FileChangeType::Added | FileChangeType::Modified => {
                        state.tombstones.remove(&normalized);
                        state.files.insert(
                            normalized,
                            FileState {
                                blob_hash: snapshot.blob_hash,
                            },
                        );
                    }
                    FileChangeType::Deleted => {
                        state.files.remove(&normalized);
                        state.tombstones.insert(normalized);
                    }
                }
            }
        }

        Ok(state)
    }

    async fn collect_lineage(&self, checkpoint_id: i64) -> CheckpointResult<Vec<i64>> {
        let mut lineage = Vec::new();
        let mut current = Some(checkpoint_id);

        while let Some(id) = current {
            let checkpoint = self
                .storage
                .get_checkpoint(id)
                .await?
                .ok_or(CheckpointError::NotFound(id))?;
            lineage.push(id);
            current = checkpoint.parent_id;
        }

        lineage.reverse();
        Ok(lineage)
    }
}

struct ResolvedPath {
    absolute: PathBuf,
    relative: String,
}

struct FileState {
    blob_hash: String,
}

#[derive(Default)]
struct AggregatedState {
    files: HashMap<String, FileState>,
    tombstones: HashSet<String>,
}

impl AggregatedState {
    fn new() -> Self {
        Self::default()
    }
}
