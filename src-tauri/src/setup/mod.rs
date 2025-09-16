//! 应用程序初始化

use crate::ai::tool::shell::TerminalState;
use crate::ai::tool::storage::StorageCoordinatorState;
use crate::ai::AIManagerState;
use crate::completion::commands::CompletionState;
use crate::config::{ConfigManagerState, ShortcutManagerState};
use crate::llm::commands::LLMManagerState;
use crate::terminal::{
    commands::TerminalContextState, ActiveTerminalContextRegistry, TerminalContextService,
    TerminalChannelState,
};
use crate::window::commands::WindowState;

use std::sync::Arc;
use tauri::{Emitter, Manager};
use tracing::warn;
use tracing_subscriber::{self, EnvFilter};

pub fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        #[cfg(debug_assertions)]
        let default_level = "debug";
        #[cfg(not(debug_assertions))]
        let default_level = "info";

        EnvFilter::new(default_level)
    });

    let result = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .with_level(true)
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

/// 初始化所有应用状态管理器
pub fn initialize_app_states<R: tauri::Runtime>(app: &tauri::App<R>) -> anyhow::Result<()> {
    let terminal_state =
        TerminalState::new().map_err(|e| anyhow::anyhow!("终端状态初始化失败: {}", e))?;
    app.manage(terminal_state);

    let paths = crate::config::paths::ConfigPaths::new()
        .map_err(|e| anyhow::anyhow!("配置路径创建失败: {}", e))?;
    app.manage(paths);

    let config_state = tauri::async_runtime::block_on(async {
        ConfigManagerState::new()
            .await
            .map_err(|e| anyhow::anyhow!("配置管理器状态初始化失败: {}", e))
    })?;
    app.manage(config_state);

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

    let storage_state = {
        let config_manager = app.state::<ConfigManagerState>().toml_manager.clone();
        tauri::async_runtime::block_on(async {
            StorageCoordinatorState::new(config_manager)
                .await
                .map_err(|e| anyhow::anyhow!("存储协调器状态初始化失败: {}", e))
        })?
    };
    app.manage(storage_state);

    let theme_service = tauri::async_runtime::block_on(async {
        use crate::config::{paths::ConfigPaths, theme::ThemeManagerOptions, theme::ThemeService};

        let storage_state = app.state::<StorageCoordinatorState>();
        let cache = storage_state.coordinator.cache();

        // 从状态中获取配置路径管理器
        let paths = app.state::<ConfigPaths>().inner().clone();

        let theme_manager_options = ThemeManagerOptions::default();

        let theme_service = ThemeService::new(paths, theme_manager_options, cache).await?;
        Ok::<ThemeService, anyhow::Error>(theme_service)
    })?;
    app.manage(std::sync::Arc::new(theme_service));

    let completion_state = CompletionState::new();
    app.manage(completion_state);

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

    let ai_state = {
        let storage_state = app.state::<StorageCoordinatorState>();
        let repositories = storage_state.coordinator.repositories();
        let cache = storage_state.coordinator.cache();
        let terminal_context_state = app.state::<TerminalContextState>();
        let terminal_context_service = terminal_context_state.context_service().clone();

        let ai_state = AIManagerState::new(repositories, cache, terminal_context_service)
            .map_err(|e| anyhow::anyhow!("AI管理器状态初始化失败: {}", e))?;

        tauri::async_runtime::block_on(async {
            ai_state
                .initialize()
                .await
                .map_err(|e| anyhow::anyhow!("AI服务初始化失败: {}", e))
        })?;

        ai_state
    };
    app.manage(ai_state);

    let llm_state = {
        let storage_state = app.state::<StorageCoordinatorState>();
        let repositories = storage_state.coordinator.repositories();
        LLMManagerState::new(repositories)
    };
    app.manage(llm_state);

    let window_state =
        WindowState::new().map_err(|e| anyhow::anyhow!("窗口状态初始化失败: {}", e))?;
    app.manage(window_state);

    let terminal_mux = crate::mux::singleton::get_mux();
    app.manage(terminal_mux);

    // Manage Terminal Channel State for streaming bytes via Tauri Channel
    let terminal_channel_state = TerminalChannelState::new();
    app.manage(terminal_channel_state);


    Ok(())
}

/// 设置应用程序事件和监听器
pub fn setup_app_events<R: tauri::Runtime>(app: &tauri::App<R>) {
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
}

/// 设置深度链接处理
pub fn setup_deep_links<R: tauri::Runtime>(app: &tauri::App<R>) {
    #[cfg(desktop)]
    {
        use tauri_plugin_deep_link::DeepLinkExt;

        let app_handle = app.handle().clone();
        app.deep_link().on_open_url(move |event| {
            let urls = event.urls();
            for url in urls {
                if url.scheme() == "file" {
                    // 使用 url.to_file_path() 方法，它能正确处理中文字符
                    match url.to_file_path() {
                        Ok(path_buf) => {
                            let path_str = path_buf.to_string_lossy().to_string();

                            // 发送到前端
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.emit("file-dropped", path_str);
                            }
                        }
                        Err(e) => {
                            warn!("无法解析文件路径: {:?}, 错误: {:?}", url, e);

                            // 降级处理：手动解码URL路径
                            let file_path = url.path();
                            if let Ok(decoded_path) = urlencoding::decode(file_path) {
                                let path_str = decoded_path.to_string();

                                if let Some(window) = app_handle.get_webview_window("main") {
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
}

/// 处理启动时的命令行参数
pub fn handle_startup_args<R: tauri::Runtime>(app: &tauri::App<R>) {
    let env = app.env();
    if env.args_os.len() > 1 {
        let file_path = &env.args_os[1];
        if let Some(window) = app.get_webview_window("main") {
            let path_str = file_path.to_string_lossy().to_string();
            let _ = window.emit("startup-file", path_str);
        }
    }
}

/// 确保主窗口正确显示
pub fn ensure_main_window_visible<R: tauri::Runtime>(app: &tauri::App<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let window_clone = window.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if let Ok(position) = window_clone.outer_position() {
                let x = position.x;
                let y = position.y;

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
}

/// 设置统一的终端事件处理器
fn setup_unified_terminal_events<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>) {
    use crate::mux::singleton::get_mux;
    use crate::terminal::create_terminal_event_handler;

    let mux = get_mux();

    let terminal_context_state = app_handle.state::<TerminalContextState>();
    let registry = terminal_context_state.registry();

    // 订阅上下文事件
    let context_event_receiver = registry.subscribe_events();

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

/// 启动系统主题监听器
fn start_system_theme_listener<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>) {
    use crate::config::theme::{handle_system_theme_change, SystemThemeDetector};
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
    use crate::config::paths::ConfigPaths;
    use std::fs;
    use tauri::path::BaseDirectory;

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
            if let Ok(content) = std::fs::read_to_string(&resource_path) {
                if std::fs::write(&dest_path, content).is_ok() {
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
    use crate::config::paths::ConfigPaths;
    use std::fs;
    use tauri::path::BaseDirectory;

    let paths = ConfigPaths::new()?;
    let config_dir = paths.config_dir();
    let config_file_path = paths.config_file();

    // 确保配置目录存在
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }

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

/// 创建一个 Tauri 插件，用于在应用启动时复制默认主题
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
