mod handler;

pub use handler::handle_menu_event;

use crate::utils::i18n::I18nManager;
use tauri::{
    menu::{Menu, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder},
    AppHandle, Runtime,
};

/// Get menu text
fn t(key: &str) -> String {
    I18nManager::get_text(key, None)
}

/// Create application menu
pub fn create_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = MenuBuilder::new(app);

    #[cfg(target_os = "macos")]
    let menu = menu
        .item(&create_app_menu(app)?)
        .item(&create_shell_menu(app)?)
        .item(&create_edit_menu(app)?)
        .item(&create_view_menu(app)?)
        .item(&create_window_menu(app)?)
        .item(&create_help_menu(app)?);

    #[cfg(not(target_os = "macos"))]
    let menu = menu
        .item(&create_shell_menu(app)?)
        .item(&create_edit_menu(app)?)
        .item(&create_view_menu(app)?)
        .item(&create_window_menu(app)?)
        .item(&create_help_menu(app)?);

    menu.build()
}

/// macOS application menu (OrbitX)
#[cfg(target_os = "macos")]
fn create_app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Submenu<R>> {
    SubmenuBuilder::new(app, "OrbitX")
        .item(&PredefinedMenuItem::about(
            app,
            Some(&t("menu.about")),
            None,
        )?)
        .separator()
        .item(
            &MenuItemBuilder::with_id("preferences", t("menu.settings"))
                .accelerator("CmdOrCtrl+,")
                .build(app)?,
        )
        .separator()
        .item(&PredefinedMenuItem::services(
            app,
            Some(&t("menu.services")),
        )?)
        .separator()
        .item(&PredefinedMenuItem::hide(app, Some(&t("menu.hide")))?)
        .item(&PredefinedMenuItem::hide_others(
            app,
            Some(&t("menu.hide_others")),
        )?)
        .item(&PredefinedMenuItem::show_all(
            app,
            Some(&t("menu.show_all")),
        )?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, Some(&t("menu.quit")))?)
        .build()
}

/// Shell menu
fn create_shell_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Submenu<R>> {
    SubmenuBuilder::new(app, "Shell")
        .item(
            &MenuItemBuilder::with_id("new_terminal", t("menu.new_terminal"))
                .accelerator("CmdOrCtrl+T")
                .build(app)?,
        )
        .build()
}

/// Edit menu
fn create_edit_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Submenu<R>> {
    SubmenuBuilder::new(app, t("menu.edit"))
        .item(&PredefinedMenuItem::undo(app, Some(&t("menu.undo")))?)
        .item(&PredefinedMenuItem::redo(app, Some(&t("menu.redo")))?)
        .separator()
        .item(&PredefinedMenuItem::cut(app, Some(&t("menu.cut")))?)
        .item(&PredefinedMenuItem::copy(app, Some(&t("menu.copy")))?)
        .item(&PredefinedMenuItem::paste(app, Some(&t("menu.paste")))?)
        .item(&PredefinedMenuItem::select_all(
            app,
            Some(&t("menu.select_all")),
        )?)
        .separator()
        .item(
            &MenuItemBuilder::with_id("find", t("menu.find"))
                .accelerator("CmdOrCtrl+F")
                .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::with_id("clear_terminal", t("menu.clear_terminal"))
                .accelerator("CmdOrCtrl+K")
                .build(app)?,
        )
        .build()
}

/// View menu
fn create_view_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Submenu<R>> {
    SubmenuBuilder::new(app, t("menu.view"))
        .item(&PredefinedMenuItem::fullscreen(
            app,
            Some(&t("menu.fullscreen")),
        )?)
        .separator()
        .item(
            &MenuItemBuilder::with_id("toggle_terminal_panel", t("menu.toggle_terminal_panel"))
                .accelerator("CmdOrCtrl+`")
                .build(app)?,
        )
        .build()
}

/// Window menu
fn create_window_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Submenu<R>> {
    SubmenuBuilder::new(app, t("menu.window"))
        .item(&PredefinedMenuItem::minimize(
            app,
            Some(&t("menu.minimize")),
        )?)
        .item(
            &MenuItemBuilder::with_id("toggle_always_on_top", t("menu.always_on_top"))
                .build(app)?,
        )
        .build()
}

/// Help menu
fn create_help_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Submenu<R>> {
    SubmenuBuilder::new(app, t("menu.help"))
        .item(&MenuItemBuilder::with_id("documentation", t("menu.documentation")).build(app)?)
        .item(&MenuItemBuilder::with_id("report_issue", t("menu.report_issue")).build(app)?)
        .build()
}
