//! TermX 终端应用后端
//!
//! 这是一个基于 Tauri 框架的终端应用后端实现，提供跨平台的终端功能。
//! 主要功能包括：
//! - 多终端会话管理
//! - 终端输入输出处理
//! - 窗口管理功能

// 模块声明
pub mod ai; // AI集成功能模块
mod commands; // Tauri 命令处理模块
pub mod completion; // 终端补全功能模块
pub mod config; // 统一配置系统模块
                // pub mod lock_optimization_demo; // 锁优化演示模块 - 暂时注释掉
pub mod mux; // 终端多路复用器核心模块
pub mod storage; // 统一存储系统模块
pub mod utils; // 工具和错误处理模块
pub mod window; // 窗口管理功能模块

use ai::commands::{
    add_ai_model, analyze_error, clear_ai_cache, clear_chat_history, explain_command,
    get_ai_models, get_chat_history, get_terminal_context, get_user_prefix_prompt, remove_ai_model,
    save_chat_history, send_chat_message, set_default_ai_model, set_user_prefix_prompt,
    stream_chat_message_with_channel, test_ai_connection, update_ai_model, update_terminal_context,
    AIManagerState,
};
use commands::{TerminalState, *};
use completion::commands::{
    clear_completion_cache, get_completion_stats, get_completions, get_enhanced_completions,
    init_completion_engine, CompletionState,
};
use config::commands::{
    // 主题系统命令
    create_builtin_themes,
    get_config,
    get_config_file_info,
    get_config_file_path,
    get_theme_index,
    get_theme_list,
    load_theme,
    open_config_file,
    refresh_theme_index,
    reset_config_to_defaults,
    save_config,
    subscribe_config_events,
    switch_theme,
    update_config,
    validate_config,
    validate_theme,
    ConfigManagerState,
};
use config::shortcuts::commands::{
    adapt_shortcuts_for_platform,
    // 快捷键系统命令
    add_shortcut,
    get_current_platform,
    get_shortcuts_config,
    get_shortcuts_statistics,
    remove_shortcut,
    reset_shortcuts_to_defaults,
    update_shortcut,
    update_shortcuts_config,
};
use config::terminal_commands::{
    // 终端配置命令
    detect_system_shells,
    get_shell_info,
    get_terminal_config,
    reset_terminal_config_to_defaults,
    update_cursor_config,
    update_terminal_behavior_config,
    update_terminal_config,
    validate_terminal_config,
    validate_terminal_shell_path,
};
use config::{
    // 新的主题系统命令
    get_available_themes,
    get_current_theme,
    get_theme_config_status,
    set_follow_system_theme,
    set_terminal_theme,
};
use storage::commands::{
    storage_clear_cache, storage_get_cache_stats, storage_get_config, storage_get_storage_stats,
    storage_health_check, storage_load_session_state, storage_preload_cache, storage_query_data,
    storage_save_data, storage_save_session_state, storage_update_config, StorageCoordinatorState,
};
use window::commands::{
    clear_directory_cache, get_current_directory, get_home_directory, get_platform_info,
    join_paths, manage_window_state, normalize_path, path_exists, WindowState,
};

use std::path::PathBuf;
use tauri::{Emitter, Manager};
use tracing::{info, warn};
use utils::init_logging;

/// 处理文件打开事件，返回文件所在的目录路径
#[tauri::command]
async fn handle_file_open(path: String) -> Result<String, String> {
    info!("处理文件打开事件: {}", path);

    // 确保路径字符串正确处理中文字符
    let path_buf = PathBuf::from(&path);

    if path_buf.exists() {
        let dir = if path_buf.is_file() {
            // 如果是文件，返回其父目录
            match path_buf.parent() {
                Some(parent) => parent,
                None => {
                    warn!("文件没有父目录: {}", path);
                    &path_buf
                }
            }
        } else {
            // 如果是目录，直接返回
            &path_buf
        };

        // 使用 to_string_lossy() 确保中文字符正确转换
        let dir_str = dir.to_string_lossy().to_string();
        info!("文件所在目录: {}", dir_str);
        Ok(dir_str)
    } else {
        let error_msg = format!("路径不存在: {}", path);
        warn!("{}", error_msg);
        Err(error_msg)
    }
}

/// 创建一个 Tauri 插件，用于在应用启动时复制默认主题。
pub fn init_plugin<R: tauri::Runtime>(name: &'static str) -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new(name)
        .setup(|app_handle, _api| {
            // 从资源目录复制配置文件和主题文件到用户配置目录
            let app_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                // 复制默认配置文件
                if let Err(e) = copy_default_config_from_resources(&app_handle).await {
                    eprintln!("复制默认配置文件失败: {}", e);
                }

                // 复制主题文件
                if let Err(e) = copy_themes_from_resources(&app_handle).await {
                    eprintln!("复制主题文件失败: {}", e);
                }
            });
            Ok(())
        })
        .build()
}

/// 应用程序主入口点
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志系统
    if let Err(e) = init_logging() {
        eprintln!("日志系统初始化失败: {}", e);
        std::process::exit(1);
    }

    info!("TermX 应用程序启动");
    println!("TermX 应用程序启动 - 控制台输出");

    let mut builder = tauri::Builder::default();

    // 配置single instance插件 (仅限桌面平台)
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            info!("New app instance opened with args: {:?}", argv);
            // 处理命令行参数中的文件路径
            if argv.len() > 1 {
                let file_path = &argv[1];
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("file-dropped", file_path);
                }
            }
        }));
    }

    builder
        .plugin(init_plugin("init"))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .invoke_handler(tauri::generate_handler![
            // 窗口管理命令
            manage_window_state,
            get_current_directory,
            get_home_directory,
            clear_directory_cache,
            normalize_path,
            join_paths,
            path_exists,
            get_platform_info,
            // 文件拖拽命令
            handle_file_open,
            // 终端管理命令
            create_terminal,
            write_to_terminal,
            resize_terminal,
            close_terminal,
            list_terminals,
            get_available_shells,
            get_default_shell,
            validate_shell_path,
            create_terminal_with_shell,
            find_shell_by_name,
            find_shell_by_path,
            get_shell_stats,
            initialize_shell_manager,
            validate_shell_manager,
            // 终端缓冲区命令
            get_terminal_buffer,
            set_terminal_buffer,
            // 补全功能命令
            init_completion_engine,
            get_completions,
            get_enhanced_completions,
            clear_completion_cache,
            get_completion_stats,
            // 配置管理命令
            get_config,
            update_config,
            save_config,
            validate_config,
            reset_config_to_defaults,
            get_config_file_path,
            get_config_file_info,
            open_config_file,
            subscribe_config_events,
            // 主题系统命令
            get_theme_list,
            get_theme_index,
            load_theme,
            switch_theme,
            validate_theme,
            refresh_theme_index,
            create_builtin_themes,
            // 新的主题系统命令
            get_theme_config_status,
            get_current_theme,
            set_terminal_theme,
            set_follow_system_theme,
            get_available_themes,
            // 终端配置命令
            get_terminal_config,
            update_terminal_config,
            validate_terminal_config,
            reset_terminal_config_to_defaults,
            detect_system_shells,
            validate_terminal_shell_path,
            get_shell_info,
            update_cursor_config,
            update_terminal_behavior_config,
            // AI功能命令
            get_ai_models,
            add_ai_model,
            update_ai_model,
            remove_ai_model,
            test_ai_connection,
            set_default_ai_model,
            send_chat_message,
            stream_chat_message_with_channel,
            explain_command,
            analyze_error,
            get_user_prefix_prompt,
            set_user_prefix_prompt,
            get_terminal_context,
            update_terminal_context,
            clear_ai_cache,
            // AI聊天历史命令
            get_chat_history,
            save_chat_history,
            clear_chat_history,
            // 快捷键系统命令
            get_shortcuts_config,
            update_shortcuts_config,
            adapt_shortcuts_for_platform,
            get_current_platform,
            reset_shortcuts_to_defaults,
            get_shortcuts_statistics,
            add_shortcut,
            remove_shortcut,
            update_shortcut,
            // 存储系统命令
            storage_get_config,
            storage_update_config,
            storage_save_session_state,
            storage_load_session_state,
            storage_query_data,
            storage_save_data,
            storage_health_check,
            storage_get_cache_stats,
            storage_get_storage_stats,
            storage_preload_cache,
            storage_clear_cache,
            // 文件拖拽处理命令
            handle_file_open
        ])
        .setup(|app| {
            // 使用统一的错误处理初始化各个状态管理器
            let init_result = || -> anyhow::Result<()> {
                // 初始化终端状态
                info!("开始初始化终端状态管理器");
                let terminal_state = TerminalState::new()
                    .map_err(|e| anyhow::anyhow!("终端状态初始化失败: {}", e))?;
                app.manage(terminal_state);
                info!("终端状态管理器已初始化");

                // 初始化配置管理器状态（必须先初始化，因为其他组件依赖它）
                info!("开始初始化配置管理器状态");
                let config_state = tauri::async_runtime::block_on(async {
                    ConfigManagerState::new()
                        .await
                        .map_err(|e| anyhow::anyhow!("配置管理器状态初始化失败: {}", e))
                })?;
                app.manage(config_state);
                info!("配置管理器状态已初始化");

                // 创建 ConfigManager 实例用于 AI 管理器和主题系统
                info!("开始创建配置管理器实例");
                // 从已初始化的ConfigManagerState中获取TomlConfigManager
                let config_manager = {
                    let config_state = app.state::<ConfigManagerState>();
                    config_state.toml_manager.clone()
                };

                // 创建 ThemeService 实例用于主题系统
                info!("开始创建主题服务实例");
                let theme_service = tauri::async_runtime::block_on(async {
                    use crate::config::{
                        paths::ConfigPaths, theme::ThemeManager, theme::ThemeManagerOptions,
                        theme::ThemeService,
                    };

                    // 创建配置路径管理器
                    let paths = ConfigPaths::new()
                        .map_err(|e| anyhow::anyhow!("配置路径创建失败: {}", e))?;

                    // 创建主题管理器
                    let theme_manager_options = ThemeManagerOptions::default();
                    let theme_manager = ThemeManager::new(paths, theme_manager_options)
                        .await
                        .map_err(|e| anyhow::anyhow!("主题管理器创建失败: {}", e))?;
                    let theme_manager = std::sync::Arc::new(theme_manager);

                    // 创建主题服务
                    let theme_service = ThemeService::new(theme_manager);
                    Ok::<ThemeService, anyhow::Error>(theme_service)
                })?;
                let theme_service = std::sync::Arc::new(theme_service);

                // 管理状态
                app.manage(config_manager.clone());
                app.manage(theme_service);
                info!("配置管理器和主题服务状态已管理");

                // 初始化补全引擎状态
                info!("开始初始化补全引擎状态管理器");
                let completion_state = CompletionState::new();
                app.manage(completion_state);
                info!("补全引擎状态管理器已初始化");

                // 初始化存储协调器状态（必须在AI管理器之前）
                info!("开始初始化存储协调器状态");
                let storage_state = tauri::async_runtime::block_on(async {
                    StorageCoordinatorState::new(config_manager.clone())
                        .await
                        .map_err(|e| anyhow::anyhow!("存储协调器状态初始化失败: {}", e))
                })?;
                app.manage(storage_state);
                info!("存储协调器状态已初始化");

                // 初始化AI管理器状态（使用存储协调器中的SQLite管理器）
                info!("开始初始化AI管理器状态");
                let ai_state = {
                    let storage_state = app.state::<StorageCoordinatorState>();
                    let sqlite_manager = storage_state.coordinator.sqlite_manager();
                    let ai_state = AIManagerState::new(Some(sqlite_manager))
                        .map_err(|e| anyhow::anyhow!("AI管理器状态初始化失败: {}", e))?;

                    // 初始化AI服务
                    tauri::async_runtime::block_on(async {
                        ai_state
                            .initialize()
                            .await
                            .map_err(|e| anyhow::anyhow!("AI服务初始化失败: {}", e))
                    })?;

                    ai_state
                };
                app.manage(ai_state);
                info!("AI管理器状态已初始化");

                // 初始化窗口状态
                info!("开始初始化窗口状态管理器");
                let window_state =
                    WindowState::new().map_err(|e| anyhow::anyhow!("窗口状态初始化失败: {}", e))?;
                app.manage(window_state);
                info!("窗口状态管理器已初始化");

                // 设置Tauri集成
                info!("开始设置Tauri事件集成");
                setup_tauri_integration(app.handle().clone());
                info!("Tauri事件集成设置完成");

                // 设置deep-link事件处理
                #[cfg(desktop)]
                {
                    use tauri_plugin_deep_link::DeepLinkExt;

                    let app_handle = app.handle().clone();
                    app.deep_link().on_open_url(move |event| {
                        let urls = event.urls();
                        info!("Deep link URLs: {:?}", urls);

                        // 处理 file:// URL
                        for url in urls {
                            if url.scheme() == "file" {
                                // 使用 url.to_file_path() 方法，它能正确处理中文字符
                                match url.to_file_path() {
                                    Ok(path_buf) => {
                                        let path_str = path_buf.to_string_lossy().to_string();
                                        info!("处理 Deep link 文件路径: {}", path_str);

                                        // 发送到前端
                                        if let Some(window) = app_handle.get_webview_window("main")
                                        {
                                            let _ = window.emit("file-dropped", path_str);
                                        }
                                    }
                                    Err(e) => {
                                        warn!("无法解析文件路径: {:?}, 错误: {:?}", url, e);

                                        // 降级处理：手动解码URL路径
                                        let file_path = url.path();
                                        if let Ok(decoded_path) = urlencoding::decode(file_path) {
                                            let path_str = decoded_path.to_string();
                                            info!("降级解码后的文件路径: {}", path_str);

                                            if let Some(window) =
                                                app_handle.get_webview_window("main")
                                            {
                                                let _ = window.emit("file-dropped", path_str);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });

                    // 注册运行时deep links (仅限开发和Linux)
                    #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
                    {
                        if let Err(e) = app.deep_link().register_all() {
                            warn!("Failed to register deep links: {}", e);
                        }
                    }
                }

                // 处理启动时的命令行参数
                let env = app.env();
                if env.args_os.len() > 1 {
                    let file_path = &env.args_os[1];
                    if let Some(window) = app.get_webview_window("main") {
                        let path_str = file_path.to_string_lossy().to_string();
                        info!("Startup file argument: {}", path_str);
                        let _ = window.emit("startup-file", path_str);
                    }
                }

                Ok(())
            };

            // 执行初始化并处理错误
            if let Err(e) = init_result() {
                eprintln!("应用程序初始化失败: {}", e);
                std::process::exit(1);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("启动 Tauri 应用程序时发生错误: {}", e);
            std::process::exit(1);
        });
}

/// 从资源目录复制主题文件到用户配置目录
async fn copy_themes_from_resources<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::config::ConfigPaths;
    use std::fs;
    use tauri::path::BaseDirectory;

    // 获取用户配置目录
    let paths = ConfigPaths::new()?;
    let themes_dir = paths.themes_dir();

    // 确保主题目录存在
    if !themes_dir.exists() {
        fs::create_dir_all(themes_dir)?;
    }

    // 定义需要复制的主题文件列表
    let theme_files = [
        "dark.toml",
        "light.toml",
        "dracula.toml",
        "gruvbox-dark.toml",
        "monokai.toml",
        "nord.toml",
        "one-dark.toml",
        "solarized-dark.toml",
        "solarized-light.toml",
        "tokyo-night.toml",
    ];

    let mut copied_count = 0;

    for theme_file in &theme_files {
        let dest_path = themes_dir.join(theme_file);

        // 如果文件已存在，跳过
        if dest_path.exists() {
            continue;
        }

        // 尝试从资源目录读取文件
        match app_handle
            .path()
            .resolve(theme_file, BaseDirectory::Resource)
        {
            Ok(resource_path) => match fs::read_to_string(&resource_path) {
                Ok(content) => match fs::write(&dest_path, content) {
                    Ok(_) => {
                        copied_count += 1;
                    }
                    Err(_) => {
                        // 静默处理写入失败，不影响应用启动
                    }
                },
                Err(_) => {
                    // 静默处理读取失败，不影响应用启动
                }
            },
            Err(_) => {
                // 静默处理路径解析失败，不影响应用启动
            }
        }
    }

    // 只在有复制文件时才输出日志
    if copied_count > 0 {
        info!("已复制 {} 个默认主题文件", copied_count);
    }

    Ok(())
}

/// 从资源目录复制默认配置文件到用户配置目录
async fn copy_default_config_from_resources<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::config::ConfigPaths;
    use std::fs;
    use tauri::path::BaseDirectory;

    // 获取用户配置目录
    let paths = ConfigPaths::new()?;
    let config_dir = paths.config_dir();
    let config_file_path = paths.config_file();

    // 确保配置目录存在
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }

    // 如果配置文件已存在，跳过
    if config_file_path.exists() {
        return Ok(());
    }

    // 尝试从资源目录读取默认配置文件
    match app_handle
        .path()
        .resolve("config.toml", BaseDirectory::Resource)
    {
        Ok(resource_path) => match fs::read_to_string(&resource_path) {
            Ok(content) => match fs::write(&config_file_path, content) {
                Ok(_) => {
                    info!("成功从资源目录复制默认配置文件");
                }
                Err(e) => {
                    warn!("写入默认配置文件失败: {}", e);
                }
            },
            Err(e) => {
                warn!("读取资源配置文件失败: {}", e);
            }
        },
        Err(e) => {
            warn!("解析配置文件资源路径失败: {}", e);
        }
    }

    Ok(())
}
