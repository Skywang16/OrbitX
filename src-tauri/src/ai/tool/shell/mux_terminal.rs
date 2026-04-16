/*!
 * Tauri command interface for terminal module
 *
 * Note: Event handling has been moved to terminal::event_handler for unified event management.
 * This module now focuses solely on terminal command implementations.
 */

use tauri::{AppHandle, Runtime, State};
use tracing::error;

use crate::mux::{
    get_mux, MuxSessionConfig, MuxShellConfig, PaneId, PtySize, ShellInfo, ShellManager,
};
use crate::utils::{ApiResponse, EmptyData, TauriApiResult};
use crate::{api_error, api_success};

/// Parameter validation helper function
fn terminal_size_valid(rows: u16, cols: u16) -> bool {
    rows > 0 && cols > 0
}

/// Terminal state management
///
pub struct TerminalState {
    // Keep this struct for future extension of other states
    _placeholder: (),
}

impl TerminalState {
    /// Initialization method
    ///
    /// Note: Mux is not validated at this time, as Mux needs to be initialized in setup
    pub fn new() -> Result<Self, String> {
        let state = Self { _placeholder: () };
        Ok(state)
    }

    /// Validate state integrity
    /// Only validates when called, not during initialization
    pub fn validate(&self) -> TauriApiResult<EmptyData> {
        let mux = get_mux();

        // Verify Mux instance is accessible
        mux.pane_count();

        Ok(ApiResponse::ok(EmptyData))
    }
}

/// Create new terminal session
///
#[tauri::command]
pub async fn terminal_create<R: Runtime>(
    rows: u16,
    cols: u16,
    cwd: Option<String>,
    thread_id: Option<i64>,
    _app: AppHandle<R>,
    _state: State<'_, TerminalState>,
) -> TauriApiResult<u32> {
    if !terminal_size_valid(rows, cols) {
        return Ok(api_error!("shell.terminal_size_invalid"));
    }

    let mux = get_mux();
    let size = PtySize::new(rows, cols);

    let mut shell_config = match MuxShellConfig::with_default_shell() {
        Ok(shell_config) => shell_config,
        Err(err) => {
            error!(
                "Failed to resolve default shell for terminal creation: {}",
                err
            );
            return Ok(api_error!("shell.shell_not_found"));
        }
    };
    if let Some(working_dir) = cwd.clone() {
        shell_config.working_directory = Some(working_dir.into());
    }
    let mut config = MuxSessionConfig::with_shell(shell_config);
    config.runtime_metadata.thread_id = thread_id;

    let result = mux
        .create_pane_with_config(size, &config)
        .await
        .map(|pane_id| (pane_id, cwd));

    match result {
        Ok((pane_id, working_dir)) => {
            // Immediately sync initial CWD to ShellIntegration to avoid cold start gap
            if let Some(initial_cwd) = &working_dir {
                mux.shell_update_pane_cwd(pane_id, initial_cwd.clone());
            }

            Ok(api_success!(pane_id.as_u32()))
        }
        Err(_) => Ok(api_error!("shell.create_terminal_failed")),
    }
}

/// Write data to terminal
///
#[tauri::command]
pub async fn terminal_write(
    pane_id: u32,
    data: String,
    _state: State<'_, TerminalState>,
) -> TauriApiResult<EmptyData> {
    if data.is_empty() {
        return Ok(api_error!("common.empty_content"));
    }

    let mux = get_mux();
    let pane_id_obj = PaneId::from(pane_id);

    match mux.write_to_pane(pane_id_obj, data.as_bytes()) {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("shell.write_terminal_failed")),
    }
}

/// Resize terminal
///
#[tauri::command]
pub async fn terminal_resize(
    pane_id: u32,
    rows: u16,
    cols: u16,
    _state: State<'_, TerminalState>,
) -> TauriApiResult<EmptyData> {
    if !terminal_size_valid(rows, cols) {
        return Ok(api_error!("shell.terminal_size_invalid"));
    }

    let mux = get_mux();
    let pane_id_obj = PaneId::from(pane_id);
    let size = PtySize::new(rows, cols);

    match mux.resize_pane(pane_id_obj, size) {
        Ok(_) => Ok(api_success!()),
        Err(err) => match err {
            crate::mux::error::TerminalMuxError::PaneNotFound { .. } => Ok(api_success!()),
            _ => Ok(api_error!("shell.resize_terminal_failed")),
        },
    }
}

/// Close terminal session
///
#[tauri::command]
pub async fn terminal_close(
    pane_id: u32,
    _state: State<'_, TerminalState>,
) -> TauriApiResult<EmptyData> {
    let mux = get_mux();
    let pane_id_obj = PaneId::from(pane_id);

    // Atomic operation: directly attempt to remove pane, avoiding race condition between check and delete
    match mux.remove_pane(pane_id_obj) {
        Ok(_) => Ok(api_success!()),
        Err(err) => {
            match err {
                crate::mux::error::TerminalMuxError::PaneNotFound { .. } => {
                    // Pane doesn't exist, consider operation successful
                    Ok(api_success!())
                }
                _ => {
                    // Other errors, return failure
                    Ok(api_error!("shell.close_terminal_failed"))
                }
            }
        }
    }
}

/// Get terminal list
///
#[tauri::command]
pub async fn terminal_list() -> TauriApiResult<Vec<u32>> {
    let mux = get_mux();
    let pane_ids: Vec<u32> = mux.list_panes().into_iter().map(|id| id.as_u32()).collect();
    Ok(api_success!(pane_ids))
}

/// Get list of available shells in the system
///
#[tauri::command]
pub async fn terminal_get_available_shells() -> TauriApiResult<Vec<ShellInfo>> {
    let shells = ShellManager::detect_available_shells();
    Ok(api_success!(shells))
}

/// Get default shell information for the system
///
#[tauri::command]
pub async fn terminal_get_default_shell() -> TauriApiResult<ShellInfo> {
    match ShellManager::terminal_get_default_shell() {
        Ok(default_shell) => Ok(api_success!(default_shell)),
        Err(err) => {
            error!("Failed to resolve default shell: {}", err);
            Ok(api_error!("shell.shell_not_found"))
        }
    }
}

/// Validate if shell path is valid
///
#[tauri::command]
pub async fn terminal_validate_shell_path(path: String) -> TauriApiResult<bool> {
    if path.trim().is_empty() {
        return Ok(api_error!("shell.command_empty"));
    }

    let is_valid = ShellManager::validate_shell(&path);
    Ok(api_success!(is_valid))
}

/// Create terminal with specified shell
///
#[tauri::command]
pub async fn terminal_create_with_shell<R: Runtime>(
    shell_name: Option<String>,
    rows: u16,
    cols: u16,
    thread_id: Option<i64>,
    _app: AppHandle<R>,
    _state: State<'_, TerminalState>,
) -> TauriApiResult<u32> {
    if rows == 0 || cols == 0 {
        return Ok(api_error!("shell.terminal_size_invalid"));
    }

    let shell_info = match shell_name {
        Some(name) => match ShellManager::terminal_find_shell_by_name(&name) {
            Some(shell) => shell,
            None => {
                error!("Specified shell not found: {}", name);
                return Ok(api_error!("shell.shell_not_found"));
            }
        },
        None => match ShellManager::terminal_get_default_shell() {
            Ok(shell) => shell,
            Err(err) => {
                error!("Failed to resolve default shell: {}", err);
                return Ok(api_error!("shell.shell_not_found"));
            }
        },
    };

    let mux = get_mux();
    let size = PtySize::new(rows, cols);

    let shell_config = MuxShellConfig::with_shell(shell_info);
    let mut config = MuxSessionConfig::with_shell(shell_config);
    config.runtime_metadata.thread_id = thread_id;

    // Create pane using configuration
    match mux.create_pane_with_config(size, &config).await {
        Ok(pane_id) => Ok(api_success!(pane_id.as_u32())),
        Err(_) => {
            error!("Failed to create terminal");
            Ok(api_error!("shell.create_terminal_failed"))
        }
    }
}
