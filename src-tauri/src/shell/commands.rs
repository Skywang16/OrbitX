//! Shell Integration Tauri Commands
//!
//! 提供前端调用的Shell Integration相关命令

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;
use tauri::State;
use tracing::{debug, error};
use anyhow::Context;
use crate::utils::error::{TauriResult, ToTauriResult};

use super::{CommandInfo, PaneShellState, ShellType};
use crate::mux::{PaneId, TerminalMux};

/// 解析命令行，正确处理引号
fn parse_command_line(command: &str) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    let mut current_part = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut chars = command.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                if !current_part.is_empty() {
                    parts.push(current_part.clone());
                    current_part.clear();
                }
            }
            _ => {
                current_part.push(ch);
            }
        }
    }

    // 添加最后一个部分
    if !current_part.is_empty() {
        parts.push(current_part);
    }

    // 检查引号是否匹配
    if in_single_quote || in_double_quote {
        return Err("引号不匹配".to_string());
    }

    Ok(parts)
}

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

#[tauri::command]
pub async fn check_shell_integration_status(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriResult<bool> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    let status = mux.is_pane_integrated(pane_id);
    Ok(status)
}

#[tauri::command]
pub async fn setup_shell_integration(
    pane_id: u32,
    silent: Option<bool>,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriResult<()> {
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
        .context("设置Shell集成失败")
        .to_tauri()?;

    Ok(())
}

#[tauri::command]
pub async fn get_pane_cwd(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriResult<Option<String>> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    let cwd = mux.get_pane_cwd(pane_id);
    Ok(cwd)
}

#[tauri::command]
pub async fn update_pane_cwd(
    pane_id: u32,
    cwd: String,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriResult<()> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    mux.update_pane_cwd(pane_id, cwd);
    Ok(())
}

#[tauri::command]
pub async fn get_pane_shell_state(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriResult<Option<FrontendPaneState>> {
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

#[tauri::command]
pub async fn set_pane_shell_type(
    pane_id: u32,
    shell_type: String,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriResult<()> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    let shell_type = ShellType::from_program(&shell_type);
    mux.set_pane_shell_type(pane_id, shell_type);
    Ok(())
}

#[tauri::command]
pub async fn generate_shell_integration_script(
    shell_type: String,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriResult<String> {
    let mux = &*state;
    let shell_type = ShellType::from_program(&shell_type);

    if !shell_type.supports_integration() {
        return Err(format!(
            "Shell type {} does not support integration",
            shell_type.display_name()
        ));
    }

    mux.generate_shell_integration_script(&shell_type)
        .context("生成Shell集成脚本失败")
        .to_tauri()
}

#[tauri::command]
pub async fn generate_shell_env_vars(
    shell_type: String,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriResult<HashMap<String, String>> {
    let mux = &*state;
    let shell_type = ShellType::from_program(&shell_type);

    let env_vars = mux.generate_shell_env_vars(&shell_type);
    Ok(env_vars)
}

#[tauri::command]
pub async fn enable_pane_integration(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriResult<()> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    mux.enable_pane_integration(pane_id);
    Ok(())
}

#[tauri::command]
pub async fn disable_pane_integration(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriResult<()> {
    let mux = &*state;
    let pane_id = PaneId::from(pane_id);

    if !mux.pane_exists(pane_id) {
        return Err(format!("Pane {} does not exist", pane_id));
    }

    mux.disable_pane_integration(pane_id);
    Ok(())
}

#[tauri::command]
pub async fn get_pane_current_command(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriResult<Option<FrontendCommandInfo>> {
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

#[tauri::command]
pub async fn get_pane_command_history(
    pane_id: u32,
    state: State<'_, Arc<TerminalMux>>,
) -> TauriResult<Vec<FrontendCommandInfo>> {
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

#[tauri::command]
pub async fn detect_shell_type(shell_program: String) -> TauriResult<String> {
    let shell_type = ShellType::from_program(&shell_program);
    Ok(shell_type.display_name().to_string())
}

#[tauri::command]
pub async fn check_shell_integration_support(shell_program: String) -> TauriResult<bool> {
    let shell_type = ShellType::from_program(&shell_program);
    Ok(shell_type.supports_integration())
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
pub async fn execute_background_command(
    command: String,
    working_directory: Option<String>,
) -> TauriResult<BackgroundCommandResult> {
    debug!("执行后台命令: {}", command);

    let start_time = Instant::now();

    // 解析命令和参数 - 正确处理引号
    let parts = parse_command_line(&command)?;
    if parts.is_empty() {
        return Err("命令不能为空".to_string());
    }

    let program = &parts[0];
    let args = &parts[1..];

    // 创建命令
    let mut cmd = Command::new(program);
    cmd.args(args);

    // 设置工作目录
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
                "后台命令执行完成: {} (退出码: {}, 耗时: {}ms)",
                command, exit_code, execution_time
            );

            Ok(BackgroundCommandResult {
                command,
                exit_code,
                stdout,
                stderr,
                execution_time_ms: execution_time,
                success,
            })
        }
        Err(e) => {
            error!("后台命令执行失败: {} - {}", command, e);
            Err(format!("命令执行失败: {}", e))
        }
    }
}
