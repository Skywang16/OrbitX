//! Shell Integration Tauri Commands
//!
//! 提供前端调用的Shell Integration相关命令

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tracing::error;

use super::{CommandInfo, PaneShellState, ShellType};
use crate::mux::{PaneId, TerminalMux};

/// 前端命令信息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendCommandInfo {
    pub id: u64,
    pub start_time: u64, // timestamp
    pub end_time: Option<u64>,
    pub exit_code: Option<i32>,
    pub status: String,
    pub command_line: Option<String>,
    pub working_directory: Option<String>,
    pub duration_ms: Option<u64>,
}

impl From<&CommandInfo> for FrontendCommandInfo {
    fn from(cmd: &CommandInfo) -> Self {
        use std::time::UNIX_EPOCH;
        let start_timestamp = cmd
            .start_time_wallclock
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let end_timestamp = cmd
            .end_time_wallclock
            .as_ref()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        let duration_ms = if cmd.is_finished() {
            Some(cmd.duration().as_millis() as u64)
        } else {
            None
        };

        Self {
            id: cmd.id,
            start_time: start_timestamp,
            end_time: end_timestamp,
            exit_code: cmd.exit_code,
            status: format!("{:?}", cmd.status),
            command_line: cmd.command_line.clone(),
            working_directory: cmd.working_directory.clone(),
            duration_ms,
        }
    }
}

/// 前端面板状态结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendPaneState {
    pub integration_enabled: bool,
    pub shell_type: Option<String>,
    pub current_working_directory: Option<String>,
    pub current_command: Option<FrontendCommandInfo>,
    pub command_history: Vec<FrontendCommandInfo>,
    pub window_title: Option<String>,
    pub last_activity: u64,
}

impl From<&PaneShellState> for FrontendPaneState {
    fn from(state: &PaneShellState) -> Self {
        use std::time::UNIX_EPOCH;

        let last_activity = state
            .last_activity
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            integration_enabled: matches!(
                state.integration_state,
                super::ShellIntegrationState::Enabled
            ),
            shell_type: state
                .shell_type
                .as_ref()
                .map(|t| t.display_name().to_string()),
            current_working_directory: state.current_working_directory.clone(),
            current_command: state
                .current_command
                .as_ref()
                .map(FrontendCommandInfo::from),
            command_history: state
                .command_history
                .iter()
                .map(FrontendCommandInfo::from)
                .collect(),
            window_title: state.window_title.clone(),
            last_activity,
        }
    }
}

/// 检查Shell Integration状态（简化版）
#[tauri::command]
pub async fn check_shell_integration_status(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> Result<bool, String> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    let status = mux.is_pane_integrated(pane_id);
    Ok(status)
}

/// 设置Shell Integration - 真正的脚本注入实现
#[tauri::command]
pub async fn setup_shell_integration(
    pane_id: u32,
    silent: Option<bool>,
    state: State<'_, Arc<TerminalMux>>,
) -> Result<(), String> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);
    let silent = silent.unwrap_or(true);

    if !mux.pane_exists(pane_id) {
        let err = format!("Pane {} does not exist", pane_id);
        error!("{}", err);
        return Err(err);
    }

    // 真正的Shell Integration设置
    mux.setup_pane_integration_with_script(pane_id, silent)
        .map_err(|e| {
            let err = format!("Failed to setup shell integration: {}", e);
            error!("{}", err);
            err
        })?;

    Ok(())
}

/// 获取pane的当前工作目录
#[tauri::command]
pub async fn get_pane_cwd(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> Result<Option<String>, String> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    let cwd = mux.get_pane_cwd(pane_id);
    Ok(cwd)
}

/// 更新pane的当前工作目录（用于静默模式）
#[tauri::command]
pub async fn update_pane_cwd(
    pane_id: u32,
    cwd: String,
    state: State<'_, Arc<TerminalMux>>,
) -> Result<(), String> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    mux.update_pane_cwd(pane_id, cwd);
    Ok(())
}

/// 获取面板的完整Shell状态
#[tauri::command]
pub async fn get_pane_shell_state(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> Result<Option<FrontendPaneState>, String> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    let shell_state = mux
        .get_pane_shell_state(pane_id)
        .map(|state| FrontendPaneState::from(&state));
    Ok(shell_state)
}

/// 设置面板的Shell类型
#[tauri::command]
pub async fn set_pane_shell_type(
    pane_id: u32,
    shell_type: String,
    state: State<'_, Arc<TerminalMux>>,
) -> Result<(), String> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    let shell_type = ShellType::from_program(&shell_type);
    mux.set_pane_shell_type(pane_id, shell_type);
    Ok(())
}

/// 生成Shell集成脚本
#[tauri::command]
pub async fn generate_shell_integration_script(
    shell_type: String,
    state: State<'_, Arc<TerminalMux>>,
) -> Result<String, String> {
    let mux = &*state;
    let shell_type = ShellType::from_program(&shell_type);

    if !shell_type.supports_integration() {
        return Err(format!(
            "Shell type {} does not support integration",
            shell_type.display_name()
        ));
    }

    mux.generate_shell_integration_script(&shell_type)
        .map_err(|e| format!("Failed to generate shell script: {}", e))
}

/// 生成Shell环境变量
#[tauri::command]
pub async fn generate_shell_env_vars(
    shell_type: String,
    state: State<'_, Arc<TerminalMux>>,
) -> Result<HashMap<String, String>, String> {
    let mux = &*state;
    let shell_type = ShellType::from_program(&shell_type);

    let env_vars = mux.generate_shell_env_vars(&shell_type);
    Ok(env_vars)
}

/// 启用面板Shell Integration
#[tauri::command]
pub async fn enable_pane_integration(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> Result<(), String> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    mux.enable_pane_integration(pane_id);
    Ok(())
}

/// 禁用面板Shell Integration
#[tauri::command]
pub async fn disable_pane_integration(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> Result<(), String> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    mux.disable_pane_integration(pane_id);
    Ok(())
}

/// 获取面板的当前命令信息
#[tauri::command]
pub async fn get_pane_current_command(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> Result<Option<FrontendCommandInfo>, String> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    let command = mux
        .get_pane_current_command(pane_id)
        .map(|cmd| FrontendCommandInfo::from(&cmd));
    Ok(command)
}

/// 获取面板的命令历史
#[tauri::command]
pub async fn get_pane_command_history(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> Result<Vec<FrontendCommandInfo>, String> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    let history = mux
        .get_pane_command_history(pane_id)
        .into_iter()
        .map(|cmd| FrontendCommandInfo::from(&cmd))
        .collect();
    Ok(history)
}

/// 检测Shell类型
#[tauri::command]
pub async fn detect_shell_type(shell_program: String) -> Result<String, String> {
    let shell_type = ShellType::from_program(&shell_program);
    Ok(shell_type.display_name().to_string())
}

/// 检查Shell是否支持集成
#[tauri::command]
pub async fn check_shell_integration_support(shell_program: String) -> Result<bool, String> {
    let shell_type = ShellType::from_program(&shell_program);
    Ok(shell_type.supports_integration())
}

