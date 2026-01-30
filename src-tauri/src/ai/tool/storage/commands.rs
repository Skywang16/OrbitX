/*!
 * 存储系统 Tauri 命令模块
 *
 * 职责边界：只提供"State(Data/Runtime)"相关能力（msgpack 会话状态、Mux 运行时终端状态）。
 * Config(JSON) 走 crate::config::* 命令入口，避免两套 API 造成写入分叉。
 */

use crate::storage::messagepack::MessagePackManager;
use crate::storage::types::SessionState;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use std::sync::Arc;
use tauri::State;
use tracing::error;

/// Extract the process name from a command line string.
fn extract_process_name(command_line: &str) -> String {
    let first_token = command_line.trim().split_whitespace().next().unwrap_or("");
    first_token
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(first_token)
        .to_string()
}

/// Extract the last component of a path (basename).
fn path_basename(path: &str) -> &str {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
}

/// Check if window title is useful (not a default shell prompt format).
fn is_useful_window_title(title: &str, cwd: &str) -> bool {
    if title.len() < 2 {
        return false;
    }
    // Skip user@host format (shell default)
    if title.contains('@') && title.chars().take_while(|&c| c != ':').any(|c| c == '@') {
        return false;
    }
    // Skip if it's just the cwd or basename
    let basename = path_basename(cwd);
    if title == cwd || title == basename || title == "~" {
        return false;
    }
    true
}

/// Compute the display title for a terminal tab.
/// Priority: useful window title > running process (from shell integration) > dir name
///
/// Window title (OSC 2) is set by the application itself (e.g. vim, claude),
/// so it's more accurate than our guess from the command line.
fn compute_display_title(
    cwd: &str,
    shell: &str,
    window_title: Option<&str>,
    current_process: Option<&str>,
) -> String {
    // 1. Application-set window title (highest priority)
    if let Some(title) = window_title {
        if is_useful_window_title(title, cwd) {
            return title.to_string();
        }
    }

    // 2. Running process from shell integration (not the shell itself)
    if let Some(process) = current_process {
        let process_lower = process.to_lowercase();
        let shell_lower = shell.to_lowercase();
        if !process.is_empty() && process_lower != shell_lower {
            return process.to_string();
        }
    }

    // 3. Fallback to directory name
    let dir_name = path_basename(cwd);
    if dir_name.is_empty() { "~".to_string() } else { dir_name.to_string() }
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
            error!("会话状态加载失败，已忽略");
            Ok(api_success!(None))
        }
    }
}

/// 从后端获取所有终端的运行时状态
#[tauri::command]
pub async fn storage_get_terminals_state(
) -> TauriApiResult<Vec<crate::storage::types::TerminalRuntimeState>> {
    use crate::mux::singleton::get_mux;
    use crate::storage::types::TerminalRuntimeState;

    let mux = get_mux();

    let terminals: Vec<TerminalRuntimeState> = mux
        .list_panes()
        .into_iter()
        .filter_map(|pane_id| {
            let pane = mux.get_pane(pane_id)?;
            let shell = pane.shell_info().display_name.clone();

            let cwd = mux.shell_get_pane_cwd(pane_id).unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "~".to_string())
            });

            let shell_state = mux.shell_integration().get_pane_shell_state(pane_id);

            let window_title = shell_state.as_ref().and_then(|s| s.window_title.as_deref());

            let current_process = shell_state
                .as_ref()
                .and_then(|s| s.current_command.as_ref())
                .filter(|cmd| !cmd.is_finished())
                .and_then(|cmd| cmd.command_line.as_deref())
                .map(|line| extract_process_name(line));

            let display_title =
                compute_display_title(&cwd, &shell, window_title, current_process.as_deref());

            Some(TerminalRuntimeState {
                id: pane_id.as_u32(),
                cwd,
                shell,
                display_title,
            })
        })
        .collect();

    Ok(api_success!(terminals))
}

/// 获取指定终端的运行时状态（包括 display_title）
#[tauri::command]
pub async fn storage_get_terminal_state(
    pane_id: u32,
) -> TauriApiResult<Option<crate::storage::types::TerminalRuntimeState>> {
    use crate::mux::singleton::get_mux;
    use crate::mux::PaneId;
    use crate::storage::types::TerminalRuntimeState;

    let mux = get_mux();
    let pane_id = PaneId::new(pane_id);

    let Some(pane) = mux.get_pane(pane_id) else {
        return Ok(api_success!(None));
    };

    let shell = pane.shell_info().display_name.clone();

    let cwd = mux.shell_get_pane_cwd(pane_id).unwrap_or_else(|| {
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "~".to_string())
    });

    let shell_state = mux.shell_integration().get_pane_shell_state(pane_id);

    let window_title = shell_state.as_ref().and_then(|s| s.window_title.as_deref());

    let current_process = shell_state
        .as_ref()
        .and_then(|s| s.current_command.as_ref())
        .filter(|cmd| !cmd.is_finished())
        .and_then(|cmd| cmd.command_line.as_deref())
        .map(|line| extract_process_name(line));

    let display_title =
        compute_display_title(&cwd, &shell, window_title, current_process.as_deref());

    Ok(api_success!(Some(TerminalRuntimeState {
        id: pane_id.as_u32(),
        cwd,
        shell,
        display_title,
    })))
}

/// 获取指定终端的当前工作目录
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
