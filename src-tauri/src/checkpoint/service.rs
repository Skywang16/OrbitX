//! CheckpointService：核心服务层
//!
//! 提供 checkpoint 的创建、回滚、diff 等核心功能

use std::collections::HashMap;
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
        // 获取父 checkpoint
        let parent = self.storage.get_latest_checkpoint(conversation_id).await?;
        let parent_id = parent.as_ref().map(|p| p.id);

        // 获取父 checkpoint 的文件快照（用于计算变更类型）
        let parent_snapshots = if let Some(ref p) = parent {
            let snapshots = self.storage.get_file_snapshots(p.id).await?;
            snapshots
                .into_iter()
                .map(|s| (s.file_path.clone(), s.blob_hash.clone()))
                .collect::<HashMap<_, _>>()
        } else {
            HashMap::new()
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
        for file_path in files {
            let abs_path = if file_path.is_absolute() {
                file_path.clone()
            } else {
                workspace_path.join(&file_path)
            };

            // 读取文件内容
            let content = match fs::read(&abs_path).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        "CheckpointService: failed to read file {:?}: {}",
                        abs_path,
                        e
                    );
                    continue;
                }
            };

            // 存储到 BlobStore
            let blob_hash = self.blob_store.store(&content).await?;
            let file_size = content.len() as i64;

            // 计算变更类型
            let rel_path = file_path.to_string_lossy().to_string();
            let change_type = match parent_snapshots.get(&rel_path) {
                None => FileChangeType::Added,
                Some(old_hash) if old_hash != &blob_hash => FileChangeType::Modified,
                Some(_) => {
                    // 内容相同，减少引用计数（因为 store 会增加）
                    self.blob_store.decrement_ref(&blob_hash).await?;
                    continue; // 跳过未变更的文件
                }
            };

            file_snapshots.push(NewFileSnapshot {
                checkpoint_id,
                file_path: rel_path,
                blob_hash,
                change_type,
                file_size,
            });
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
        // 获取目标 checkpoint
        let checkpoint = self
            .storage
            .get_checkpoint(checkpoint_id)
            .await?
            .ok_or(CheckpointError::NotFound(checkpoint_id))?;

        // 获取文件快照
        let snapshots = self.storage.get_file_snapshots(checkpoint_id).await?;

        let mut restored_files = Vec::new();
        let mut failed_files = Vec::new();

        // 恢复每个文件
        for snapshot in &snapshots {
            let abs_path = workspace_path.join(&snapshot.file_path);

            // 获取 blob 内容
            let content = match self.blob_store.get(&snapshot.blob_hash).await? {
                Some(c) => c,
                None => {
                    failed_files.push((
                        snapshot.file_path.clone(),
                        format!("Blob not found: {}", snapshot.blob_hash),
                    ));
                    continue;
                }
            };

            // 确保父目录存在
            if let Some(parent) = abs_path.parent() {
                if let Err(e) = fs::create_dir_all(parent).await {
                    failed_files.push((
                        snapshot.file_path.clone(),
                        format!("Create dir failed: {}", e),
                    ));
                    continue;
                }
            }

            // 写入文件
            match fs::write(&abs_path, &content).await {
                Ok(_) => {
                    restored_files.push(snapshot.file_path.clone());
                }
                Err(e) => {
                    failed_files.push((snapshot.file_path.clone(), e.to_string()));
                }
            }
        }

        // 创建回滚后的新 checkpoint
        let files: Vec<PathBuf> = restored_files.iter().map(PathBuf::from).collect();
        let new_checkpoint = self
            .create_checkpoint(
                checkpoint.conversation_id,
                &format!("Rollback to checkpoint #{}", checkpoint_id),
                files,
                workspace_path,
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
    ) -> CheckpointResult<Vec<FileDiff>> {
        let from_snapshots = self.storage.get_file_snapshots(from_id).await?;
        let to_snapshots = self.storage.get_file_snapshots(to_id).await?;

        let from_map: HashMap<String, String> = from_snapshots
            .into_iter()
            .map(|s| (s.file_path, s.blob_hash))
            .collect();

        let to_map: HashMap<String, String> = to_snapshots
            .into_iter()
            .map(|s| (s.file_path, s.blob_hash))
            .collect();

        let mut diffs = Vec::new();

        // 检查 to 中的文件
        for (path, to_hash) in &to_map {
            match from_map.get(path) {
                None => {
                    // 新增文件
                    diffs.push(FileDiff {
                        file_path: path.clone(),
                        change_type: FileChangeType::Added,
                        diff_content: None,
                    });
                }
                Some(from_hash) if from_hash != to_hash => {
                    // 修改文件，计算 diff
                    let from_content = self.blob_store.get(from_hash).await?.unwrap_or_default();
                    let to_content = self.blob_store.get(to_hash).await?.unwrap_or_default();

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
        for path in from_map.keys() {
            if !to_map.contains_key(path) {
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
        // 获取 checkpoint 中的文件快照
        let snapshot = self
            .storage
            .get_file_snapshot(checkpoint_id, file_path)
            .await?;

        let snapshot = match snapshot {
            Some(s) => s,
            None => return Ok(None),
        };

        // 获取 checkpoint 中的内容
        let checkpoint_content = self
            .blob_store
            .get(&snapshot.blob_hash)
            .await?
            .unwrap_or_default();

        // 读取当前文件内容
        let abs_path = workspace_path.join(file_path);
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
}
