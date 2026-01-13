//! Shell 集成命令

use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tauri::State;
use tokio::process::Command as AsyncCommand;
use tracing::error;

use super::{CommandInfo, PaneShellState};
use crate::mux::{PaneId, TerminalMux};

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
                .map(FrontendCommandInfo::from),
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
pub async fn shell_pane_setup_integration(
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
pub async fn shell_pane_get_state(
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

/// 后台命令执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundCommandResult {
    pub program: String,
    pub args: Vec<String>,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u64,
    pub success: bool,
}

/// 在后台执行命令（结构化参数），不显示在终端UI中
#[tauri::command]
pub async fn shell_execute_background_program(
    program: String,
    args: Vec<String>,
    working_directory: Option<String>,
) -> TauriApiResult<BackgroundCommandResult> {
    let start_time = Instant::now();

    if program.trim().is_empty() {
        return Ok(api_error!("shell.command_empty"));
    }

    let mut cmd = AsyncCommand::new(&program);
    cmd.args(&args);

    if let Some(cwd) = working_directory {
        cmd.current_dir(cwd);
    }

    match cmd.output().await {
        Ok(output) => {
            let execution_time = start_time.elapsed().as_millis() as u64;
            let exit_code = output.status.code().unwrap_or(-1);
            let success = output.status.success();

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            Ok(api_success!(BackgroundCommandResult {
                program,
                args,
                exit_code,
                stdout,
                stderr,
                execution_time_ms: execution_time,
                success,
            }))
        }
        Err(e) => {
            error!("Background command failed: {} {:?} - {}", program, args, e);
            Ok(api_error!("shell.execute_command_failed"))
        }
    }
}
