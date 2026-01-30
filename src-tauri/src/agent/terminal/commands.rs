use crate::agent::terminal::AgentTerminalManager;
use crate::agent::terminal::AgentTerminal;
use crate::utils::{TauriApiResult, EmptyData};
use crate::{api_success, api_error};

#[tauri::command]
pub async fn agent_terminal_list(session_id: Option<i64>) -> TauriApiResult<Vec<AgentTerminal>> {
    let manager = match AgentTerminalManager::global() {
        Some(m) => m,
        None => return Ok(api_error!("agent.terminal_manager_not_initialized")),
    };
    Ok(api_success!(manager.list_terminals(session_id)))
}

#[tauri::command]
pub async fn agent_terminal_abort(terminal_id: String) -> TauriApiResult<EmptyData> {
    let manager = match AgentTerminalManager::global() {
        Some(m) => m,
        None => return Ok(api_error!("agent.terminal_manager_not_initialized")),
    };
    match manager.abort_terminal(&terminal_id) {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("common.operation_failed")),
    }
}

#[tauri::command]
pub async fn agent_terminal_remove(terminal_id: String) -> TauriApiResult<EmptyData> {
    let manager = match AgentTerminalManager::global() {
        Some(m) => m,
        None => return Ok(api_error!("agent.terminal_manager_not_initialized")),
    };
    match manager.remove_terminal(&terminal_id) {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("common.operation_failed")),
    }
}
