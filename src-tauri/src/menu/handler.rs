use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_opener::OpenerExt;

const DOCS_URL: &str = "https://github.com/user/orbitx";
const ISSUES_URL: &str = "https://github.com/user/orbitx/issues";

/// 处理菜单事件
pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event_id: &str) {
    match event_id {
        // 转发到前端的事件
        "new_tab"
        | "close_tab"
        | "find"
        | "clear_terminal"
        | "increase_font_size"
        | "decrease_font_size"
        | "increase_opacity"
        | "decrease_opacity"
        | "toggle_ai_sidebar"
        | "toggle_always_on_top"
        | "prev_tab"
        | "next_tab"
        | "preferences" => {
            let _ = app.emit(&format!("menu:{}", event_id.replace('_', "-")), ());
        }

        // 帮助
        "documentation" => {
            let _ = app.opener().open_url(DOCS_URL, None::<&str>);
        }
        "report_issue" => {
            let _ = app.opener().open_url(ISSUES_URL, None::<&str>);
        }

        _ => {}
    }
}
