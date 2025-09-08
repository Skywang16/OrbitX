/*!
 * 存储系统Tauri命令模块
 *
 * 提供统一的存储API命令，基于新的Repository架构实现
 * 包含配置管理、会话状态、数据查询等功能
 */

use crate::storage::types::SessionState;
use crate::storage::StorageCoordinator;
use crate::utils::error::{AppResult, ToTauriResult};
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
