/*!
 * Storage system Tauri commands module
 *
 * Responsibility boundary: Only provides runtime terminal state capabilities.
 * Config(JSON) goes through crate::config::* command entry points to avoid write conflicts from duplicate APIs.
 * UI layout persistence uses app_preferences (via workspace::commands).
 */

use crate::utils::TauriApiResult;
use crate::{api_error, api_success};
use tracing::{error, warn};

use crate::agent::terminal::{
    AgentTerminal, AgentTerminalManager, TerminalExecutionMode, TerminalStatus,
};
use crate::storage::types::{
    AgentTerminalMode, AgentTerminalStatus, TerminalRuntimeKind, TerminalRuntimeState,
};

/// Extract the process name from a command line string.
fn extract_process_name(command_line: &str) -> String {
    let first_token = command_line.split_whitespace().next().unwrap_or_default();
    first_token
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(first_token)
        .to_string()
}

/// Extract the last component of a path (basename).
fn path_basename(path: &str) -> &str {
    let trimmed = path.trim_end_matches(['/', '\\']);
    match trimmed.rsplit(['/', '\\']).next() {
        Some(component) => component,
        None => path,
    }
}

fn resolve_terminal_cwd(
    mux: &crate::mux::TerminalMux,
    pane_id: crate::mux::PaneId,
) -> Option<String> {
    match mux.shell_get_pane_cwd(pane_id) {
        Some(cwd) => Some(cwd),
        None => {
            warn!(
                "Terminal {} has no real-time current working directory available, using fallback",
                pane_id.as_u32()
            );
            resolve_terminal_cwd_fallback()
        }
    }
}

fn resolve_terminal_cwd_fallback() -> Option<String> {
    dirs::home_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|path| path.to_string_lossy().into_owned())
        })
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
    if dir_name.is_empty() {
        "~".to_string()
    } else {
        dir_name.to_string()
    }
}

fn map_agent_terminal_mode(mode: &TerminalExecutionMode) -> AgentTerminalMode {
    match mode {
        TerminalExecutionMode::Blocking => AgentTerminalMode::Blocking,
        TerminalExecutionMode::Background => AgentTerminalMode::Background,
    }
}

fn map_agent_terminal_status(status: &TerminalStatus) -> AgentTerminalStatus {
    match status {
        TerminalStatus::Initializing => AgentTerminalStatus::Initializing,
        TerminalStatus::Running => AgentTerminalStatus::Running,
        TerminalStatus::Completed { .. } => AgentTerminalStatus::Completed,
        TerminalStatus::Failed { .. } => AgentTerminalStatus::Failed,
        TerminalStatus::Aborted => AgentTerminalStatus::Aborted,
    }
}

fn build_terminal_runtime_state(
    pane_id: u32,
    cwd: String,
    shell: String,
    computed_display_title: String,
    thread_id: Option<i64>,
    agent_terminal: Option<AgentTerminal>,
) -> TerminalRuntimeState {
    let is_agent_terminal = agent_terminal.is_some();
    let source_label = agent_terminal
        .as_ref()
        .and_then(|terminal| terminal.label.clone())
        .filter(|label| !label.trim().is_empty());
    let display_title = if is_agent_terminal {
        match source_label.clone() {
            Some(label) => label,
            None => computed_display_title,
        }
    } else {
        computed_display_title
    };

    TerminalRuntimeState {
        id: pane_id,
        cwd,
        shell,
        display_title,
        kind: if is_agent_terminal {
            TerminalRuntimeKind::Agent
        } else {
            TerminalRuntimeKind::Workspace
        },
        thread_id: agent_terminal
            .as_ref()
            .map(|terminal| terminal.thread_id)
            .or(thread_id),
        agent_terminal_id: agent_terminal.as_ref().map(|terminal| terminal.id.clone()),
        source_label,
        agent_mode: agent_terminal
            .as_ref()
            .map(|terminal| map_agent_terminal_mode(&terminal.mode)),
        agent_status: agent_terminal
            .as_ref()
            .map(|terminal| map_agent_terminal_status(&terminal.status)),
    }
}

/// Get runtime state of all terminals from backend
#[tauri::command]
pub async fn storage_get_terminals_state() -> TauriApiResult<Vec<TerminalRuntimeState>> {
    use crate::mux::singleton::get_mux;

    let mux = get_mux();
    let terminal_manager = AgentTerminalManager::global();

    let terminals: Vec<TerminalRuntimeState> = mux
        .list_panes()
        .into_iter()
        .filter_map(|pane_id| {
            let pane = mux.get_pane(pane_id)?;
            let shell = pane.shell_info().display_name.clone();
            let cwd = resolve_terminal_cwd(&mux, pane_id)?;

            let shell_state = mux.shell_integration().get_pane_shell_state(pane_id);

            let window_title = shell_state.as_ref().and_then(|s| s.window_title.as_deref());

            let current_process = shell_state
                .as_ref()
                .and_then(|s| s.current_command.as_ref())
                .filter(|cmd| !cmd.is_finished())
                .and_then(|cmd| cmd.command_line.as_deref())
                .map(extract_process_name);

            let display_title =
                compute_display_title(&cwd, &shell, window_title, current_process.as_deref());
            let agent_terminal = terminal_manager
                .as_ref()
                .and_then(|manager| manager.get_terminal_by_pane_id(pane_id.as_u32()));
            let thread_id = mux
                .get_pane_runtime_metadata(pane_id)
                .and_then(|metadata| metadata.thread_id);

            Some(build_terminal_runtime_state(
                pane_id.as_u32(),
                cwd,
                shell,
                display_title,
                thread_id,
                agent_terminal,
            ))
        })
        .collect();

    Ok(api_success!(terminals))
}

/// Get runtime state of specified terminal (including display_title)
#[tauri::command]
pub async fn storage_get_terminal_state(
    pane_id: u32,
) -> TauriApiResult<Option<TerminalRuntimeState>> {
    use crate::mux::singleton::get_mux;
    use crate::mux::PaneId;

    let mux = get_mux();
    let pane_id = PaneId::new(pane_id);
    let terminal_manager = AgentTerminalManager::global();

    let Some(pane) = mux.get_pane(pane_id) else {
        return Ok(api_success!(None));
    };

    let shell = pane.shell_info().display_name.clone();
    let cwd = resolve_terminal_cwd(&mux, pane_id).unwrap_or_else(|| "~".to_string());

    let shell_state = mux.shell_integration().get_pane_shell_state(pane_id);

    let window_title = shell_state.as_ref().and_then(|s| s.window_title.as_deref());

    let current_process = shell_state
        .as_ref()
        .and_then(|s| s.current_command.as_ref())
        .filter(|cmd| !cmd.is_finished())
        .and_then(|cmd| cmd.command_line.as_deref())
        .map(extract_process_name);

    let display_title =
        compute_display_title(&cwd, &shell, window_title, current_process.as_deref());
    let agent_terminal = terminal_manager
        .as_ref()
        .and_then(|manager| manager.get_terminal_by_pane_id(pane_id.as_u32()));
    let thread_id = mux
        .get_pane_runtime_metadata(pane_id)
        .and_then(|metadata| metadata.thread_id);

    Ok(api_success!(Some(build_terminal_runtime_state(
        pane_id.as_u32(),
        cwd,
        shell,
        display_title,
        thread_id,
        agent_terminal,
    ))))
}

/// Get current working directory of specified terminal
#[tauri::command]
pub async fn storage_get_terminal_cwd(pane_id: u32) -> TauriApiResult<String> {
    use crate::mux::singleton::get_mux;
    use crate::mux::PaneId;

    let mux = get_mux();
    let pane_id = PaneId::new(pane_id);

    // Check if pane exists
    if !mux.pane_exists(pane_id) {
        error!("Terminal {} does not exist", pane_id.as_u32());
        return Ok(api_error!("terminal.pane_not_found"));
    }

    // Get real-time CWD from ShellIntegration
    let Some(cwd) = resolve_terminal_cwd(&mux, pane_id) else {
        error!(
            "Terminal {} has no current working directory available after fallback",
            pane_id.as_u32()
        );
        return Ok(api_error!("terminal.cwd_unavailable"));
    };

    Ok(api_success!(cwd))
}
