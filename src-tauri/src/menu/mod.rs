mod handler;

pub use handler::handle_menu_event;

use crate::utils::i18n::I18nManager;
use tauri::{
    menu::{Menu, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder},
    AppHandle, Runtime,
};

/// 获取菜单文本
fn t(key: &str) -> String {
    I18nManager::get_text(key, None)
}

/// 创建应用菜单
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

/// macOS 应用菜单 (OrbitX)
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

/// Shell 菜单
fn create_shell_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Submenu<R>> {
    SubmenuBuilder::new(app, "Shell")
        .item(
            &MenuItemBuilder::with_id("new_tab", t("menu.new_tab"))
                .accelerator("CmdOrCtrl+T")
                .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::with_id("close_tab", t("menu.close_tab"))
                .accelerator("CmdOrCtrl+W")
                .build(app)?,
        )
        .build()
}

/// 编辑菜单
fn create_edit_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Submenu<R>> {
    SubmenuBuilder::new(app, &t("menu.edit"))
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

/// 显示菜单
fn create_view_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Submenu<R>> {
    SubmenuBuilder::new(app, &t("menu.view"))
        .item(
            &MenuItemBuilder::with_id("increase_font_size", t("menu.increase_font_size"))
                .accelerator("CmdOrCtrl+=")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("decrease_font_size", t("menu.decrease_font_size"))
                .accelerator("CmdOrCtrl+-")
                .build(app)?,
        )
        .separator()
        .item(&MenuItemBuilder::with_id("increase_opacity", t("menu.increase_opacity")).build(app)?)
        .item(&MenuItemBuilder::with_id("decrease_opacity", t("menu.decrease_opacity")).build(app)?)
        .separator()
        .item(&PredefinedMenuItem::fullscreen(
            app,
            Some(&t("menu.fullscreen")),
        )?)
        .separator()
        .item(
            &MenuItemBuilder::with_id("toggle_ai_sidebar", t("menu.toggle_ai_sidebar"))
                .accelerator("CmdOrCtrl+L")
                .build(app)?,
        )
        .build()
}

/// 窗口菜单
fn create_window_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Submenu<R>> {
    SubmenuBuilder::new(app, &t("menu.window"))
        .item(&PredefinedMenuItem::minimize(
            app,
            Some(&t("menu.minimize")),
        )?)
        .item(
            &MenuItemBuilder::with_id("toggle_always_on_top", t("menu.always_on_top"))
                .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::with_id("prev_tab", t("menu.prev_tab"))
                .accelerator("CmdOrCtrl+Shift+[")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id("next_tab", t("menu.next_tab"))
                .accelerator("CmdOrCtrl+Shift+]")
                .build(app)?,
        )
        .build()
}

/// 帮助菜单
fn create_help_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Submenu<R>> {
    SubmenuBuilder::new(app, &t("menu.help"))
        .item(&MenuItemBuilder::with_id("documentation", t("menu.documentation")).build(app)?)
        .item(&MenuItemBuilder::with_id("report_issue", t("menu.report_issue")).build(app)?)
        .build()
}
