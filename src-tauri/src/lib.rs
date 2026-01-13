pub mod agent;
pub mod ai;
pub mod checkpoint;
pub mod commands;
pub mod completion;
pub mod config;
pub mod code_intel;
pub mod dock;
pub mod events;
pub mod filesystem;
pub mod git;
pub mod llm;
pub mod menu;
pub mod mux;
pub mod node;
pub mod setup;
pub mod shell;
pub mod settings;
pub mod storage;
pub mod terminal;
pub mod utils;
pub mod vector_db;
pub mod window;
pub mod workspace;

use setup::{
    ensure_main_window_visible, handle_startup_args, init_logging, init_plugin,
    initialize_app_states, setup_app_events, setup_deep_links,
};
use utils::i18n::I18nManager;

use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();

    if let Err(e) = I18nManager::initialize() {
        eprintln!("初始化国际化失败: {}", e);
    }

    let mut builder = tauri::Builder::default();

    // 配置single instance插件 (仅限桌面平台)
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if argv.len() > 1 {
                let file_path = &argv[1];
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("file-dropped", file_path);
                }
            }
        }));
    }

    let app_result = builder
        .plugin(init_plugin("init"))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin({
            #[cfg(target_os = "macos")]
            {
                tauri_plugin_autostart::init(
                    tauri_plugin_autostart::MacosLauncher::AppleScript,
                    None,
                )
            }
            #[cfg(not(target_os = "macos"))]
            {
                tauri_plugin_autostart::Builder::new().build()
            }
        });

    let app_result = commands::register_all_commands(app_result);

    let app_instance = app_result
        .setup(|app| {
            if let Err(e) = initialize_app_states(app) {
                eprintln!("应用程序初始化失败: {}", e);
                std::process::exit(1);
            }

            // 创建并设置应用菜单
            match menu::create_menu(app.handle()) {
                Ok(menu) => {
                    if let Err(e) = app.set_menu(menu) {
                        eprintln!("设置菜单失败: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("创建菜单失败: {}", e);
                }
            }

            // 注册菜单事件处理器
            let app_handle = app.handle().clone();
            app.on_menu_event(move |_app, event| {
                menu::handle_menu_event(&app_handle, event.id().as_ref());
            });

            setup_app_events(app);
            setup_deep_links(app);
            handle_startup_args(app);
            ensure_main_window_visible(app);

            Ok(())
        })
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("构建 Tauri 应用程序时发生错误: {}", e);
            std::process::exit(1);
        });

    app_instance.run(|app_handle, event| {
        match event {
            // macOS: 监听应用激活事件（点击 Dock 图标）
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                // 用户点击了 Dock 图标，检查主窗口是否被隐藏
                if let Some(window) = app_handle.get_webview_window("main") {
                    // 检查窗口是否可见
                    if let Ok(is_visible) = window.is_visible() {
                        if !is_visible {
                            // 如果窗口被隐藏，重新显示它
                            if let Err(e) = window.show() {
                                eprintln!("无法显示窗口: {}", e);
                            }
                            // 将窗口置于最前
                            let _ = window.set_focus();
                        }
                    }
                }
            }
            // 监听应用退出事件（Command+Q 或菜单退出）
            // 在应用真正退出前清理资源
            tauri::RunEvent::ExitRequested { .. } => {
                if let Err(e) = crate::mux::singleton::shutdown_mux() {
                    eprintln!("清理 TerminalMux 失败: {}", e);
                }
            }
            _ => {}
        }
    });
}
