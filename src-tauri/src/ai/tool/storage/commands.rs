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
use tracing::{debug, error};

/// 获取配置数据
#[tauri::command]
pub async fn storage_get_config(
    section: String,
    config: State<'_, Arc<TomlConfigManager>>,
) -> TauriApiResult<Value> {
    debug!("存储命令: 获取配置节 {}", section);

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
            debug!("配置节 {} 获取成功", section);
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
    debug!("存储命令: 更新配置节 {}", section);

    if section.trim().is_empty() {
        return Ok(api_error!("common.invalid_params"));
    }

    match config.inner().update_section(&section, data).await {
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
    msgpack: State<'_, Arc<MessagePackManager>>,
) -> TauriApiResult<EmptyData> {
    debug!("保存会话状态: {} tabs", session_state.tabs.len());

    match msgpack.inner().save_state(&session_state).await {
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
    msgpack: State<'_, Arc<MessagePackManager>>,
) -> TauriApiResult<Option<SessionState>> {
    debug!("开始加载会话状态");

    match msgpack.inner().load_state().await {
        Ok(Some(session_state)) => {
            debug!("加载会话状态成功: {} tabs", session_state.tabs.len());
            Ok(api_success!(Some(session_state)))
        }
        Ok(None) => {
            debug!("没有找到保存的会话状态");
            Ok(api_success!(None))
        }
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
pub async fn storage_get_terminals_state() -> TauriApiResult<Vec<crate::storage::types::TerminalRuntimeState>> {
    use crate::mux::singleton::get_mux;
    use crate::storage::types::TerminalRuntimeState;

    debug!("🔍 查询终端运行时状态");

    let mux = get_mux();
    let pane_ids = mux.list_panes();

    let terminals: Vec<TerminalRuntimeState> = pane_ids
        .into_iter()
        .map(|pane_id| {
            let cwd = mux.shell_get_pane_cwd(pane_id).unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "~".to_string())
            });

            let shell_state = mux.get_pane_shell_state(pane_id);
            let shell_type = shell_state
                .as_ref()
                .and_then(|state| state.shell_type.as_ref().map(|t| format!("{:?}", t)));

            TerminalRuntimeState {
                id: pane_id.as_u32(),
                cwd,
                shell: shell_type,
            }
        })
        .collect();

    debug!("✅ 返回 {} 个终端状态", terminals.len());
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
