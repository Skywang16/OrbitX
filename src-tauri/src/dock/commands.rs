use crate::dock::state::TabEntry;
use crate::utils::TauriApiResult;
use crate::{api_error, api_success};
use tauri::State;

#[tauri::command]
pub async fn dock_update_tabs(
    dock_manager: State<'_, crate::dock::DockManager>,
    tabs: Vec<TabEntry>,
    active_tab_id: Option<String>,
) -> TauriApiResult<()> {
    match dock_manager.update_tabs(tabs, active_tab_id) {
        Ok(_) => Ok(api_success!(())),
        Err(e) => {
            tracing::error!("Failed to update dock tabs: {}", e);
            Ok(api_error!("dock.update_failed"))
        }
    }
}

#[tauri::command]
pub async fn dock_get_tabs(
    dock_manager: State<'_, crate::dock::DockManager>,
) -> TauriApiResult<Vec<TabEntry>> {
    match dock_manager.state().get_tabs() {
        Ok(tabs) => Ok(api_success!(tabs)),
        Err(e) => {
            tracing::error!("Failed to get dock tabs: {}", e);
            Ok(api_error!("dock.get_failed"))
        }
    }
}

#[tauri::command]
pub async fn dock_clear_tabs(
    dock_manager: State<'_, crate::dock::DockManager>,
) -> TauriApiResult<()> {
    match dock_manager.state().clear() {
        Ok(_) => Ok(api_success!(())),
        Err(e) => {
            tracing::error!("Failed to clear dock tabs: {}", e);
            Ok(api_error!("dock.clear_failed"))
        }
    }
}
