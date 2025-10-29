/*!
 * Workspace Commands
 *
 * 工作区相关的 Tauri 命令
 * 包含：最近工作区管理、项目规则管理
 */

use super::rules::get_available_rules_files;
use crate::storage::{DatabaseManager, UnifiedCache};
use crate::storage::repositories::{RecentWorkspace, RecentWorkspaces};
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use std::sync::Arc;
use tauri::State;

// ===== 最近工作区管理命令 =====

#[tauri::command]
pub async fn workspace_get_recent(
    limit: Option<i64>,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<Vec<RecentWorkspace>> {
    let limit = limit.unwrap_or(10).min(50);
    match RecentWorkspaces::new(&database)
        .get_recent(limit)
        .await
    {
        Ok(workspaces) => Ok(api_success!(workspaces)),
        Err(e) => {
            tracing::error!("Failed to get recent workspaces: {}", e);
            Ok(api_error!("workspace.recent.get_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_add_recent(
    path: String,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<EmptyData> {
    match RecentWorkspaces::new(&database)
        .add_or_update(&path)
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("Failed to add recent workspace: {}", e);
            Ok(api_error!("workspace.recent.add_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_remove_recent(
    path: String,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<EmptyData> {
    match RecentWorkspaces::new(&database)
        .remove(&path)
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("Failed to remove recent workspace: {}", e);
            Ok(api_error!("workspace.recent.remove_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_maintain(
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<(u64, u64)> {
    // 清理 30 天未访问 + 限制最多 50 条
    match RecentWorkspaces::new(&database)
        .maintain(30, 50)
        .await
    {
        Ok(counts) => Ok(api_success!(counts)),
        Err(e) => {
            tracing::error!("Failed to maintain workspaces: {}", e);
            Ok(api_error!("workspace.recent.maintain_failed"))
        }
    }
}

// ===== 项目规则管理命令 =====

/// 获取当前项目规则
#[tauri::command]
pub async fn workspace_get_project_rules(
    cache: State<'_, Arc<UnifiedCache>>,
) -> TauriApiResult<Option<String>> {
    let rules = cache.get_project_rules().await;
    Ok(api_success!(rules))
}

/// 设置项目规则
#[tauri::command]
pub async fn workspace_set_project_rules(
    rules: Option<String>,
    cache: State<'_, Arc<UnifiedCache>>,
) -> TauriApiResult<EmptyData> {
    cache.set_project_rules(rules).await.ok();
    Ok(api_success!())
}

/// 列出指定目录下所有可用的规则文件
#[tauri::command]
pub async fn workspace_list_rules_files(cwd: String) -> TauriApiResult<Vec<String>> {
    let files = get_available_rules_files(cwd);
    Ok(api_success!(files))
}
