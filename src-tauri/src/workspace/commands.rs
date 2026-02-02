/*!
 * Workspace Commands
 *
 * 工作区相关的 Tauri 命令
 * 包含：工作区管理、会话管理、项目规则管理
 */

use super::rules::get_available_rules_files;
use super::{SessionRecord, WorkspaceRecord, WorkspaceService};
use crate::agent::types::Message;
use crate::storage::repositories::AppPreferences;
use crate::storage::{DatabaseManager, UnifiedCache};
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use std::sync::Arc;
use tauri::State;

// ===== 工作区管理命令 =====

#[tauri::command]
pub async fn workspace_get_recent(
    limit: Option<i64>,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<Vec<WorkspaceRecord>> {
    let limit = limit.unwrap_or(10).clamp(1, 50);
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.list_recent_workspaces(limit).await {
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
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.get_or_create_workspace(&path).await {
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
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.delete_workspace(&path).await {
        Ok(()) => Ok(api_success!()),
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
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.maintain(30, 50).await {
        Ok(counts) => Ok(api_success!(counts)),
        Err(e) => {
            tracing::error!("Failed to maintain workspaces: {}", e);
            Ok(api_error!("workspace.recent.maintain_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_get_or_create(
    path: String,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<WorkspaceRecord> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.get_or_create_workspace(&path).await {
        Ok(record) => Ok(api_success!(record)),
        Err(err) => {
            tracing::error!("workspace_get_or_create failed: {}", err);
            Ok(api_error!("workspace.get_failed"))
        }
    }
}

// ===== 会话管理命令 =====

#[tauri::command]
pub async fn workspace_list_sessions(
    path: String,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<Vec<SessionRecord>> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.list_sessions(&path).await {
        Ok(records) => Ok(api_success!(records)),
        Err(err) => {
            tracing::error!("workspace_list_sessions failed: {}", err);
            Ok(api_error!("workspace.sessions_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_get_messages(
    session_id: i64,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<Vec<Message>> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.get_session_messages(session_id).await {
        Ok(records) => Ok(api_success!(records)),
        Err(err) => {
            tracing::error!("workspace_get_messages failed: {}", err);
            Ok(api_error!("workspace.messages_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_get_active_session(
    path: String,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<SessionRecord> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.ensure_active_session(&path).await {
        Ok(session) => Ok(api_success!(session)),
        Err(err) => {
            tracing::error!("workspace_get_active_session failed: {}", err);
            Ok(api_error!("workspace.active_session_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_create_session(
    path: String,
    title: Option<String>,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<SessionRecord> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.create_session(&path, title.as_deref()).await {
        Ok(session) => Ok(api_success!(session)),
        Err(err) => {
            tracing::error!("workspace_create_session failed: {}", err);
            Ok(api_error!("workspace.create_session_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_set_active_session(
    path: String,
    session_id: i64,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<EmptyData> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.set_active_session(&path, Some(session_id)).await {
        Ok(()) => Ok(api_success!()),
        Err(err) => {
            tracing::error!("workspace_set_active_session failed: {}", err);
            Ok(api_error!("workspace.set_active_session_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_clear_active_session(
    path: String,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<EmptyData> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.set_active_session(&path, None).await {
        Ok(()) => Ok(api_success!()),
        Err(err) => {
            tracing::error!("workspace_clear_active_session failed: {}", err);
            Ok(api_error!("workspace.clear_active_session_failed"))
        }
    }
}

// ===== 项目规则管理命令 =====

/// 获取当前项目规则
#[tauri::command]
pub async fn workspace_get_project_rules(
    database: State<'_, Arc<DatabaseManager>>,
    cache: State<'_, Arc<UnifiedCache>>,
) -> TauriApiResult<Option<String>> {
    match AppPreferences::new(&database)
        .get("workspace.project_rules")
        .await
    {
        Ok(value) => {
            // 同步缓存，保证 Prompt 构建使用最新数据
            let _ = cache.set_project_rules(value.clone()).await;
            Ok(api_success!(value))
        }
        Err(e) => {
            tracing::error!("Failed to load project rules: {}", e);
            Ok(api_error!("workspace.rules.load_failed"))
        }
    }
}

/// 设置项目规则
#[tauri::command]
pub async fn workspace_set_project_rules(
    rules: Option<String>,
    database: State<'_, Arc<DatabaseManager>>,
    cache: State<'_, Arc<UnifiedCache>>,
) -> TauriApiResult<EmptyData> {
    match AppPreferences::new(&database)
        .set("workspace.project_rules", rules.as_deref())
        .await
    {
        Ok(_) => {
            let _ = cache.set_project_rules(rules).await;
            Ok(api_success!())
        }
        Err(e) => {
            tracing::error!("Failed to persist project rules: {}", e);
            Ok(api_error!("workspace.rules.save_failed"))
        }
    }
}

/// 列出指定目录下所有可用的规则文件
#[tauri::command]
pub async fn workspace_list_rules_files(cwd: String) -> TauriApiResult<Vec<String>> {
    let files = get_available_rules_files(cwd);
    Ok(api_success!(files))
}
