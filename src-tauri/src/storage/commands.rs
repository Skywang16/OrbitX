/*!
 * 存储系统Tauri命令模块
 *
 * 提供统一的存储API命令，基于StorageCoordinator实现
 * 包含配置管理、会话状态、数据查询等功能
 */

use crate::storage::types::{CacheStats, DataQuery, SaveOptions, SessionState, StorageStats};
use crate::storage::{HealthCheckResult, StorageCoordinator};
use crate::utils::error::AppResult;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;
use tracing::debug;

/// 存储协调器状态管理
pub struct StorageCoordinatorState {
    pub coordinator: Arc<StorageCoordinator>,
}

impl StorageCoordinatorState {
    pub async fn new() -> AppResult<Self> {
        use crate::storage::{StorageCoordinatorOptions, StoragePaths};
        use std::env;

        // 获取应用数据目录
        let app_dir = if let Ok(dir) = env::var("TERMX_DATA_DIR") {
            std::path::PathBuf::from(dir)
        } else {
            // 使用默认的应用数据目录
            dirs::data_dir()
                .ok_or_else(|| anyhow::anyhow!("无法获取应用数据目录"))?
                .join("termx")
        };

        let paths = StoragePaths::new(app_dir)?;
        let options = StorageCoordinatorOptions::default();
        let coordinator = Arc::new(StorageCoordinator::new(paths, options).await?);

        Ok(Self { coordinator })
    }
}

/// 获取配置数据
#[tauri::command]
pub async fn storage_get_config(
    section: String,
    state: State<'_, StorageCoordinatorState>,
) -> Result<Value, String> {
    debug!("存储命令: 获取配置节 {}", section);

    state
        .coordinator
        .get_config(&section)
        .await
        .map_err(|e| e.to_string())
}

/// 更新配置数据
#[tauri::command]
pub async fn storage_update_config(
    section: String,
    data: Value,
    state: State<'_, StorageCoordinatorState>,
) -> Result<(), String> {
    debug!("存储命令: 更新配置节 {}", section);

    state
        .coordinator
        .update_config(&section, data)
        .await
        .map_err(|e| e.to_string())
}

/// 保存会话状态
#[tauri::command]
pub async fn storage_save_session_state(
    session_state: SessionState,
    state: State<'_, StorageCoordinatorState>,
) -> Result<(), String> {
    debug!("存储命令: 保存会话状态");

    state
        .coordinator
        .save_session_state(&session_state)
        .await
        .map_err(|e| e.to_string())
}

/// 加载会话状态
#[tauri::command]
pub async fn storage_load_session_state(
    state: State<'_, StorageCoordinatorState>,
) -> Result<Option<SessionState>, String> {
    debug!("存储命令: 加载会话状态");

    state
        .coordinator
        .load_session_state()
        .await
        .map_err(|e| e.to_string())
}

/// 查询数据
#[tauri::command]
pub async fn storage_query_data(
    query: DataQuery,
    state: State<'_, StorageCoordinatorState>,
) -> Result<Vec<Value>, String> {
    debug!("存储命令: 查询数据 {}", query.query);

    state
        .coordinator
        .query_data(&query)
        .await
        .map_err(|e| e.to_string())
}

/// 保存数据
#[tauri::command]
pub async fn storage_save_data(
    data: Value,
    options: SaveOptions,
    state: State<'_, StorageCoordinatorState>,
) -> Result<(), String> {
    debug!("存储命令: 保存数据到表 {:?}", options.table);

    state
        .coordinator
        .save_data(&data, &options)
        .await
        .map_err(|e| e.to_string())
}

/// 健康检查
#[tauri::command]
pub async fn storage_health_check(
    state: State<'_, StorageCoordinatorState>,
) -> Result<HealthCheckResult, String> {
    debug!("存储命令: 健康检查");

    state
        .coordinator
        .health_check()
        .await
        .map_err(|e| e.to_string())
}

/// 获取缓存统计信息
#[tauri::command]
pub async fn storage_get_cache_stats(
    state: State<'_, StorageCoordinatorState>,
) -> Result<CacheStats, String> {
    debug!("存储命令: 获取缓存统计信息");

    state
        .coordinator
        .get_cache_stats()
        .await
        .map_err(|e| e.to_string())
}

/// 获取存储统计信息
#[tauri::command]
pub async fn storage_get_storage_stats(
    state: State<'_, StorageCoordinatorState>,
) -> Result<StorageStats, String> {
    debug!("存储命令: 获取存储统计信息");

    state
        .coordinator
        .get_storage_stats()
        .await
        .map_err(|e| e.to_string())
}

/// 预加载缓存
#[tauri::command]
pub async fn storage_preload_cache(
    state: State<'_, StorageCoordinatorState>,
) -> Result<(), String> {
    debug!("存储命令: 预加载缓存");

    state
        .coordinator
        .preload_cache()
        .await
        .map_err(|e| e.to_string())
}

/// 清空缓存
#[tauri::command]
pub async fn storage_clear_cache(state: State<'_, StorageCoordinatorState>) -> Result<(), String> {
    debug!("存储命令: 清空缓存");

    state
        .coordinator
        .clear_cache()
        .await
        .map_err(|e| e.to_string())
}
