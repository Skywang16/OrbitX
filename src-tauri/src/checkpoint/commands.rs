//! Checkpoint Tauri 命令

use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;

use crate::storage::DatabaseManager;
use crate::utils::TauriApiResult;
use crate::workspace::WorkspaceService;
use crate::{api_error, api_success};

use super::models::{CheckpointSummary, FileDiff, RollbackResult};
use super::service::CheckpointService;

/// Checkpoint 状态
pub struct CheckpointState {
    pub service: Arc<CheckpointService>,
}

impl CheckpointState {
    pub fn new(service: Arc<CheckpointService>) -> Self {
        Self { service }
    }
}

/// 获取会话的 checkpoint 列表
#[tauri::command]
pub async fn checkpoint_list(
    state: State<'_, CheckpointState>,
    session_id: i64,
    workspace_path: String,
) -> TauriApiResult<Vec<CheckpointSummary>> {
    if workspace_path.trim().is_empty() {
        return Ok(api_error!("common.invalid_path"));
    }

    match state
        .service
        .list_by_session(session_id, &workspace_path)
        .await
    {
        Ok(checkpoints) => Ok(api_success!(checkpoints)),
        Err(e) => {
            tracing::error!("Failed to list checkpoints: {}", e);
            Ok(api_error!("checkpoint.list_failed"))
        }
    }
}

/// 回滚到指定 checkpoint
///
/// 只需要 checkpoint_id，其他信息从 checkpoint 记录中获取
#[tauri::command]
pub async fn checkpoint_rollback(
    state: State<'_, CheckpointState>,
    database: State<'_, Arc<DatabaseManager>>,
    checkpoint_id: i64,
) -> TauriApiResult<RollbackResult> {
    // 先获取 checkpoint 信息
    let checkpoint = match state.service.get(checkpoint_id).await {
        Ok(Some(cp)) => cp,
        Ok(None) => {
            return Ok(api_error!("checkpoint.not_found"));
        }
        Err(e) => {
            tracing::error!("Failed to get checkpoint {}: {}", checkpoint_id, e);
            return Ok(api_error!("checkpoint.rollback_failed"));
        }
    };

    // 执行文件回滚
    let result = match state.service.rollback(checkpoint_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to rollback to checkpoint {}: {}", checkpoint_id, e);
            return Ok(api_error!("checkpoint.rollback_failed"));
        }
    };

    // 清理消息历史（使用 checkpoint 中存储的 message_id）
    let workspace_service = WorkspaceService::new(Arc::clone(&database));
    if let Err(e) = workspace_service
        .trim_session_messages(
            &checkpoint.workspace_path,
            checkpoint.session_id,
            checkpoint.message_id,
        )
        .await
    {
        tracing::error!("Failed to trim session messages: {}", e);
        return Ok(api_error!("workspace.trim_session_failed"));
    }

    Ok(api_success!(result))
}

/// 获取两个 checkpoint 之间的 diff
#[tauri::command]
pub async fn checkpoint_diff(
    state: State<'_, CheckpointState>,
    from_id: Option<i64>,
    to_id: i64,
    workspace_path: String,
) -> TauriApiResult<Vec<FileDiff>> {
    if workspace_path.trim().is_empty() {
        return Ok(api_error!("common.invalid_path"));
    }

    let workspace = PathBuf::from(&workspace_path);

    match state.service.diff(from_id, to_id, &workspace).await {
        Ok(diffs) => Ok(api_success!(diffs)),
        Err(e) => {
            tracing::error!("Failed to compute checkpoint diff: {}", e);
            Ok(api_error!("checkpoint.diff_failed"))
        }
    }
}

/// 获取 checkpoint 与当前工作区的 diff
#[tauri::command]
pub async fn checkpoint_diff_with_workspace(
    state: State<'_, CheckpointState>,
    checkpoint_id: i64,
    workspace_path: String,
) -> TauriApiResult<Vec<FileDiff>> {
    if workspace_path.trim().is_empty() {
        return Ok(api_error!("common.invalid_path"));
    }

    let workspace = PathBuf::from(&workspace_path);

    match state
        .service
        .diff_with_workspace(checkpoint_id, &workspace)
        .await
    {
        Ok(diff) => Ok(api_success!(diff)),
        Err(e) => {
            tracing::error!("Failed to diff checkpoint with workspace: {}", e);
            Ok(api_error!("checkpoint.diff_failed"))
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
