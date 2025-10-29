//! Shell 集成命令

use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;
use tauri::State;
use tracing::{debug, error};

use super::{CommandInfo, PaneShellState, ShellType};
use crate::mux::{PaneId, TerminalMux};

/// 使用shell-words解析命令行 - 零开销,不重复造轮子
fn parse_command_line(command: &str) -> Result<Vec<String>, String> {
    shell_words::split(command).map_err(|_| "Invalid shell syntax".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendCommandInfo {
    pub id: u64,
    pub start_time: u64, // 时间戳
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendPaneState {
    pub integration_enabled: bool,
    pub shell_type: Option<String>,
    pub current_working_directory: Option<String>,
    pub current_command: Option<FrontendCommandInfo>,
    pub command_history: Vec<FrontendCommandInfo>,
    pub window_title: Option<String>,
    pub last_activity: u64,
    pub node_version: Option<String>,
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
                .map(|cmd| FrontendCommandInfo::from(&**cmd)),
            command_history: state
                .command_history
                .iter()
                .map(|cmd| FrontendCommandInfo::from(&**cmd))
                .collect(),
            window_title: state.window_title.clone(),
            last_activity,
            node_version: state.node_version.clone(),
        }
    }
}

#[tauri::command]
pub async fn shell_check_integration_status(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriApiResult<bool> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Ok(api_error!("shell.pane_not_exist"));
    }

    let status = mux.is_pane_integrated(pane_id);
    Ok(api_success!(status))
}

#[tauri::command]
pub async fn shell_setup_integration(
    pane_id: u32,
    silent: Option<bool>,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriApiResult<EmptyData> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);
    let silent = silent.unwrap_or(true);

    if !mux.pane_exists(pane_id) {
        error!("Pane {} does not exist", pane_id);
        return Ok(api_error!("shell.pane_not_exist"));
    }

    // 真正的Shell Integration设置
    match mux.setup_pane_integration_with_script(pane_id, silent) {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("shell.setup_integration_failed")),
    }
}

#[tauri::command]
pub async fn shell_update_pane_cwd(
    pane_id: u32,
    cwd: String,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriApiResult<EmptyData> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Ok(api_error!("shell.pane_not_exist"));
    }

    mux.shell_update_pane_cwd(pane_id, cwd);
    Ok(api_success!())
}

#[tauri::command]
pub async fn get_pane_shell_state(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriApiResult<Option<FrontendPaneState>> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Ok(api_error!("shell.pane_not_exist"));
    }

    let shell_state = mux
        .get_pane_shell_state(pane_id)
        .map(|state| FrontendPaneState::from(&state));
    Ok(api_success!(shell_state))
}

#[tauri::command]
pub async fn set_pane_shell_type(
    pane_id: u32,
    shell_type: String,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriApiResult<EmptyData> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Ok(api_error!("shell.pane_not_exist"));
    }

    let shell_type = ShellType::from_program(&shell_type);
    mux.set_pane_shell_type(pane_id, shell_type);
    Ok(api_success!())
}

#[tauri::command]
pub async fn generate_shell_integration_script(
    shell_type: String,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriApiResult<String> {
    let mux = &*state;
    let shell_type = ShellType::from_program(&shell_type);

    if !shell_type.supports_integration() {
        return Ok(api_error!("shell.shell_not_supported"));
    }

    match mux.generate_shell_integration_script(&shell_type) {
        Ok(script) => Ok(api_success!(script)),
        Err(_) => Ok(api_error!("shell.generate_script_failed")),
    }
}

#[tauri::command]
pub async fn generate_shell_env_vars(
    shell_type: String,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriApiResult<HashMap<String, String>> {
    let mux = &*state;
    let shell_type = ShellType::from_program(&shell_type);

    let env_vars = mux.generate_shell_env_vars(&shell_type);
    Ok(api_success!(env_vars))
}

#[tauri::command]
pub async fn enable_pane_integration(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriApiResult<EmptyData> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Ok(api_error!("shell.pane_not_exist"));
    }

    mux.enable_pane_integration(pane_id);
    Ok(api_success!())
}

#[tauri::command]
pub async fn disable_pane_integration(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriApiResult<EmptyData> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Ok(api_error!("shell.pane_not_exist"));
    }

    mux.disable_pane_integration(pane_id);
    Ok(api_success!())
}

#[tauri::command]
pub async fn get_pane_current_command(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriApiResult<Option<FrontendCommandInfo>> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Ok(api_error!("shell.pane_not_exist"));
    }

    let command = mux
        .get_pane_current_command(pane_id)
        .map(|cmd| FrontendCommandInfo::from(&*cmd));
    Ok(api_success!(command))
}

#[tauri::command]
pub async fn get_pane_command_history(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriApiResult<Vec<FrontendCommandInfo>> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Ok(api_error!("shell.pane_not_exist"));
    }

    let history = mux
        .get_pane_command_history(pane_id)
        .into_iter()
        .map(|cmd| FrontendCommandInfo::from(&*cmd))
        .collect();
    Ok(api_success!(history))
}

#[tauri::command]
pub async fn detect_shell_type(shell_program: String) -> TauriApiResult<String> {
    let shell_type = ShellType::from_program(&shell_program);
    Ok(api_success!(shell_type.display_name().to_string()))
}

#[tauri::command]
pub async fn check_shell_integration_support(shell_program: String) -> TauriApiResult<bool> {
    let shell_type = ShellType::from_program(&shell_program);
    Ok(api_success!(shell_type.supports_integration()))
}

/// 后台命令执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundCommandResult {
    pub command: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u64,
    pub success: bool,
}

/// 在后台执行命令，不显示在终端UI中
#[tauri::command]
pub async fn shell_execute_background_command(
    command: String,
    working_directory: Option<String>,
) -> TauriApiResult<BackgroundCommandResult> {
    debug!("执行后台命令: {}", command);

    let start_time = Instant::now();

    // 解析命令和参数 - 正确处理引号
    let parts = match parse_command_line(&command) {
        Ok(parts) => parts,
        Err(_) => return Ok(api_error!("shell.quotes_mismatch")),
    };

    if parts.is_empty() {
        return Ok(api_error!("shell.command_empty"));
    }

    let program = &parts[0];
    let args = &parts[1..];

    let mut cmd = Command::new(program);
    cmd.args(args);

    if let Some(cwd) = working_directory {
        cmd.current_dir(cwd);
    }

    // 执行命令
    match cmd.output() {
        Ok(output) => {
            let execution_time = start_time.elapsed().as_millis() as u64;
            let exit_code = output.status.code().unwrap_or(-1);
            let success = output.status.success();

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            debug!(
                "Background command finished: {} (exit_code: {}, elapsed_ms: {})",
                command, exit_code, execution_time
            );

            Ok(api_success!(BackgroundCommandResult {
                command,
                exit_code,
                stdout,
                stderr,
                execution_time_ms: execution_time,
                success,
            }))
        }
        Err(e) => {
            error!("Background command failed: {} - {}", command, e);
            Ok(api_error!("shell.execute_command_failed"))
        }
    }
}
