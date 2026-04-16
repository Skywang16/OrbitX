use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_opener::OpenerExt;
use tracing::warn;

const DOCS_URL: &str = "https://github.com/user/orbitx";
const ISSUES_URL: &str = "https://github.com/user/orbitx/issues";

/// Handle menu events
pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event_id: &str) {
    match event_id {
        // Events forwarded to frontend
        "new_terminal"
        | "find"
        | "clear_terminal"
        | "toggle_terminal_panel"
        | "toggle_always_on_top"
        | "preferences" => {
            if let Err(err) = app.emit(&format!("menu:{}", event_id.replace('_', "-")), ()) {
                warn!("Failed to emit menu event '{}': {}", event_id, err);
            }
        }

        // Help
        "documentation" => {
            if let Err(err) = app.opener().open_url(DOCS_URL, None::<&str>) {
                warn!("Failed to open documentation URL: {}", err);
            }
        }
        "report_issue" => {
            if let Err(err) = app.opener().open_url(ISSUES_URL, None::<&str>) {
                warn!("Failed to open issue tracker URL: {}", err);
            }
        }

        _ => {}
    }
}
