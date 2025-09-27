/*!
 * 存储系统Tauri命令模块
 *
 * 提供统一的存储API命令，基于新的Repository架构实现
 * 包含配置管理、会话状态、数据查询等功能
 *
 * NOTE: Task-related commands removed during refactor
 */

use crate::storage::repositories::tasks::{EkoContext, UITask};
use crate::storage::types::SessionState;
use crate::storage::StorageCoordinator;
use crate::utils::error::AppResult;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use anyhow::Context;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;
use tracing::{debug, error};

/// 存储协调器状态管理
pub struct StorageCoordinatorState {
    pub coordinator: Arc<StorageCoordinator>,
}

impl StorageCoordinatorState {
    pub async fn new(config_manager: Arc<crate::config::TomlConfigManager>) -> AppResult<Self> {
        use crate::storage::{StorageCoordinatorOptions, StoragePaths};
        use std::env;
        use tracing::debug;

        let app_dir = if let Ok(dir) = env::var("OrbitX_DATA_DIR") {
            debug!("使用环境变量指定的数据目录: {}", dir);
            std::path::PathBuf::from(dir)
        } else {
            // 使用默认的应用数据目录
            let data_dir = dirs::data_dir().ok_or_else(|| {
                anyhow::anyhow!(
                    "无法获取系统应用数据目录，请检查系统配置或设置 OrbitX_DATA_DIR 环境变量"
                )
            })?;
            let app_dir = data_dir.join("OrbitX");
            debug!("使用默认应用数据目录: {}", app_dir.display());
            app_dir
        };

        debug!("初始化存储路径，应用目录: {}", app_dir.display());
        let paths =
            StoragePaths::new(app_dir).with_context(|| "存储路径初始化失败，请检查目录权限")?;

        let options = StorageCoordinatorOptions::default();
        let coordinator = Arc::new(
            StorageCoordinator::new(paths, options, config_manager)
                .await
                .with_context(|| "存储协调器创建失败")?,
        );

        debug!("存储协调器状态初始化成功");
        Ok(Self { coordinator })
    }
}

/// 获取配置数据
#[tauri::command]
pub async fn storage_get_config(
    section: String,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<Value> {
    debug!("存储命令: 获取配置节 {}", section);

    if section.trim().is_empty() {
        return Ok(api_error!("common.invalid_params"));
    }

    match state.coordinator.config_get(&section).await {
        Ok(config) => {
            debug!("配置节 {} 获取成功", section);
            Ok(api_success!(config))
        }
        Err(e) => {
            error!("配置节 {} 获取失败: {}", section, e);
            Ok(api_error!("storage.get_config_failed"))
        }
    }
}

/// 更新配置数据
#[tauri::command]
pub async fn storage_update_config(
    section: String,
    data: Value,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<EmptyData> {
    debug!("存储命令: 更新配置节 {}", section);

    if section.trim().is_empty() {
        return Ok(api_error!("common.invalid_params"));
    }

    match state.coordinator.config_update(&section, data).await {
        Ok(()) => {
            debug!("配置节 {} 更新成功", section);
            Ok(api_success!())
        }
        Err(e) => {
            error!("配置节 {} 更新失败: {}", section, e);
            Ok(api_error!("storage.update_config_failed"))
        }
    }
}

/// 保存会话状态
#[tauri::command]
pub async fn storage_save_session_state(
    session_state: SessionState,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<EmptyData> {
    debug!("📊 会话状态统计:");
    debug!("  - 终端数量: {}", session_state.terminals.len());
    debug!("  - 版本: {}", session_state.version);
    debug!("  - AI可见: {}", session_state.ai.visible);

    match state.coordinator.save_session_state(&session_state).await {
        Ok(()) => {
            debug!("✅ 会话状态保存成功");
            Ok(api_success!())
        }
        Err(_e) => {
            error!("❌ 会话状态保存失败");
            Ok(api_error!("storage.save_session_failed"))
        }
    }
}

/// 加载会话状态
#[tauri::command]
pub async fn storage_load_session_state(
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<Option<SessionState>> {
    debug!("🔍 开始加载会话状态");

    match state.coordinator.load_session_state().await {
        Ok(Some(session_state)) => {
            debug!("  - 终端数量: {}", session_state.terminals.len());
            debug!("  - 版本: {}", session_state.version);
            debug!("  - AI可见: {}", session_state.ai.visible);

            Ok(api_success!(Some(session_state)))
        }
        Ok(None) => {
            debug!("ℹ️ 没有找到保存的会话状态");
            Ok(api_success!(None))
        }
        Err(_e) => {
            error!("❌ 会话状态加载失败");
            Ok(api_error!("storage.load_session_failed"))
        }
    }
}
// ============================================================================
// 双轨制任务系统 API - 按照 task-system-architecture-final.md 设计
// ============================================================================

// ---- 原始上下文轨 API ----

/// 更新或插入任务状态到上下文轨
#[tauri::command]
pub async fn eko_ctx_upsert_state(
    task_id: String,
    context: String,
    conversation_id: Option<i64>,
    node_id: Option<String>,
    status: Option<String>,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<i64> {
    let eko_context = EkoContext {
        id: None,
        task_id: task_id.clone(),
        conversation_id: conversation_id.unwrap_or(1), // 默认会话ID
        kind: crate::storage::repositories::tasks::EkoContextKind::State,
        name: None,
        node_id,
        status: status
            .and_then(|s| crate::storage::repositories::tasks::EkoStatus::from_str(&s).ok()),
        payload_json: context,
        created_at: chrono::Utc::now(),
    };

    match state
        .coordinator
        .repositories()
        .tasks()
        .save_eko_context(&eko_context)
        .await
    {
        Ok(id) => Ok(api_success!(id)),
        Err(e) => {
            error!("Eko状态保存失败: {}", e);
            Ok(api_error!("eko_ctx.upsert_state_failed"))
        }
    }
}

/// 追加事件到上下文轨
#[tauri::command]
pub async fn eko_ctx_append_event(
    task_id: String,
    event: String,
    conversation_id: i64,
    node_id: Option<String>,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<i64> {
    let eko_context = EkoContext {
        id: None,
        task_id: task_id.clone(),
        conversation_id,
        kind: crate::storage::repositories::tasks::EkoContextKind::Event,
        name: None,
        node_id,
        status: None,
        payload_json: event,
        created_at: chrono::Utc::now(),
    };

    match state
        .coordinator
        .repositories()
        .tasks()
        .save_eko_context(&eko_context)
        .await
    {
        Ok(id) => Ok(api_success!(id)),
        Err(_) => Ok(api_error!("eko_ctx.append_event_failed")),
    }
}

/// 保存快照到上下文轨
#[tauri::command]
pub async fn eko_ctx_snapshot_save(
    task_id: String,
    name: Option<String>,
    snapshot: String,
    conversation_id: Option<i64>,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<i64> {
    let eko_context = EkoContext {
        id: None,
        task_id: task_id.clone(),
        conversation_id: conversation_id.unwrap_or(1), // 默认会话ID为1
        kind: crate::storage::repositories::tasks::EkoContextKind::Snapshot,
        name,
        node_id: None,
        status: None,
        payload_json: snapshot,
        created_at: chrono::Utc::now(),
    };

    match state
        .coordinator
        .repositories()
        .tasks()
        .save_eko_context(&eko_context)
        .await
    {
        Ok(id) => {
            debug!("Eko快照保存成功，ID: {}", id);
            Ok(api_success!(id))
        }
        Err(e) => {
            error!("Eko快照保存失败: {}", e);
            Ok(api_error!("eko_ctx.snapshot_save_failed"))
        }
    }
}

/// 获取任务的最新状态
#[tauri::command]
pub async fn eko_ctx_get_state(
    task_id: String,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<Option<EkoContext>> {
    match state
        .coordinator
        .repositories()
        .tasks()
        .get_latest_eko_state(&task_id)
        .await
    {
        Ok(context) => {
            debug!("Eko状态获取成功");
            Ok(api_success!(context))
        }
        Err(e) => {
            error!("Eko状态获取失败: {}", e);
            Ok(api_error!("eko_ctx.get_state_failed"))
        }
    }
}

/// 重建任务执行上下文（用于恢复/重跑）
#[tauri::command]
pub async fn eko_ctx_rebuild(
    task_id: String,
    from_snapshot_name: Option<String>,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<String> {
    match state
        .coordinator
        .repositories()
        .tasks()
        .rebuild_eko_context(&task_id, from_snapshot_name.as_deref())
        .await
    {
        Ok(context) => {
            debug!("Eko上下文重建成功");
            Ok(api_success!(context))
        }
        Err(e) => {
            error!("Eko上下文重建失败: {}", e);
            Ok(api_error!("eko_ctx.rebuild_failed"))
        }
    }
}

/// 构建Prompt（统一入口）
#[tauri::command]
pub async fn eko_ctx_build_prompt(
    task_id: String,
    user_input: String,
    pane_id: Option<String>,
    tag_context: Option<String>,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<String> {
    match state
        .coordinator
        .repositories()
        .tasks()
        .build_prompt(
            &task_id,
            &user_input,
            pane_id.as_deref(),
            tag_context.as_deref(),
        )
        .await
    {
        Ok(prompt) => {
            debug!("Prompt构建成功");
            Ok(api_success!(prompt))
        }
        Err(e) => {
            error!("Prompt构建失败: {}", e);
            Ok(api_error!("eko_ctx.build_prompt_failed"))
        }
    }
}

// ---- UI 轨 API ----

/// 创建或更新UI任务
#[tauri::command]
pub async fn ui_task_upsert(
    record: UITask,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<i64> {
    debug!("UI任务: 创建/更新 task_id={}", record.task_id);

    match state
        .coordinator
        .repositories()
        .tasks()
        .upsert_ui_task(&record)
        .await
    {
        Ok(ui_id) => {
            debug!("UI任务操作成功，ID: {}", ui_id);
            Ok(api_success!(ui_id))
        }
        Err(e) => {
            error!("UI任务操作失败: {}", e);
            Ok(api_error!("ui_task.upsert_failed"))
        }
    }
}

/// 批量创建或更新UI任务
#[tauri::command]
pub async fn ui_task_bulk_upsert(
    records: Vec<UITask>,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<Vec<i64>> {
    debug!("UI任务: 批量操作 {} 个任务", records.len());

    let mut results = Vec::new();
    for record in records {
        match state
            .coordinator
            .repositories()
            .tasks()
            .upsert_ui_task(&record)
            .await
        {
            Ok(ui_id) => results.push(ui_id),
            Err(e) => {
                error!("批量UI任务操作失败: {}", e);
                return Ok(api_error!("ui_task.bulk_upsert_failed"));
            }
        }
    }

    debug!("批量UI任务操作成功");
    Ok(api_success!(results))
}

/// 获取会话的UI任务列表
#[tauri::command]
pub async fn ui_task_list(
    conversation_id: i64,
    _filters: Option<String>,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<Vec<UITask>> {
    match state
        .coordinator
        .repositories()
        .tasks()
        .get_ui_tasks(conversation_id)
        .await
    {
        Ok(tasks) => Ok(api_success!(tasks)),
        Err(e) => {
            error!("UI任务列表获取失败: {}", e);
            Ok(api_error!("ui_task.list_failed"))
        }
    }
}

/// 删除UI任务
#[tauri::command]
pub async fn ui_task_delete(
    ui_id: i64,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<EmptyData> {
    debug!("UI任务: 删除 ui_id={}", ui_id);

    match state
        .coordinator
        .repositories()
        .tasks()
        .delete_ui_task(ui_id)
        .await
    {
        Ok(()) => {
            debug!("UI任务删除成功");
            Ok(api_success!())
        }
        Err(e) => {
            error!("UI任务删除失败: {}", e);
            Ok(api_error!("ui_task.delete_failed"))
        }
    }
}
