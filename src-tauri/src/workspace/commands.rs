/*!
 * Workspace Commands
 */

use super::rules::get_available_rules_files;
use super::{RunActionRecord, ThreadRecord, ThreadViewRecord, WorkspaceRecord, WorkspaceService};
use crate::agent::types::{Message, SubagentRecord};
use crate::storage::repositories::AppPreferences;
use crate::storage::{DatabaseManager, UnifiedCache};
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

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

#[tauri::command]
pub async fn workspace_list_thread_views(
    path: String,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<Vec<ThreadViewRecord>> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.list_thread_views(&path).await {
        Ok(records) => Ok(api_success!(records)),
        Err(err) => {
            tracing::error!("workspace_list_thread_views failed: {}", err);
            Ok(api_error!("workspace.threads_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_get_thread_messages(
    thread_id: i64,
    limit: Option<i64>,
    before_id: Option<i64>,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<Vec<Message>> {
    let service = WorkspaceService::new(Arc::clone(&database));
    let limit = limit.unwrap_or(i64::MAX).clamp(1, 200);
    match service
        .get_thread_messages(thread_id, limit, before_id)
        .await
    {
        Ok(records) => Ok(api_success!(records)),
        Err(err) => {
            tracing::error!("workspace_get_thread_messages failed: {}", err);
            Ok(api_error!("workspace.messages_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_list_subagents(
    thread_id: i64,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<Vec<SubagentRecord>> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.list_subagents(thread_id).await {
        Ok(records) => Ok(api_success!(records)),
        Err(err) => {
            tracing::error!("workspace_list_subagents failed: {}", err);
            Ok(api_error!("workspace.messages_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_get_active_thread(
    path: String,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<ThreadRecord> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.ensure_active_thread(&path).await {
        Ok(thread) => Ok(api_success!(thread)),
        Err(err) => {
            tracing::error!("workspace_get_active_thread failed: {}", err);
            Ok(api_error!("workspace.active_thread_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_get_thread(
    thread_id: i64,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<ThreadRecord> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.get_thread(thread_id).await {
        Ok(Some(thread)) => Ok(api_success!(thread)),
        Ok(None) => Ok(api_error!("workspace.active_thread_failed")),
        Err(err) => {
            tracing::error!("workspace_get_thread failed: {}", err);
            Ok(api_error!("workspace.active_thread_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_create_thread(
    path: String,
    title: Option<String>,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<ThreadRecord> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.create_thread(&path, title.as_deref()).await {
        Ok(thread) => Ok(api_success!(thread)),
        Err(err) => {
            tracing::error!("workspace_create_thread failed: {}", err);
            Ok(api_error!("workspace.create_thread_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_set_active_thread(
    path: String,
    thread_id: i64,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<EmptyData> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.set_active_thread(&path, Some(thread_id)).await {
        Ok(()) => Ok(api_success!()),
        Err(err) => {
            tracing::error!("workspace_set_active_thread failed: {}", err);
            Ok(api_error!("workspace.set_active_thread_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_clear_active_thread(
    path: String,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<EmptyData> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.set_active_thread(&path, None).await {
        Ok(()) => Ok(api_success!()),
        Err(err) => {
            tracing::error!("workspace_clear_active_thread failed: {}", err);
            Ok(api_error!("workspace.clear_active_thread_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_delete_thread(
    thread_id: i64,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<EmptyData> {
    let service = WorkspaceService::new(Arc::clone(&database));
    match service.delete_thread(thread_id).await {
        Ok(()) => Ok(api_success!()),
        Err(err) => {
            tracing::error!("workspace_delete_thread failed: {}", err);
            Ok(api_error!("workspace.delete_thread_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_get_project_rules(
    cache: State<'_, Arc<UnifiedCache>>,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<Option<String>> {
    if let Ok(cached) = cache
        .get_deserialized_ns(crate::storage::cache::CacheNamespace::Rules, "global_rules")
        .await
    {
        return Ok(api_success!(cached));
    }

    let prefs = AppPreferences::new(&database);
    match prefs.get("workspace.project_rules").await {
        Ok(value) => Ok(api_success!(value)),
        Err(err) => {
            tracing::error!("workspace_get_project_rules failed: {}", err);
            Ok(api_error!("workspace.project_rules_get_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_set_project_rules(
    rules: Option<String>,
    cache: State<'_, Arc<UnifiedCache>>,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<EmptyData> {
    let prefs = AppPreferences::new(&database);
    let normalized = rules
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let result = match normalized.as_deref() {
        Some(value) => prefs.set("workspace.project_rules", Some(value)).await,
        None => prefs.set("workspace.project_rules", None).await,
    };

    match result {
        Ok(()) => {
            if let Err(err) = cache.set_global_rules(normalized).await {
                tracing::warn!("Failed to update project rules cache: {}", err);
            }
            Ok(api_success!())
        }
        Err(err) => {
            tracing::error!("workspace_set_project_rules failed: {}", err);
            Ok(api_error!("workspace.project_rules_set_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_list_rules_files(cwd: String) -> TauriApiResult<Vec<String>> {
    let files = get_available_rules_files(&cwd)
        .into_iter()
        // README.md is a fallback context source for the agent but is not a
        // user-configured "rules file", so exclude it from the UI picker.
        .filter(|f| f != "README.md")
        .collect();
    Ok(api_success!(files))
}

#[tauri::command]
pub async fn workspace_list_run_actions(
    path: String,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<Vec<RunActionRecord>> {
    let pool = database.pool();
    match sqlx::query(
        "SELECT id, workspace_path, name, command, sort_order
         FROM run_actions
         WHERE workspace_path = ?
         ORDER BY sort_order ASC, name ASC",
    )
    .bind(&path)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => {
            let mut actions = Vec::with_capacity(rows.len());
            for row in rows {
                actions.push(RunActionRecord {
                    id: row.get("id"),
                    workspace_path: row.get("workspace_path"),
                    name: row.get("name"),
                    command: row.get("command"),
                    sort_order: row.get("sort_order"),
                });
            }
            Ok(api_success!(actions))
        }
        Err(err) => {
            tracing::error!("workspace_list_run_actions failed: {}", err);
            Ok(api_error!("workspace.run_actions_list_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_create_run_action(
    path: String,
    name: String,
    command: String,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<RunActionRecord> {
    let pool = database.pool();
    let id = uuid::Uuid::new_v4().to_string();
    let sort_order: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM run_actions WHERE workspace_path = ?",
    )
    .bind(&path)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    match sqlx::query(
        "INSERT INTO run_actions (id, workspace_path, name, command, sort_order)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&path)
    .bind(&name)
    .bind(&command)
    .bind(sort_order)
    .execute(pool)
    .await
    {
        Ok(_) => Ok(api_success!(RunActionRecord {
            id,
            workspace_path: path,
            name,
            command,
            sort_order,
        })),
        Err(err) => {
            tracing::error!("workspace_create_run_action failed: {}", err);
            Ok(api_error!("workspace.run_action_create_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_update_run_action(
    id: String,
    name: String,
    command: String,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<EmptyData> {
    match sqlx::query("UPDATE run_actions SET name = ?, command = ? WHERE id = ?")
        .bind(&name)
        .bind(&command)
        .bind(&id)
        .execute(database.pool())
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(err) => {
            tracing::error!("workspace_update_run_action failed: {}", err);
            Ok(api_error!("workspace.run_action_update_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_delete_run_action(
    id: String,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<EmptyData> {
    match sqlx::query("DELETE FROM run_actions WHERE id = ?")
        .bind(&id)
        .execute(database.pool())
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(err) => {
            tracing::error!("workspace_delete_run_action failed: {}", err);
            Ok(api_error!("workspace.run_action_delete_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_set_selected_run_action(
    path: String,
    action_id: Option<String>,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<EmptyData> {
    match sqlx::query("UPDATE workspaces SET selected_run_action_id = ? WHERE path = ?")
        .bind(action_id)
        .bind(path)
        .execute(database.pool())
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(err) => {
            tracing::error!("workspace_set_selected_run_action failed: {}", err);
            Ok(api_error!("workspace.run_action_select_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_get_selected_run_action(
    path: String,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<Option<String>> {
    match sqlx::query_scalar::<_, Option<String>>(
        "SELECT selected_run_action_id FROM workspaces WHERE path = ?",
    )
    .bind(path)
    .fetch_optional(database.pool())
    .await
    {
        Ok(value) => Ok(api_success!(value.flatten())),
        Err(err) => {
            tracing::error!("workspace_get_selected_run_action failed: {}", err);
            Ok(api_error!("workspace.run_action_get_selected_failed"))
        }
    }
}

#[tauri::command]
pub async fn workspace_get_run_actions_by_paths(
    paths: Vec<String>,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<HashMap<String, Vec<RunActionRecord>>> {
    let pool = database.pool();
    let mut result = HashMap::new();
    for path in paths {
        match sqlx::query(
            "SELECT id, workspace_path, name, command, sort_order
             FROM run_actions
             WHERE workspace_path = ?
             ORDER BY sort_order ASC, name ASC",
        )
        .bind(&path)
        .fetch_all(pool)
        .await
        {
            Ok(rows) => {
                let mut actions = Vec::with_capacity(rows.len());
                for row in rows {
                    actions.push(RunActionRecord {
                        id: row.get("id"),
                        workspace_path: row.get("workspace_path"),
                        name: row.get("name"),
                        command: row.get("command"),
                        sort_order: row.get("sort_order"),
                    });
                }
                result.insert(path, actions);
            }
            Err(err) => {
                tracing::error!(
                    "workspace_get_run_actions_by_paths failed for path: {}",
                    err
                );
                result.insert(path, Vec::new());
            }
        }
    }
    Ok(api_success!(result))
}

#[tauri::command]
pub async fn preferences_get_batch(
    keys: Vec<String>,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<HashMap<String, String>> {
    let prefs = AppPreferences::new(&database);
    let mut values = HashMap::new();
    for key in keys {
        match prefs.get(&key).await {
            Ok(Some(value)) => {
                values.insert(key, value);
            }
            Ok(None) => {}
            Err(err) => {
                tracing::error!("preferences_get_batch failed: {}", err);
                return Ok(api_error!("workspace.preferences_get_failed"));
            }
        }
    }
    Ok(api_success!(values))
}

#[tauri::command]
pub async fn preferences_set(
    key: String,
    value: Option<String>,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<EmptyData> {
    let prefs = AppPreferences::new(&database);
    let result = match value {
        Some(value) => prefs.set(&key, Some(&value)).await,
        None => prefs.set(&key, None).await,
    };

    match result {
        Ok(()) => Ok(api_success!()),
        Err(err) => {
            tracing::error!("preferences_set failed: {}", err);
            Ok(api_error!("workspace.preferences_set_failed"))
        }
    }
}
