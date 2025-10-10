pub mod agent;
pub mod ai;
pub mod ck;
pub mod commands;
pub mod completion;
pub mod config;
pub mod filesystem;
pub mod llm;
pub mod mux;
pub mod node;
pub mod setup;
pub mod shell;
pub mod storage;
pub mod terminal;
pub mod utils;
pub mod window;

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

    tracing::debug!("OrbitX 应用程序启动");
    println!("OrbitX 应用程序启动 - 控制台输出");

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

    app_result
        .setup(|app| {
            if let Err(e) = initialize_app_states(app) {
                eprintln!("应用程序初始化失败: {}", e);
                std::process::exit(1);
            }

            setup_app_events(app);
            setup_deep_links(app);
            handle_startup_args(app);
            ensure_main_window_visible(app);

            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("启动 Tauri 应用程序时发生错误: {}", e);
            std::process::exit(1);
        });
}
