/*!
 * 存储系统 Tauri 命令模块 - 直接使用管理器
 */

use crate::config::TomlConfigManager;
use crate::storage::messagepack::MessagePackManager;
use crate::storage::types::SessionState;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use serde_json::Value;
use std::sync::Arc;
use tauri::State;
use tracing::error;

/// 获取配置数据
#[tauri::command]
pub async fn storage_get_config(
    section: String,
    config: State<'_, Arc<TomlConfigManager>>,
) -> TauriApiResult<Value> {
    if section.trim().is_empty() {
        return Ok(api_error!("common.invalid_params"));
    }

    match config.inner().config_get().await {
        Ok(app_config) => {
            // 从配置中提取section
            let value = serde_json::to_value(&app_config)
                .ok()
                .and_then(|v| v.get(&section).cloned())
                .unwrap_or(Value::Null);
            Ok(api_success!(value))
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
    config: State<'_, Arc<TomlConfigManager>>,
) -> TauriApiResult<EmptyData> {
    if section.trim().is_empty() {
        return Ok(api_error!("common.invalid_params"));
    }

    match config.inner().update_section(&section, data).await {
        Ok(()) => Ok(api_success!()),
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
    msgpack: State<'_, Arc<MessagePackManager>>,
) -> TauriApiResult<EmptyData> {
    match msgpack.inner().save_state(&session_state).await {
        Ok(()) => Ok(api_success!()),
        Err(_) => {
            error!("❌ 会话状态保存失败");
            Ok(api_error!("storage.save_session_failed"))
        }
    }
}

/// 加载会话状态
#[tauri::command]
pub async fn storage_load_session_state(
    msgpack: State<'_, Arc<MessagePackManager>>,
) -> TauriApiResult<Option<SessionState>> {
    match msgpack.inner().load_state().await {
        Ok(Some(session_state)) => Ok(api_success!(Some(session_state))),
        Ok(None) => Ok(api_success!(None)),
        Err(_) => {
            error!("会话状态加载失败");
            Ok(api_error!("storage.load_session_failed"))
        }
    }
}

/// 从后端获取所有终端的运行时状态（包括实时 CWD）
///
/// 设计说明：
/// - 直接从 Mux 查询当前运行时状态，Mux 是单一数据源
/// - ShellIntegration 状态恢复应该在应用启动时完成，不在此处理
#[tauri::command]
pub async fn storage_get_terminals_state(
) -> TauriApiResult<Vec<crate::storage::types::TerminalRuntimeState>> {
    use crate::mux::singleton::get_mux;
    use crate::storage::types::TerminalRuntimeState;

    let mux = get_mux();
    let pane_ids = mux.list_panes();

    let terminals: Vec<TerminalRuntimeState> = pane_ids
        .into_iter()
        .filter_map(|pane_id| {
            let pane = mux.get_pane(pane_id)?;

            let cwd = mux.shell_get_pane_cwd(pane_id).unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "~".to_string())
            });

            // 直接从 Pane 读取创建时的 shell 信息，使用 displayName
            let shell = pane.shell_info().display_name.clone();

            Some(TerminalRuntimeState {
                id: pane_id.as_u32(),
                cwd,
                shell,
            })
        })
        .collect();

    Ok(api_success!(terminals))
}

/// 获取指定终端的当前工作目录
///
/// 设计说明：
/// - 直接从 ShellIntegration 查询实时 CWD
/// - 供 Agent 工具、前端组件等需要单个终端 CWD 的场景使用
#[tauri::command]
pub async fn storage_get_terminal_cwd(pane_id: u32) -> TauriApiResult<String> {
    use crate::mux::singleton::get_mux;
    use crate::mux::PaneId;

    let mux = get_mux();
    let pane_id = PaneId::new(pane_id);

    // 检查 pane 是否存在
    if !mux.pane_exists(pane_id) {
        error!("❌ 终端 {} 不存在", pane_id.as_u32());
        return Ok(api_error!("terminal.pane_not_found"));
    }

    // 从 ShellIntegration 获取实时 CWD
    let cwd = mux.shell_get_pane_cwd(pane_id).unwrap_or_else(|| {
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "~".to_string())
    });

    Ok(api_success!(cwd))
}
