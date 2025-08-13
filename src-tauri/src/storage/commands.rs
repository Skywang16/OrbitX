/*!
 * 存储系统Tauri命令模块
 *
 * 提供统一的存储API命令，基于StorageCoordinator实现
 * 包含配置管理、会话状态、数据查询等功能
 */

use crate::storage::types::{DataQuery, SaveOptions, SessionState, StorageStats};
use crate::storage::{HealthCheckResult, StorageCoordinator};
use crate::utils::error::{serialize_to_json, AppResult, ToTauriResult};
use anyhow::Context;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;
use tracing::{debug, error, info};

/// 存储协调器状态管理
pub struct StorageCoordinatorState {
    pub coordinator: Arc<StorageCoordinator>,
}

impl StorageCoordinatorState {
    pub async fn new(config_manager: Arc<crate::config::TomlConfigManager>) -> AppResult<Self> {
        use crate::storage::{StorageCoordinatorOptions, StoragePaths};
        use std::env;
        use tracing::{debug, info};

        // 获取应用数据目录
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

        info!("初始化存储路径，应用目录: {}", app_dir.display());
        let paths =
            StoragePaths::new(app_dir).with_context(|| "存储路径初始化失败，请检查目录权限")?;

        let options = StorageCoordinatorOptions::default();
        let coordinator = Arc::new(
            StorageCoordinator::new(paths, options, config_manager)
                .await
                .with_context(|| "存储协调器创建失败")?,
        );

        info!("存储协调器状态初始化成功");
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

    state.coordinator.get_config(&section).await.to_tauri()
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
        .to_tauri()
}

/// 保存会话状态
#[tauri::command]
pub async fn storage_save_session_state(
    session_state: SessionState,
    state: State<'_, StorageCoordinatorState>,
) -> Result<(), String> {
    info!("🔄 开始保存会话状态");
    info!("📊 会话状态统计:");
    info!(
        "  - 终端会话数量: {}",
        session_state.terminal_sessions.len()
    );
    info!("  - 标签页数量: {}", session_state.tabs.len());
    info!("  - 版本: {}", session_state.version);

    // 打印终端会话详情
    for (id, session) in &session_state.terminal_sessions {
        info!(
            "  📱 终端会话 {}: title='{}', is_active={}, working_dir='{}'",
            id, session.title, session.is_active, session.working_directory
        );
    }

    // 打印标签页详情
    for tab in &session_state.tabs {
        info!(
            "  📋 标签页 {}: title='{}', is_active={}",
            tab.id, tab.title, tab.is_active
        );
    }

    match state.coordinator.save_session_state(&session_state).await {
        Ok(()) => {
            info!("✅ 会话状态保存成功");
            Ok(())
        }
        Err(e) => {
            error!("❌ 会话状态保存失败: {}", e);
            Err(e.to_string())
        }
    }
}

/// 加载会话状态
#[tauri::command]
pub async fn storage_load_session_state(
    state: State<'_, StorageCoordinatorState>,
) -> Result<Option<SessionState>, String> {
    info!("🔍 开始加载会话状态");

    match state.coordinator.load_session_state().await {
        Ok(Some(session_state)) => {
            info!("✅ 会话状态加载成功");
            info!("📊 加载的会话状态统计:");
            info!(
                "  - 终端会话数量: {}",
                session_state.terminal_sessions.len()
            );
            info!("  - 标签页数量: {}", session_state.tabs.len());
            info!("  - 版本: {}", session_state.version);

            // 打印终端会话详情
            for (id, session) in &session_state.terminal_sessions {
                info!(
                    "  📱 终端会话 {}: title='{}', is_active={}, working_dir='{}'",
                    id, session.title, session.is_active, session.working_directory
                );
            }

            // 打印标签页详情
            for tab in &session_state.tabs {
                info!(
                    "  📋 标签页 {}: title='{}', is_active={}",
                    tab.id, tab.title, tab.is_active
                );
            }

            Ok(Some(session_state))
        }
        Ok(None) => {
            info!("ℹ️ 没有找到保存的会话状态");
            Ok(None)
        }
        Err(e) => {
            error!("❌ 会话状态加载失败: {}", e);
            Err(e.to_string())
        }
    }
}

/// 查询数据
#[tauri::command]
pub async fn storage_query_data(
    query: DataQuery,
    state: State<'_, StorageCoordinatorState>,
) -> Result<Vec<Value>, String> {
    debug!("存储命令: 查询数据 {}", query.query);

    state.coordinator.query_data(&query).await.to_tauri()
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
        .to_tauri()
}

/// 健康检查
#[tauri::command]
pub async fn storage_health_check(
    state: State<'_, StorageCoordinatorState>,
) -> Result<HealthCheckResult, String> {
    debug!("存储命令: 健康检查");

    state.coordinator.health_check().await.to_tauri()
}

/// 获取缓存统计信息
#[tauri::command]
pub async fn storage_get_cache_stats(
    state: State<'_, StorageCoordinatorState>,
) -> Result<String, String> {
    debug!("存储命令: 获取缓存统计信息");

    let stats = state.coordinator.get_cache_stats().await.to_tauri()?;
    serialize_to_json(&stats, "缓存统计")
}

/// 获取存储统计信息
#[tauri::command]
pub async fn storage_get_storage_stats(
    state: State<'_, StorageCoordinatorState>,
) -> Result<StorageStats, String> {
    debug!("存储命令: 获取存储统计信息");

    state.coordinator.get_storage_stats().await.to_tauri()
}

/// 预加载缓存
#[tauri::command]
pub async fn storage_preload_cache(
    state: State<'_, StorageCoordinatorState>,
) -> Result<(), String> {
    debug!("存储命令: 预加载缓存");

    state.coordinator.preload_cache().await.to_tauri()
}

/// 清空缓存
#[tauri::command]
pub async fn storage_clear_cache(state: State<'_, StorageCoordinatorState>) -> Result<(), String> {
    debug!("存储命令: 清空缓存");

    state.coordinator.clear_cache().await.to_tauri()
}
