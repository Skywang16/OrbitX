/*!
 * 存储系统Tauri命令模块
 *
 * 提供统一的存储API命令，基于新的Repository架构实现
 * 包含配置管理、会话状态、数据查询等功能
 */

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

// =========================
// M3: Task Index/List Retrieval (no UI)
// =========================

#[tauri::command]
pub async fn task_get(
    task_id: String,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<Option<Value>> {
    debug!("获取任务详情(SQL): {}", task_id);
    match (|| -> AppResult<Option<Value>> {
        let repos = state.coordinator.repositories();
        let v = tauri::async_runtime::block_on(async { repos.tasks().get_task(&task_id).await })?;
        Ok(v)
    })() {
        Ok(v) => Ok(api_success!(v)),
        Err(e) => {
            error!("获取任务详情(SQL)失败: {}", e);
            Ok(api_error!("task.get_failed"))
        }
    }
}

#[tauri::command]
pub async fn task_list(
    status: Option<String>,
    parent_task_id: Option<String>,
    root_task_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    order_desc: Option<bool>,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<Vec<Value>> {
    debug!("列出任务(SQL)");
    match (|| -> AppResult<Vec<Value>> {
        let repos = state.coordinator.repositories();
        let mut filter = crate::storage::repositories::tasks::TaskListFilter::default();
        filter.status = status;
        filter.parent_task_id = parent_task_id;
        filter.root_task_id = root_task_id;
        if limit.is_some() {
            filter.limit = limit;
        }
        if offset.is_some() {
            filter.offset = offset;
        }
        if let Some(desc) = order_desc {
            filter.order_desc = desc;
        }
        let list = tauri::async_runtime::block_on(async { repos.tasks().list_tasks(filter).await })?;
        Ok(list)
    })() {
        Ok(list) => Ok(api_success!(list)),
        Err(e) => {
            error!("列出任务(SQL)失败: {}", e);
            Ok(api_error!("task.list_failed"))
        }
    }
}

#[tauri::command]
pub async fn task_delete(
    task_id: String,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<EmptyData> {
    debug!("删除任务(SQL): {}", task_id);
    match (|| -> AppResult<()> {
        let repos = state.coordinator.repositories();
        tauri::async_runtime::block_on(async { repos.tasks().delete_task(&task_id).await })?;
        Ok(())
    })() {
        Ok(()) => Ok(api_success!()),
        Err(e) => {
            error!("删除任务(SQL)失败: {}", e);
            Ok(api_error!("task.delete_failed"))
        }
    }
}

// =========================
// M2: Task Persistence APIs (Pure SQL via Repository)
// =========================

#[tauri::command]
pub async fn task_save_ui_messages(
    task_id: String,
    messages: Value,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<EmptyData> {
    debug!("保存任务 UI 消息(SQL): {}", task_id);
    match (|| -> AppResult<()> {
        // messages can be an array or { events: [...] }
        let events: Vec<Value> = if let Some(arr) = messages.as_array() {
            arr.clone()
        } else if let Some(ev) = messages.get("events").and_then(|v| v.as_array()) {
            ev.clone()
        } else {
            return Err(anyhow::anyhow!("无效的UI消息格式: 需要数组或{{events: []}}"));
        };
        let repos = state.coordinator.repositories();
        tauri::async_runtime::block_on(async {
            repos.tasks().replace_ui_events(&task_id, &events).await
        })?;
        Ok(())
    })() {
        Ok(()) => Ok(api_success!()),
        Err(e) => {
            error!("保存 UI 消息(SQL)失败: {}", e);
            Ok(api_error!("task.save_ui_failed"))
        }
    }
}

#[tauri::command]
pub async fn task_read_ui_messages(
    task_id: String,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<Value> {
    debug!("读取任务 UI 消息(SQL): {}", task_id);
    match (|| -> AppResult<Value> {
        let repos = state.coordinator.repositories();
        let events =
            tauri::async_runtime::block_on(async { repos.tasks().read_ui_events(&task_id).await })?;
        Ok(Value::Array(events))
    })() {
        Ok(v) => Ok(api_success!(v)),
        Err(e) => {
            error!("读取 UI 消息(SQL)失败: {}", e);
            Ok(api_error!("task.read_ui_failed"))
        }
    }
}

#[tauri::command]
pub async fn task_save_api_messages(
    task_id: String,
    messages: Value,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<EmptyData> {
    debug!("保存任务 API 消息(SQL): {}", task_id);
    match (|| -> AppResult<()> {
        let repos = state.coordinator.repositories();
        tauri::async_runtime::block_on(async {
            repos.tasks().save_api_messages(&task_id, &messages).await
        })?;
        Ok(())
    })() {
        Ok(()) => Ok(api_success!()),
        Err(e) => {
            error!("保存 API 消息(SQL)失败: {}", e);
            Ok(api_error!("task.save_api_failed"))
        }
    }
}

#[tauri::command]
pub async fn task_read_api_messages(
    task_id: String,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<Value> {
    debug!("读取任务 API 消息(SQL): {}", task_id);
    match (|| -> AppResult<Value> {
        let repos = state.coordinator.repositories();
        let v = tauri::async_runtime::block_on(async {
            repos.tasks().read_api_messages(&task_id).await
        })?;
        Ok(v)
    })() {
        Ok(v) => Ok(api_success!(v)),
        Err(e) => {
            error!("读取 API 消息(SQL)失败: {}", e);
            Ok(api_error!("task.read_api_failed"))
        }
    }
}

#[tauri::command]
pub async fn task_save_metadata(
    task_id: String,
    metadata: Value,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<EmptyData> {
    debug!("保存任务元数据(SQL): {}", task_id);
    match (|| -> AppResult<()> {
        let repos = state.coordinator.repositories();
        tauri::async_runtime::block_on(async {
            repos.tasks().save_metadata(&task_id, &metadata).await
        })?;
        // 同步写索引（name/status/parent/root 可选）
        let name = metadata.get("name").and_then(|v| v.as_str());
        let status = metadata.get("status").and_then(|v| v.as_str());
        let parent = metadata.get("parentTaskId").and_then(|v| v.as_str());
        let root = metadata.get("rootTaskId").and_then(|v| v.as_str());
        tauri::async_runtime::block_on(async {
            repos
                .tasks()
                .upsert_task_index(&task_id, name, status, parent, root, Some(&metadata))
                .await
        })?;
        Ok(())
    })() {
        Ok(()) => Ok(api_success!()),
        Err(e) => {
            error!("保存元数据(SQL)失败: {}", e);
            Ok(api_error!("task.save_metadata_failed"))
        }
    }
}

#[tauri::command]
pub async fn task_read_metadata(
    task_id: String,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<Value> {
    debug!("读取任务元数据(SQL): {}", task_id);
    match (|| -> AppResult<Value> {
        let repos = state.coordinator.repositories();
        let v =
            tauri::async_runtime::block_on(async { repos.tasks().read_metadata(&task_id).await })?;
        Ok(v)
    })() {
        Ok(v) => Ok(api_success!(v)),
        Err(e) => {
            error!("读取元数据(SQL)失败: {}", e);
            Ok(api_error!("task.read_metadata_failed"))
        }
    }
}

#[tauri::command]
pub async fn task_checkpoint_save(
    task_id: String,
    checkpoint: Value,
    name: Option<String>,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<String> {
    debug!("保存任务检查点(SQL): {}", task_id);
    match (|| -> AppResult<String> {
        let repos = state.coordinator.repositories();
        let saved = tauri::async_runtime::block_on(async {
            repos
                .tasks()
                .save_checkpoint(&task_id, name.as_deref(), &checkpoint)
                .await
        })?;
        Ok(saved)
    })() {
        Ok(name) => Ok(api_success!(name)),
        Err(e) => {
            error!("保存检查点(SQL)失败: {}", e);
            Ok(api_error!("task.save_checkpoint_failed"))
        }
    }
}

#[tauri::command]
pub async fn task_checkpoint_list(
    task_id: String,
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<Vec<String>> {
    debug!("列出任务检查点(SQL): {}", task_id);
    match (|| -> AppResult<Vec<String>> {
        let repos = state.coordinator.repositories();
        let list = tauri::async_runtime::block_on(async {
            repos.tasks().list_checkpoints(&task_id).await
        })?;
        Ok(list)
    })() {
        Ok(list) => Ok(api_success!(list)),
        Err(e) => {
            error!("列出检查点(SQL)失败: {}", e);
            Ok(api_error!("task.list_checkpoint_failed"))
        }
    }
}

#[tauri::command]
pub async fn task_purge_all(
    state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<EmptyData> {
    debug!("清理所有任务数据(SQL)");
    match (|| -> AppResult<()> {
        let repos = state.coordinator.repositories();
        tauri::async_runtime::block_on(async { repos.tasks().purge_all().await })?;
        Ok(())
    })() {
        Ok(()) => Ok(api_success!()),
        Err(e) => {
            error!("清理任务数据(SQL)失败: {}", e);
            Ok(api_error!("task.purge_failed"))
        }
    }
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
