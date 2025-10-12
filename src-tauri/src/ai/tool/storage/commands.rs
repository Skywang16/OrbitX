/*!
 * 存储系统 Tauri 命令模块
 *
 * 仅保留通用配置与会话状态读写接口，
 * 任务相关命令已在 Agent 持久层中实现。
 */

use crate::storage::error::StorageCoordinatorError;
use crate::storage::types::SessionState;
use crate::storage::StorageCoordinator;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use serde_json::Value;
use std::sync::Arc;
use tauri::State;
use tracing::{debug, error};

/// 存储协调器状态管理
pub struct StorageCoordinatorState {
    pub coordinator: Arc<StorageCoordinator>,
}

impl StorageCoordinatorState {
    pub async fn new(
        config_manager: Arc<crate::config::TomlConfigManager>,
    ) -> Result<Self, StorageCoordinatorError> {
        use crate::storage::{StorageCoordinatorOptions, StoragePaths};
        use std::env;

        let app_dir = if let Ok(dir) = env::var("OrbitX_DATA_DIR") {
            tracing::debug!("使用环境变量指定的数据目录: {}", dir);
            std::path::PathBuf::from(dir)
        } else {
            let data_dir = dirs::data_dir().ok_or_else(|| {
                StorageCoordinatorError::Internal(
                    "无法获取系统应用数据目录，请检查系统配置或设置 OrbitX_DATA_DIR 环境变量"
                        .to_string(),
                )
            })?;
            let app_dir = data_dir.join("OrbitX");
            tracing::debug!("使用默认应用数据目录: {}", app_dir.display());
            app_dir
        };

        tracing::debug!("初始化存储路径，应用目录: {}", app_dir.display());
        let paths = StoragePaths::new(app_dir)?;

        let coordinator = Arc::new(
            StorageCoordinator::new(paths, StorageCoordinatorOptions::default(), config_manager)
                .await?,
        );

        tracing::debug!("存储协调器状态初始化成功");
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
    debug!("  - Tabs数量: {}", session_state.tabs.len());
    debug!("  - 版本: {}", session_state.version);
    debug!("  - AI可见: {}", session_state.ai.visible);

    match state.coordinator.save_session_state(&session_state).await {
        Ok(()) => {
            debug!("✅ 会话状态保存成功");
            Ok(api_success!())
        }
        Err(_) => {
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
            debug!("  - Tabs数量: {}", session_state.tabs.len());
            debug!("  - 版本: {}", session_state.version);
            debug!("  - AI可见: {}", session_state.ai.visible);

            Ok(api_success!(Some(session_state)))
        }
        Ok(None) => {
            debug!("ℹ️ 没有找到保存的会话状态");
            Ok(api_success!(None))
        }
        Err(_) => {
            error!("❌ 会话状态加载失败");
            Ok(api_error!("storage.load_session_failed"))
        }
    }
}

/// 从后端获取所有终端的运行时状态（包括实时 CWD）
///
/// 设计说明：
/// - 实时查询 ShellIntegration 获取当前 CWD
/// - 不依赖持久化数据，确保数据准确性
/// - 用于应用启动、会话恢复、前端同步等场景
#[tauri::command]
pub async fn storage_get_terminals_state(
    _state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<Vec<crate::storage::types::TerminalRuntimeState>> {
    use crate::mux::singleton::get_mux;
    use crate::storage::types::TerminalRuntimeState;

    debug!("🔍 查询所有终端的实时运行状态");

    let mux = get_mux();
    let pane_ids = mux.list_panes();

    let mut terminals = Vec::new();
    for pane_id in pane_ids {
        // 从 ShellIntegration 获取实时 CWD
        let cwd = mux.shell_get_pane_cwd(pane_id).unwrap_or_else(|| {
            // 回退：如果 Shell Integration 还未初始化，使用 home 目录
            dirs::home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "~".to_string())
        });

        let shell_state = mux.get_pane_shell_state(pane_id);
        let shell_type = shell_state
            .as_ref()
            .and_then(|state| state.shell_type.as_ref().map(|t| format!("{:?}", t)));

        let title = cwd
            .trim_end_matches('/')
            .split('/')
            .last()
            .unwrap_or("~")
            .to_string();

        terminals.push(TerminalRuntimeState {
            id: pane_id.as_u32(),
            title,
            cwd,
            active: false,
            shell: shell_type,
        });
    }

    debug!(
        "✅ 查询到 {} 个终端，CWD 数据来源：ShellIntegration",
        terminals.len()
    );
    Ok(api_success!(terminals))
}

/// 获取指定终端的当前工作目录
///
/// 设计说明：
/// - 直接从 ShellIntegration 查询实时 CWD
/// - 供 Agent 工具、前端组件等需要单个终端 CWD 的场景使用
#[tauri::command]
pub async fn storage_get_terminal_cwd(
    pane_id: u32,
    _state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<String> {
    use crate::mux::singleton::get_mux;
    use crate::mux::PaneId;

    debug!("🔍 查询终端 {} 的当前工作目录", pane_id);

    let mux = get_mux();
    let pane_id = PaneId::new(pane_id);

    // 检查 pane 是否存在
    if !mux.pane_exists(pane_id) {
        error!("❌ 终端 {} 不存在", pane_id.as_u32());
        return Ok(api_error!("terminal.pane_not_found"));
    }

    // 从 ShellIntegration 获取实时 CWD
    let cwd = mux.shell_get_pane_cwd(pane_id).unwrap_or_else(|| {
        debug!(
            "⚠️ 终端 {} 的 Shell Integration 尚未初始化，返回 home 目录",
            pane_id.as_u32()
        );
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "~".to_string())
    });

    debug!("✅ 终端 {} 的 CWD: {}", pane_id.as_u32(), cwd);
    Ok(api_success!(cwd))
}
