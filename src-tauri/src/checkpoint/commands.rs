//! Checkpoint Tauri 命令接口

use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;

use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};

use super::models::{Checkpoint, CheckpointSummary, FileDiff, RollbackResult};
use super::service::CheckpointService;

/// Checkpoint 状态管理
pub struct CheckpointState {
    pub service: Arc<CheckpointService>,
}

impl CheckpointState {
    pub fn new(service: Arc<CheckpointService>) -> Self {
        Self { service }
    }
}

/// 创建 checkpoint
#[tauri::command]
pub async fn checkpoint_create(
    state: State<'_, CheckpointState>,
    conversation_id: i64,
    user_message: String,
    files: Vec<String>,
    workspace_path: String,
) -> TauriApiResult<Checkpoint> {
    let files: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
    let workspace = PathBuf::from(&workspace_path);

    match state
        .service
        .create_checkpoint(conversation_id, &user_message, files, &workspace)
        .await
    {
        Ok(checkpoint) => Ok(api_success!(checkpoint)),
        Err(e) => {
            tracing::error!("Failed to create checkpoint: {}", e);
            Ok(api_error!("checkpoint.create_failed"))
        }
    }
}

/// 获取 checkpoint 列表
#[tauri::command]
pub async fn checkpoint_list(
    state: State<'_, CheckpointState>,
    conversation_id: i64,
) -> TauriApiResult<Vec<CheckpointSummary>> {
    match state.service.list_checkpoints(conversation_id).await {
        Ok(checkpoints) => Ok(api_success!(checkpoints)),
        Err(e) => {
            tracing::error!("Failed to list checkpoints: {}", e);
            Ok(api_error!("checkpoint.list_failed"))
        }
    }
}

/// 回滚到指定 checkpoint
#[tauri::command]
pub async fn checkpoint_rollback(
    state: State<'_, CheckpointState>,
    checkpoint_id: i64,
    workspace_path: String,
) -> TauriApiResult<RollbackResult> {
    let workspace = PathBuf::from(&workspace_path);

    match state.service.rollback_to(checkpoint_id, &workspace).await {
        Ok(result) => Ok(api_success!(result)),
        Err(e) => {
            tracing::error!("Failed to rollback to checkpoint {}: {}", checkpoint_id, e);
            Ok(api_error!("checkpoint.rollback_failed"))
        }
    }
}

/// 获取两个 checkpoint 之间的 diff
#[tauri::command]
pub async fn checkpoint_diff(
    state: State<'_, CheckpointState>,
    from_id: i64,
    to_id: i64,
) -> TauriApiResult<Vec<FileDiff>> {
    match state.service.diff_checkpoints(from_id, to_id).await {
        Ok(diffs) => Ok(api_success!(diffs)),
        Err(e) => {
            tracing::error!("Failed to compute checkpoint diff: {}", e);
            Ok(api_error!("checkpoint.diff_failed"))
        }
    }
}

/// 获取当前文件与 checkpoint 的 diff
#[tauri::command]
pub async fn checkpoint_diff_with_current(
    state: State<'_, CheckpointState>,
    checkpoint_id: i64,
    file_path: String,
    workspace_path: String,
) -> TauriApiResult<Option<String>> {
    let workspace = PathBuf::from(&workspace_path);

    match state
        .service
        .diff_with_current(checkpoint_id, &file_path, &workspace)
        .await
    {
        Ok(diff) => Ok(api_success!(diff)),
        Err(e) => {
            tracing::error!("Failed to compute diff with current: {}", e);
            Ok(api_error!("checkpoint.diff_current_failed"))
        }
    }
}

/// 获取 checkpoint 中某个文件的内容
#[tauri::command]
pub async fn checkpoint_get_file_content(
    state: State<'_, CheckpointState>,
    checkpoint_id: i64,
    file_path: String,
) -> TauriApiResult<Option<String>> {
    match state
        .service
        .get_file_content(checkpoint_id, &file_path)
        .await
    {
        Ok(content) => {
            let text = content.map(|c| String::from_utf8_lossy(&c).into_owned());
            Ok(api_success!(text))
        }
        Err(e) => {
            tracing::error!("Failed to get file content: {}", e);
            Ok(api_error!("checkpoint.get_content_failed"))
        }
    }
}

/// 删除 checkpoint
#[tauri::command]
pub async fn checkpoint_delete(
    state: State<'_, CheckpointState>,
    checkpoint_id: i64,
) -> TauriApiResult<EmptyData> {
    match state.service.delete_checkpoint(checkpoint_id).await {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("Failed to delete checkpoint {}: {}", checkpoint_id, e);
            Ok(api_error!("checkpoint.delete_failed"))
        }
    }
}
