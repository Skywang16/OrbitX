/*
 * Copyright (C) 2025 OrbitX Contributors
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

//! OrbitX 终端应用后端
//!
//! 这是一个基于 Tauri 框架的终端应用后端实现，提供跨平台的终端功能。
//! 主要功能包括：
//! - 多终端会话管理
//! - 终端输入输出处理
//! - 窗口管理功能

pub mod ai;
pub mod completion;
pub mod config;
pub mod llm;
// pub mod lock_optimization_demo;
pub mod mux;
pub mod shell;
pub mod storage;
pub mod terminal;
pub mod utils;
pub mod window;

use ai::commands::{
    add_ai_model, build_prompt_with_context, create_conversation, delete_conversation,
    get_ai_models, get_compressed_context, get_conversation, get_conversations,
    get_user_prefix_prompt, remove_ai_model, save_message, set_user_prefix_prompt,
    test_ai_connection_with_config, truncate_conversation, update_ai_model,
    update_conversation_title, update_message_content, update_message_status, update_message_steps,
    AIManagerState,
};
use ai::tool::ast::commands::analyze_code;
use ai::tool::network::{simple_web_fetch, web_fetch_headless};
use ai::tool::shell::{TerminalState, *};
use ai::tool::storage::{
    storage_get_config, storage_load_session_state, storage_save_session_state,
    storage_update_config, StorageCoordinatorState,
};
use completion::commands::{
    clear_completion_cache, get_completion_stats, get_completions, get_enhanced_completions,
    init_completion_engine, CompletionState,
};
use config::commands::{
    get_config, get_config_file_info, get_config_file_path, get_config_folder_path,
    open_config_file, open_config_folder, reset_config_to_defaults, save_config,
    subscribe_config_events, update_config, validate_config, ConfigManagerState,
};
use config::shortcuts::{
    // 全新快捷键系统命令
    add_shortcut,
    detect_shortcuts_conflicts,
    execute_shortcut_action,
    export_shortcuts_config,
    get_action_metadata,
    get_current_platform,
    get_registered_actions,
    get_shortcuts_config,
    get_shortcuts_statistics,
    import_shortcuts_config,
    remove_shortcut,
    reset_shortcuts_to_defaults,
    search_shortcuts,
    update_shortcut,
    update_shortcuts_config,
    validate_key_combination,
    validate_shortcuts_config,
    ShortcutManagerState,
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
use llm::commands::{
    llm_call, llm_call_stream, llm_check_model_feature, llm_get_available_models,
    llm_get_model_info, llm_get_provider_models, llm_get_providers, llm_test_model_connection,
    LLMManagerState,
};
use shell::commands::{
    check_shell_integration_status, execute_background_command, get_pane_cwd,
    setup_shell_integration, update_pane_cwd,
};
use std::sync::Arc;
use terminal::commands::{
    get_active_pane, get_active_terminal_context, get_terminal_context, set_active_pane,
    TerminalContextState,
};
use terminal::{ActiveTerminalContextRegistry, TerminalContextService};
use window::commands::{
    clear_directory_cache, get_current_directory, get_home_directory, get_platform_info,
    get_window_opacity, join_paths, manage_window_state, normalize_path, path_exists,
    set_window_opacity, WindowState,
};

use std::path::PathBuf;
use tauri::{Emitter, Manager};
use tracing::{debug, warn};
use tracing_subscriber::{self, EnvFilter};

/// 初始化日志系统
fn init_logging() {
    // 设置默认的日志级别，如果没有设置 RUST_LOG 环境变量
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // 开发构建：显示 debug 级别及以上的日志
        // 发布构建：显示 info 级别及以上的日志
        #[cfg(debug_assertions)]
        let default_level = "debug";
        #[cfg(not(debug_assertions))]
        let default_level = "info";

        EnvFilter::new(default_level)
    });

    // 初始化日志订阅器
    let result = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)  // 显示模块路径
        .with_thread_ids(false)  // 不显示线程ID
        .with_file(false)  // 不显示文件名
        .with_line_number(false)  // 不显示行号
        .with_level(true)  // 显示日志级别
        .try_init();

    match result {
        Ok(_) => {
            println!("日志系统初始化成功");
        }
        Err(e) => {
            eprintln!("日志系统初始化失败: {}", e);
            std::process::exit(1);
        }
    }
}

/// 处理文件打开事件，返回文件所在的目录路径
#[tauri::command]
async fn handle_file_open(path: String) -> Result<String, String> {
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
    init_logging();

    debug!("OrbitX 应用程序启动");
    println!("OrbitX 应用程序启动 - 控制台输出");

    // 测试日志输出（移除以减少启动噪声）

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

    builder
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
                // Windows 和 Linux 使用默认配置
                tauri_plugin_autostart::Builder::new().build()
            }
        })
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
            // 窗口透明度命令
            set_window_opacity,
            get_window_opacity,
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
            // 后台命令执行
            execute_background_command,
            update_config,
            save_config,
            validate_config,
            reset_config_to_defaults,
            get_config_file_path,
            get_config_file_info,
            open_config_file,
            subscribe_config_events,
            get_config_folder_path,
            open_config_folder,
            // 主题系统命令
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
            // Shell integration命令
            setup_shell_integration,
            check_shell_integration_status,
            get_pane_cwd,
            update_pane_cwd,
            // AI模型管理命令
            get_ai_models,
            add_ai_model,
            update_ai_model,
            remove_ai_model,
            test_ai_connection_with_config,
            // LLM调用命令
            llm_call,
            llm_call_stream,
            llm_get_available_models,
            llm_test_model_connection,
            // LLM供应商和模型信息命令
            llm_get_providers,
            llm_get_provider_models,
            llm_get_model_info,
            llm_check_model_feature,
            // AI会话上下文管理命令
            create_conversation,
            get_conversations,
            get_conversation,
            update_conversation_title,
            delete_conversation,
            get_compressed_context,
            build_prompt_with_context,
            get_user_prefix_prompt,
            set_user_prefix_prompt,
            save_message,
            update_message_content,
            update_message_steps,
            update_message_status,
            truncate_conversation,
            // 终端上下文管理命令
            set_active_pane,
            get_active_pane,
            get_terminal_context,
            get_active_terminal_context,
            // 全新快捷键系统命令
            get_shortcuts_config,
            update_shortcuts_config,
            validate_shortcuts_config,
            detect_shortcuts_conflicts,
            add_shortcut,
            remove_shortcut,
            update_shortcut,
            reset_shortcuts_to_defaults,
            get_shortcuts_statistics,
            search_shortcuts,
            execute_shortcut_action,
            get_current_platform,
            export_shortcuts_config,
            import_shortcuts_config,
            get_registered_actions,
            get_action_metadata,
            validate_key_combination,
            // 存储系统命令
            storage_get_config,
            storage_update_config,
            storage_save_session_state,
            storage_load_session_state,
            // 网络请求命令
            web_fetch_headless,
            simple_web_fetch,
            // AST代码分析命令
            analyze_code,
            // 文件拖拽处理命令
            handle_file_open,
            // Shell integration命令
            setup_shell_integration,
            check_shell_integration_status,
            get_pane_cwd,
            update_pane_cwd
        ])
        .setup(|app| {
            // 使用统一的错误处理初始化各个状态管理器
            let init_result = || -> anyhow::Result<()> {
                // 初始化终端状态
                let terminal_state = TerminalState::new()
                    .map_err(|e| anyhow::anyhow!("终端状态初始化失败: {}", e))?;
                app.manage(terminal_state);

                // 创建配置路径管理器
                let paths = config::paths::ConfigPaths::new()
                    .map_err(|e| anyhow::anyhow!("配置路径创建失败: {}", e))?;
                app.manage(paths);

                // 初始化配置管理器状态（必须先初始化，因为其他组件依赖它）
                let config_state = tauri::async_runtime::block_on(async {
                    ConfigManagerState::new()
                        .await
                        .map_err(|e| anyhow::anyhow!("配置管理器状态初始化失败: {}", e))
                })?;
                app.manage(config_state);

                // 初始化快捷键管理器状态（依赖配置管理器）
                let shortcut_state = {
                    let config_state = app.state::<ConfigManagerState>();
                    tauri::async_runtime::block_on(async {
                        ShortcutManagerState::new(&config_state)
                            .await
                            .map_err(|e| anyhow::anyhow!("快捷键管理器状态初始化失败: {}", e))
                    })?
                };
                app.manage(shortcut_state);

                // 提取并管理 TomlConfigManager，以便其他服务可以依赖它
                let config_manager = app.state::<ConfigManagerState>().toml_manager.clone();
                app.manage(config_manager);

                // 初始化存储协调器状态（必须在依赖它的服务之前）
                let storage_state = {
                    let config_manager = app.state::<ConfigManagerState>().toml_manager.clone();
                    tauri::async_runtime::block_on(async {
                        StorageCoordinatorState::new(config_manager)
                            .await
                            .map_err(|e| anyhow::anyhow!("存储协调器状态初始化失败: {}", e))
                    })?
                };
                app.manage(storage_state);

                // 创建 ThemeService 实例用于主题系统
                let theme_service = tauri::async_runtime::block_on(async {
                    use crate::config::{
                        paths::ConfigPaths, theme::ThemeManagerOptions, theme::ThemeService,
                    };

                    // 获取缓存实例
                    let storage_state = app.state::<StorageCoordinatorState>();
                    let cache = storage_state.coordinator.cache();

                    // 从状态中获取配置路径管理器
                    let paths = app.state::<ConfigPaths>().inner().clone();

                    // 创建主题管理器选项
                    let theme_manager_options = ThemeManagerOptions::default();

                    // 创建主题服务
                    let theme_service =
                        ThemeService::new(paths, theme_manager_options, cache).await?;
                    Ok::<ThemeService, anyhow::Error>(theme_service)
                })?;
                app.manage(std::sync::Arc::new(theme_service));

                // 初始化补全引擎状态
                let completion_state = CompletionState::new();
                app.manage(completion_state);

                // 初始化终端上下文服务（使用全局单例TerminalMux并启用集成回调）
                let terminal_context_state = {
                    use crate::shell::ShellIntegrationManager;

                    let registry = Arc::new(ActiveTerminalContextRegistry::new());
                    let shell_integration = Arc::new(
                        ShellIntegrationManager::new()
                            .map_err(|e| anyhow::anyhow!("Shell集成管理器初始化失败: {}", e))?,
                    );
                    // 使用全局单例，避免与事件系统订阅的Mux不一致
                    let global_mux = crate::mux::singleton::get_mux();
                    // 启用与 ShellIntegration 的上下文服务集成（回调、缓存失效、事件转发）
                    let context_service = TerminalContextService::new_with_integration(
                        registry.clone(),
                        shell_integration,
                        global_mux,
                    );

                    TerminalContextState::new(registry, context_service.clone())
                };
                app.manage(terminal_context_state);

                // 初始化AI管理器状态（使用存储协调器中的SQLite管理器和缓存）
                let ai_state = {
                    let storage_state = app.state::<StorageCoordinatorState>();
                    let repositories = storage_state.coordinator.repositories();
                    let cache = storage_state.coordinator.cache();
                    let terminal_context_state = app.state::<TerminalContextState>();
                    let terminal_context_service = terminal_context_state.context_service().clone();

                    let ai_state =
                        AIManagerState::new(repositories, cache, terminal_context_service)
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

                // 初始化LLM管理器状态
                let llm_state = {
                    let storage_state = app.state::<StorageCoordinatorState>();
                    let repositories = storage_state.coordinator.repositories();
                    LLMManagerState::new(repositories)
                };
                app.manage(llm_state);

                // 初始化窗口状态
                let window_state =
                    WindowState::new().map_err(|e| anyhow::anyhow!("窗口状态初始化失败: {}", e))?;
                app.manage(window_state);

                // 初始化TerminalMux状态（用于shell integration命令）
                let terminal_mux = crate::mux::singleton::get_mux();
                app.manage(terminal_mux);

                // Shell Integration现在通过环境变量自动启用，无需复杂初始化

                // 设置统一的终端事件处理器
                setup_unified_terminal_events(app.handle().clone());

                // 启动系统主题监听器
                start_system_theme_listener(app.handle().clone());

                // 在窗口关闭请求时优雅关闭 TerminalMux，释放后台线程
                if let Some(window) = app.get_webview_window("main") {
                    use tauri::WindowEvent;
                    window.on_window_event(|event| {
                        if let WindowEvent::CloseRequested { .. } = event {
                            if let Err(e) = crate::mux::singleton::shutdown_mux() {
                                warn!("关闭 TerminalMux 失败: {}", e);
                            } else {
                            }
                        }
                    });
                }

                // 设置deep-link事件处理
                #[cfg(desktop)]
                {
                    use tauri_plugin_deep_link::DeepLinkExt;

                    let app_handle = app.handle().clone();
                    app.deep_link().on_open_url(move |event| {
                        let urls = event.urls();
                        // 处理 file:// URL
                        for url in urls {
                            if url.scheme() == "file" {
                                // 使用 url.to_file_path() 方法，它能正确处理中文字符
                                match url.to_file_path() {
                                    Ok(path_buf) => {
                                        let path_str = path_buf.to_string_lossy().to_string();

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
                        let _ = window.emit("startup-file", path_str);
                    }
                }

                // 确保主窗口可见并处于正确位置
                if let Some(window) = app.get_webview_window("main") {
                    let window_clone = window.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                        // 检查窗口位置是否异常
                        if let Ok(position) = window_clone.outer_position() {
                            let x = position.x;
                            let y = position.y;

                            // 如果位置异常，重置到安全位置
                            if x < -500 || y < -500 || x > 5000 || y > 5000 {
                                let _ = window_clone.set_position(tauri::Position::Logical(
                                    tauri::LogicalPosition { x: 100.0, y: 100.0 },
                                ));
                            }
                        }

                        let _ = window_clone.show();
                        let _ = window_clone.set_focus();
                    });
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

/// 获取回退的主题文件列表
fn get_fallback_theme_list() -> Vec<String> {
    vec![
        "dark.toml".to_string(),
        "light.toml".to_string(),
        "dracula.toml".to_string(),
        "gruvbox-dark.toml".to_string(),
        "index.toml".to_string(),
        "monokai.toml".to_string(),
        "nord.toml".to_string(),
        "one-dark.toml".to_string(),
        "solarized-dark.toml".to_string(),
        "solarized-light.toml".to_string(),
        "tokyo-night.toml".to_string(),
    ]
}

/// 动态获取资源目录中的所有主题文件
async fn get_theme_files_from_resources<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    use std::path::PathBuf;
    use tauri::path::BaseDirectory;

    // 开发模式直接从项目根目录读取，生产模式从资源读取
    let themes_resource_path = if cfg!(debug_assertions) {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        current_dir.join("..").join("config").join("themes")
    } else {
        app_handle
            .path()
            .resolve("themes", BaseDirectory::Resource)
            .map_err(|_| "无法解析资源路径")?
    };
    match std::fs::read_dir(&themes_resource_path) {
        Ok(entries) => {
            let mut theme_files = Vec::new();
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(file_name) = path.file_name() {
                            if let Some(file_name_str) = file_name.to_str() {
                                if file_name_str.ends_with(".toml") {
                                    theme_files.push(file_name_str.to_string());
                                }
                            }
                        }
                    }
                }
            }

            if theme_files.is_empty() {
                Ok(get_fallback_theme_list())
            } else {
                Ok(theme_files)
            }
        }
        Err(_) => Ok(get_fallback_theme_list()),
    }
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

    // 动态获取所有主题文件，避免硬编码列表
    let theme_files = get_theme_files_from_resources(app_handle).await?;

    let mut _copied_count = 0;

    for theme_file in &theme_files {
        let dest_path = themes_dir.join(theme_file);

        // 如果文件已存在，跳过
        if dest_path.exists() {
            continue;
        }

        // 开发模式直接从项目根目录读取，生产模式从资源读取
        let source_path = if cfg!(debug_assertions) {
            let current_dir =
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let dev_file_path = current_dir
                .join("..")
                .join("config")
                .join("themes")
                .join(theme_file);
            Some(dev_file_path)
        } else {
            app_handle
                .path()
                .resolve(theme_file, BaseDirectory::Resource)
                .ok()
        };

        if let Some(resource_path) = source_path {
            if let Ok(content) = fs::read_to_string(&resource_path) {
                if fs::write(&dest_path, content).is_ok() {
                    _copied_count += 1;
                }
            }
        }
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
                    // 成功复制默认配置文件（静默）
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

/// 启动系统主题监听器
fn start_system_theme_listener<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>) {
    use config::theme::{handle_system_theme_change, SystemThemeDetector};
    use std::sync::Arc;

    let handle = Arc::new(app_handle);
    let _listener_handle = SystemThemeDetector::start_system_theme_listener({
        let handle = Arc::clone(&handle);
        move |is_dark| {
            let handle = Arc::clone(&handle);
            tauri::async_runtime::spawn(async move {
                if let Err(e) = handle_system_theme_change(&*handle, is_dark).await {
                    warn!("处理系统主题变化失败: {}", e);
                } else {
                    // 系统主题已更新（静默）
                }
            });
        }
    });

    // 存储监听器句柄，防止被drop
    // 注意：在实际应用中，你可能需要在应用关闭时停止监听器
}

/// 设置统一的终端事件处理器
///
/// 替代原有的重复事件处理逻辑，提供单一的事件集成路径
fn setup_unified_terminal_events<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>) {
    use crate::mux::singleton::get_mux;
    use crate::terminal::create_terminal_event_handler;

    // 获取终端多路复用器
    let mux = get_mux();

    // 获取终端上下文状态
    let terminal_context_state = app_handle.state::<TerminalContextState>();
    let registry = terminal_context_state.registry();

    // 订阅上下文事件
    let context_event_receiver = registry.subscribe_events();

    // 创建并启动统一的事件处理器
    match create_terminal_event_handler(app_handle.clone(), &mux, context_event_receiver) {
        Ok(handler) => {
            tracing::debug!("统一终端事件处理器已启动");
            // Use Box::leak to prevent the handler from being dropped
            // This ensures the event subscriptions remain active for the app lifetime
            // The memory will be cleaned up when the process exits
            Box::leak(Box::new(handler));
        }
        Err(e) => {
            tracing::error!("启动统一终端事件处理器失败: {}", e);
        }
    }
}
