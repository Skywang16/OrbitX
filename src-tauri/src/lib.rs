//! 主要功能包括：
//! - 多终端会话管理
//! - 终端输入输出处理
//! - 窗口管理功能

pub mod ai;
pub mod commands;
pub mod completion;
pub mod config;
pub mod llm;
// pub mod lock_optimization_demo;
pub mod ck;
pub mod mux;
pub mod setup;
pub mod shell;
pub mod storage;
pub mod terminal;
pub mod utils;
pub mod window;

use commands::register_all_commands;
use setup::{
    ensure_main_window_visible, handle_startup_args, init_logging, init_plugin,
    initialize_app_states, setup_app_events, setup_deep_links,
};
use utils::i18n::I18nManager;

use tauri::{Emitter, Manager};

/// 应用程序主入口点
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志系统
    init_logging();

    // 初始化国际化
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
            // 处理命令行参数中的文件路径
            if argv.len() > 1 {
                let file_path = &argv[1];
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("file-dropped", file_path);
                }
            }
        }));
    }

    // 构建应用程序
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

    // 注册所有命令
    let app_result = register_all_commands(app_result);

    // 设置应用程序
    app_result
        .setup(|app| {
            // 初始化所有状态管理器
            if let Err(e) = initialize_app_states(app) {
                eprintln!("应用程序初始化失败: {}", e);
                std::process::exit(1);
            }

            // 设置事件和监听器
            setup_app_events(app);

            // 设置深度链接处理
            setup_deep_links(app);

            // 处理启动参数
            handle_startup_args(app);

            // 确保主窗口可见
            ensure_main_window_visible(app);

            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("启动 Tauri 应用程序时发生错误: {}", e);
            std::process::exit(1);
        });
}
