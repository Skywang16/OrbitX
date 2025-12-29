//! 应用程序初始化

pub mod error;

pub use error::{SetupError, SetupResult};

use crate::ai::tool::shell::TerminalState;
use crate::ai::AIManagerState;
use crate::completion::commands::CompletionState;
use crate::config::{ConfigManagerState, ShortcutManagerState};
use crate::llm::commands::LLMManagerState;
use crate::terminal::{
    commands::TerminalContextState, ActiveTerminalContextRegistry, TerminalChannelState,
    TerminalContextService,
};
use crate::window::commands::WindowState;

use std::sync::Arc;
use tauri::{Emitter, Manager};
use tracing::warn;
use tracing_subscriber::{self, EnvFilter};

pub fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        #[cfg(debug_assertions)]
        let default_level = "debug,ignore=warn";
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
        Ok(_) => {}
        Err(e) => {
            eprintln!("Log system initialization failed: {}", e);
            std::process::exit(1);
        }
    }
}

/// 初始化所有应用状态管理器
pub fn initialize_app_states<R: tauri::Runtime>(app: &tauri::App<R>) -> SetupResult<()> {
    let terminal_state = TerminalState::new().map_err(SetupError::TerminalState)?;
    app.manage(terminal_state);

    let paths = crate::config::paths::ConfigPaths::new()?;
    app.manage(paths);

    let config_state = tauri::async_runtime::block_on(async { ConfigManagerState::new().await })?;
    app.manage(config_state);

    let shortcut_state = {
        let config_state = app.state::<ConfigManagerState>();
        tauri::async_runtime::block_on(async { ShortcutManagerState::new(&config_state).await })?
    };
    app.manage(shortcut_state);

    // 提取并管理 TomlConfigManager，以便其他服务可以依赖它
    let config_manager = app.state::<ConfigManagerState>().toml_manager.clone();
    app.manage(config_manager);

    // 初始化 DatabaseManager
    let database_manager = {
        use crate::storage::{DatabaseManager, StoragePaths};
        use std::env;

        let app_dir = if let Ok(dir) = env::var("OrbitX_DATA_DIR") {
            std::path::PathBuf::from(dir)
        } else {
            let data_dir = dirs::data_dir().ok_or("无法获取系统数据目录")?;
            data_dir.join("OrbitX")
        };

        let paths = StoragePaths::new(app_dir)?;
        let options = crate::storage::DatabaseOptions::default();

        Arc::new(tauri::async_runtime::block_on(async {
            let db = DatabaseManager::new(paths.clone(), options).await?;
            db.initialize().await?;
            Ok::<_, SetupError>(db)
        })?)
    };
    app.manage(database_manager.clone());

    // 初始化 MessagePackManager
    let messagepack_manager = {
        use crate::storage::{MessagePackManager, MessagePackOptions, StoragePaths};
        use std::env;

        let app_dir = if let Ok(dir) = env::var("OrbitX_DATA_DIR") {
            std::path::PathBuf::from(dir)
        } else {
            let data_dir = dirs::data_dir().ok_or("无法获取系统数据目录")?;
            data_dir.join("OrbitX")
        };

        let paths = StoragePaths::new(app_dir)?;

        Arc::new(tauri::async_runtime::block_on(async {
            MessagePackManager::new(paths, MessagePackOptions::default()).await
        })?)
    };
    app.manage(messagepack_manager);

    // 初始化 UnifiedCache
    let cache = Arc::new(crate::storage::cache::UnifiedCache::new());
    app.manage(cache.clone());

    // 在 ThemeManager 初始化前复制主题文件
    tauri::async_runtime::block_on(async {
        let _ = copy_themes_from_resources(app.handle()).await;
        let _ = copy_default_config_from_resources(app.handle()).await;
    });

    let theme_service = tauri::async_runtime::block_on(async {
        use crate::config::{paths::ConfigPaths, theme::ThemeManagerOptions, theme::ThemeService};

        let cache = app
            .state::<Arc<crate::storage::cache::UnifiedCache>>()
            .inner()
            .clone();
        let paths = app.state::<ConfigPaths>().inner().clone();

        ThemeService::new(paths, ThemeManagerOptions::default(), cache).await
    })?;
    app.manage(Arc::new(theme_service));

    let completion_state = CompletionState::new();
    app.manage(completion_state);

    // 创建 Shell Integration 并注册 Node 版本回调
    let shell_integration = Arc::new(crate::shell::ShellIntegrationManager::new());

    // TODO: Node版本变化事件已迁移到IoHandler处理
    // 如需前端通知,应添加MuxNotification::NodeVersionChanged类型

    // 初始化全局 Mux
    let global_mux =
        crate::mux::singleton::init_mux_with_shell_integration(shell_integration.clone())
            .expect("初始化全局 TerminalMux 失败");

    let terminal_context_state = {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let cache = app
            .state::<Arc<crate::storage::cache::UnifiedCache>>()
            .inner()
            .clone();

        // 启用与 ShellIntegration 的上下文服务集成（回调、缓存失效、事件转发）
        let context_service = TerminalContextService::new_with_integration(
            registry.clone(),
            shell_integration,
            global_mux.clone(),
            cache,
        );

        TerminalContextState::new(registry, context_service.clone())
    };
    app.manage(terminal_context_state);

    let ai_state = {
        let database = app
            .state::<Arc<crate::storage::DatabaseManager>>()
            .inner()
            .clone();
        let cache = app
            .state::<Arc<crate::storage::cache::UnifiedCache>>()
            .inner()
            .clone();
        let terminal_context_state = app.state::<TerminalContextState>();
        let terminal_context_service = terminal_context_state.context_service().clone();

        let state = AIManagerState::new(database, cache, terminal_context_service)
            .map_err(SetupError::AIState)?;

        tauri::async_runtime::block_on(async {
            state
                .initialize()
                .await
                .map_err(SetupError::AIInitialization)
        })?;

        state
    };
    app.manage(ai_state);

    let llm_state = {
        let database = app
            .state::<Arc<crate::storage::DatabaseManager>>()
            .inner()
            .clone();
        LLMManagerState::new(database)
    };
    app.manage(llm_state);

    // 初始化 Checkpoint 服务（提前创建，供 TaskExecutor 使用）
    let checkpoint_service = {
        use crate::checkpoint::{BlobStore, CheckpointService, CheckpointStorage};

        let database = app
            .state::<Arc<crate::storage::DatabaseManager>>()
            .inner()
            .clone();
        let pool = database.pool().clone();

        let storage = Arc::new(CheckpointStorage::new(pool.clone()));
        let blob_store = Arc::new(BlobStore::new(pool));
        Arc::new(CheckpointService::new(storage, blob_store))
    };

    // 初始化TaskExecutor状态（带有 Checkpoint 服务）
    let task_executor_state = {
        let database_manager = app
            .state::<Arc<crate::storage::DatabaseManager>>()
            .inner()
            .clone();
        let agent_persistence = Arc::new(crate::agent::persistence::AgentPersistence::new(
            Arc::clone(&database_manager),
        ));
        let ui_persistence = Arc::new(crate::agent::ui::AgentUiPersistence::new(Arc::clone(
            &database_manager,
        )));
        let cache = app
            .state::<Arc<crate::storage::UnifiedCache>>()
            .inner()
            .clone();

        let executor = Arc::new(crate::agent::core::TaskExecutor::with_checkpoint_service(
            Arc::clone(&database_manager),
            Arc::clone(&cache),
            Arc::clone(&agent_persistence),
            Arc::clone(&ui_persistence),
            Arc::clone(&checkpoint_service),
        ));

        crate::agent::core::commands::TaskExecutorState::new(executor)
    };
    app.manage(task_executor_state);

    let window_state = WindowState::new().map_err(SetupError::WindowState)?;
    app.manage(window_state);

    // 复用之前创建的 global_mux，不要再次调用 get_mux()
    app.manage(global_mux);

    // Manage Terminal Channel State for streaming bytes via Tauri Channel
    let terminal_channel_state = TerminalChannelState::new();
    app.manage(terminal_channel_state);

    // Initialize Dock Manager for platform-specific dock/jump list menus
    match crate::dock::DockManager::new(&app.handle()) {
        Ok(dock_manager) => {
            app.manage(dock_manager);
        }
        Err(e) => {
            tracing::warn!("Failed to initialize dock manager: {}", e);
        }
    }

    // 初始化 Checkpoint 状态（复用之前创建的 checkpoint_service）
    let checkpoint_state = {
        use crate::checkpoint::CheckpointState;
        CheckpointState::new(checkpoint_service)
    };
    app.manage(checkpoint_state);

    // 初始化向量数据库状态
    {
        use crate::llm::types::LLMProviderConfig;
        use crate::storage::repositories::{AIModels, ModelType};
        use crate::vector_db::{
            commands::VectorDbState,
            core::{RemoteEmbeddingConfig, VectorDbConfig},
            index::VectorIndex,
            search::SemanticSearchEngine,
            storage::IndexManager,
        };
        use std::path::PathBuf;
        use std::sync::Arc;

        // 复用与数据库相同的应用数据目录作为根
        let app_dir = if let Ok(dir) = std::env::var("OrbitX_DATA_DIR") {
            PathBuf::from(dir)
        } else {
            let data_dir = dirs::data_dir().ok_or("无法获取系统数据目录")?;
            data_dir.join("OrbitX")
        };

        // 从数据库读取 embedding 模型配置
        let database = app
            .state::<Arc<crate::storage::DatabaseManager>>()
            .inner()
            .clone();
        let embedding_config = tauri::async_runtime::block_on(async {
            let models = AIModels::new(&database)
                .find_all()
                .await
                .unwrap_or_default();
            models
                .into_iter()
                .find(|m| m.model_type == ModelType::Embedding)
        });

        let config =
            if let Some(model) = embedding_config {
                // 从 options 中读取维度，默认 1024
                let dimension = model
                    .options
                    .as_ref()
                    .and_then(|opts| opts.get("dimension"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize)
                    .unwrap_or(1024);

                tracing::info!(
                    "使用配置的 embedding 模型: {} @ {}, 维度: {}",
                    model.model,
                    model.api_url,
                    dimension
                );
                VectorDbConfig {
                    embedding: RemoteEmbeddingConfig {
                        provider_config: LLMProviderConfig {
                            provider_type: model.provider.as_str().to_string(),
                            api_key: model.api_key,
                            api_url: Some(model.api_url),
                            options: model.options.as_ref().and_then(|v| v.as_object()).map(
                                |obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                            ),
                        },
                        model_name: model.model,
                        dimension,
                        chunk_size: 512,
                        chunk_overlap: 100,
                    },
                    ..VectorDbConfig::default()
                }
            } else {
                tracing::warn!("未找到 embedding 模型配置，使用默认值");
                VectorDbConfig::default()
            };

        if let Err(e) = config.validate() {
            warn!("Vector DB config validate failed: {}", e);
        }

        if let Ok(state) = (|| -> Result<VectorDbState, crate::vector_db::core::VectorDbError> {
            let index_manager = Arc::new(IndexManager::new(&app_dir, config.clone())?);
            let embedder = crate::vector_db::embedding::create_embedder(&config.embedding)?;
            let vector_index = Arc::new(VectorIndex::new(config.embedding.dimension));
            let search_engine = Arc::new(SemanticSearchEngine::new(
                index_manager.clone(),
                vector_index.clone(),
                embedder.clone(),
                config,
            ));
            // 设置全局只读访问
            crate::vector_db::commands::set_global_state(
                search_engine.clone(),
                index_manager.clone(),
                vector_index.clone(),
            );
            Ok(VectorDbState::new(
                search_engine,
                index_manager,
                vector_index,
            ))
        })() {
            app.manage(state);
        } else {
            warn!("Failed to initialize vector DB");
        }
    }

    Ok(())
}

/// 设置应用程序事件和监听器
pub fn setup_app_events<R: tauri::Runtime>(app: &tauri::App<R>) {
    setup_unified_terminal_events(app.handle().clone());

    // 启动系统主题监听器
    start_system_theme_listener(app.handle().clone());

    // 配置窗口关闭行为：macOS 上隐藏窗口，其他平台退出应用
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(target_os = "macos")]
        {
            // macOS: 点击关闭按钮时隐藏窗口，应用保持在 Dock 栏运行
            // 用户可以通过 Command+Q 或菜单退出来真正退出应用
            let window_clone = window.clone();
            let app_handle = app.handle().clone();
            window.on_window_event(move |event| {
                use tauri::WindowEvent;
                if let WindowEvent::CloseRequested { api, .. } = event {
                    // 阻止默认的关闭行为
                    api.prevent_close();

                    // 清空所有标签页
                    if let Some(dock_manager) = app_handle.try_state::<crate::dock::DockManager>() {
                        if let Err(e) = dock_manager.state().clear() {
                            warn!("Failed to clear dock tabs: {}", e);
                        }
                    }

                    // 通知前端清空所有标签页
                    if let Err(e) = window_clone.emit("clear-all-tabs", ()) {
                        warn!("Failed to emit clear-all-tabs event: {}", e);
                    }

                    // 隐藏窗口而不是关闭
                    if let Err(e) = window_clone.hide() {
                        warn!("Failed to hide window: {}", e);
                    }
                }
            });
        }

        #[cfg(not(target_os = "macos"))]
        {
            // 其他平台：点击关闭按钮时退出应用并清理资源
            use tauri::WindowEvent;
            window.on_window_event(|event| {
                if let WindowEvent::CloseRequested { .. } = event {
                    if let Err(e) = crate::mux::singleton::shutdown_mux() {
                        warn!("Failed to shutdown TerminalMux: {}", e);
                    }
                }
            });
        }
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
                            warn!("Failed to parse file path: {:?}, error: {:?}", url, e);

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

    // 订阅Shell事件
    let shell_integration = mux.shell_integration();
    let shell_event_receiver = shell_integration.subscribe_events();

    match create_terminal_event_handler(
        app_handle.clone(),
        &mux,
        context_event_receiver,
        shell_event_receiver,
    ) {
        Ok(handler) => {
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
                    warn!("Failed to handle system theme change: {}", e);
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
        "catppuccin-latte.toml".to_string(),
        "catppuccin-mocha.toml".to_string(),
        "dark.toml".to_string(),
        "dracula.toml".to_string(),
        "github-dark.toml".to_string(),
        "gruvbox-dark.toml".to_string(),
        "index.toml".to_string(),
        "light.toml".to_string(),
        "material-dark.toml".to_string(),
        "nord.toml".to_string(),
        "one-dark.toml".to_string(),
        "tokyo-night.toml".to_string(),
    ]
}

/// 动态获取资源目录中的所有主题文件
async fn get_theme_files_from_resources<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    use std::path::PathBuf;
    use tauri::path::BaseDirectory;

    let themes_resource_path = if cfg!(debug_assertions) {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        current_dir.join("..").join("config").join("themes")
    } else {
        app_handle
            .path()
            .resolve("_up_/config/themes", BaseDirectory::Resource)
            .map_err(|_| "Failed to resolve resource path")?
    };

    match std::fs::read_dir(&themes_resource_path) {
        Ok(entries) => {
            let theme_files: Vec<String> = entries
                .flatten()
                .filter_map(|entry| {
                    let path = entry.path();
                    if path.is_file() {
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .filter(|name| name.ends_with(".toml"))
                            .map(String::from)
                    } else {
                        None
                    }
                })
                .collect();

            Ok(if theme_files.is_empty() {
                get_fallback_theme_list()
            } else {
                theme_files
            })
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

    if !themes_dir.exists() {
        fs::create_dir_all(themes_dir)?;
    }

    let theme_files = get_theme_files_from_resources(app_handle).await?;

    for theme_file in &theme_files {
        let dest_path = themes_dir.join(theme_file);

        let source_path = if cfg!(debug_assertions) {
            let current_dir =
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            current_dir
                .join("..")
                .join("config")
                .join("themes")
                .join(theme_file)
        } else {
            app_handle.path().resolve(
                format!("_up_/config/themes/{}", theme_file),
                BaseDirectory::Resource,
            )?
        };

        if let Ok(content) = std::fs::read_to_string(&source_path) {
            let _ = std::fs::write(&dest_path, content);
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
        .resolve("_up_/config/config.toml", BaseDirectory::Resource)
    {
        Ok(resource_path) => match fs::read_to_string(&resource_path) {
            Ok(content) => match fs::write(&config_file_path, content) {
                Ok(_) => {
                    // 成功复制默认配置文件（静默）
                }
                Err(e) => {
                    warn!("Failed to write default config file: {}", e);
                }
            },
            Err(e) => {
                warn!("Failed to read resource config file: {}", e);
            }
        },
        Err(e) => {
            warn!("Failed to resolve config file resource path: {}", e);
        }
    }

    Ok(())
}

/// 创建一个 Tauri 插件，用于应用初始化
pub fn init_plugin<R: tauri::Runtime>(name: &'static str) -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new(name).build()
}
