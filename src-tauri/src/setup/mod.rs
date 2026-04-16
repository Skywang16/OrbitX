//! Application initialization

pub mod error;

pub use error::{SetupError, SetupResult};

use crate::ai::tool::shell::TerminalState;
use crate::ai::AIManagerState;
use crate::config::{ConfigManager, ShortcutManagerState};
use crate::llm::commands::LLMManagerState;
use crate::settings::SettingsManager;
use crate::terminal::{
    commands::TerminalContextState, ActiveTerminalContextRegistry, TerminalChannelState,
    TerminalContextService,
};
use crate::window::commands::AppWindowState;

use std::sync::Arc;
use tauri::{Emitter, Manager};
use tracing::warn;
use tracing_subscriber::{self, EnvFilter};

fn resolve_app_data_dir() -> SetupResult<std::path::PathBuf> {
    use std::env;

    match env::var("ORBITX_DATA_DIR") {
        Ok(dir) => {
            let trimmed = dir.trim();
            if trimmed.is_empty() {
                return Err(SetupError::Environment(
                    "ORBITX_DATA_DIR is set but empty".to_string(),
                ));
            }
            Ok(std::path::PathBuf::from(trimmed))
        }
        Err(env::VarError::NotPresent) => {
            let data_dir = dirs::data_dir().ok_or_else(|| {
                SetupError::Environment("system data_dir unavailable".to_string())
            })?;
            Ok(data_dir.join("OrbitX"))
        }
        Err(err) => Err(SetupError::Environment(format!(
            "Failed to read ORBITX_DATA_DIR: {err}"
        ))),
    }
}

pub fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        #[cfg(debug_assertions)]
        let default_level =
            "debug,ignore=warn,globset=warn,hyper_util=info,hyper=info,reqwest=info,html5ever=warn,markup5ever=warn,selectors=warn";
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
            eprintln!("Log system initialization failed: {e}");
            std::process::exit(1);
        }
    }
}

/// Initialize all application state managers
pub fn initialize_app_states<R: tauri::Runtime>(app: &tauri::App<R>) -> SetupResult<()> {
    let terminal_state = TerminalState::new().map_err(SetupError::TerminalState)?;
    app.manage(terminal_state);

    let paths = crate::config::paths::ConfigPaths::new()?;
    app.manage(paths);

    // Ensure config.json exists before ConfigManager initialization (no migration: only write default template if missing)
    tauri::async_runtime::block_on(async {
        if let Err(err) = copy_config_from_resources(app.handle()).await {
            warn!("Failed to copy default config from resources: {}", err);
        }
    });

    let config_manager = Arc::new(tauri::async_runtime::block_on(async {
        ConfigManager::new().await
    })?);
    app.manage(Arc::clone(&config_manager));

    let shortcut_state = tauri::async_runtime::block_on(async {
        ShortcutManagerState::new(Arc::clone(&config_manager)).await
    })?;
    app.manage(shortcut_state);

    // Initialize SettingsManager (settings.json / workspace .orbitx/settings.json)
    app.manage(Arc::new(SettingsManager::new()?));
    // Initialize MCP Registry (cache MCP clients by workspace)
    app.manage(Arc::new(crate::agent::mcp::McpRegistry::default()));
    app.manage(Arc::new(crate::lsp::LspManager::new()));

    // Initialize DatabaseManager
    let database_manager = {
        use crate::storage::{DatabaseManager, StoragePaths};

        let app_dir = resolve_app_data_dir()?;

        let paths = StoragePaths::new(app_dir)?;
        let options = crate::storage::DatabaseOptions::default();

        Arc::new(tauri::async_runtime::block_on(async {
            let db = DatabaseManager::new(paths.clone(), options).await?;
            db.initialize().await?;
            Ok::<_, SetupError>(db)
        })?)
    };
    app.manage(database_manager.clone());

    // Initialize UnifiedCache
    let cache = Arc::new(crate::storage::cache::UnifiedCache::new());
    app.manage(cache.clone());

    // Copy theme files before ThemeManager initialization
    tauri::async_runtime::block_on(async {
        if let Err(err) = copy_themes_from_resources(app.handle()).await {
            warn!("Failed to copy themes from resources: {}", err);
        }
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

    // Create Shell Integration and register Node version callback
    let shell_integration = Arc::new(crate::shell::ShellIntegrationManager::new());

    // TODO: Node version change events have been migrated to IoHandler processing
    // If frontend notification is needed, add MuxNotification::NodeVersionChanged type

    // Initialize global Mux
    let global_mux =
        crate::mux::singleton::init_mux_with_shell_integration(shell_integration.clone())
            .map_err(|err| SetupError::TerminalState(err.to_string()))?;

    let terminal_context_state = {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let cache = app
            .state::<Arc<crate::storage::cache::UnifiedCache>>()
            .inner()
            .clone();

        // Enable context service integration with ShellIntegration (callbacks, cache invalidation, event forwarding)
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

    // Initialize OAuthManager (OAuth authorization manager)
    let oauth_manager = {
        let database = app
            .state::<Arc<crate::storage::DatabaseManager>>()
            .inner()
            .clone();
        Arc::new(crate::llm::oauth::OAuthManager::new(database))
    };
    app.manage(oauth_manager);

    // Initialize Checkpoint service (create early for agent run executor use)
    let checkpoint_service = {
        use crate::checkpoint::{
            BlobStore, CheckpointConfig, CheckpointService, CheckpointStorage,
        };

        let database = app
            .state::<Arc<crate::storage::DatabaseManager>>()
            .inner()
            .clone();
        let pool = database.pool().clone();

        let config = CheckpointConfig::default();
        let storage = Arc::new(CheckpointStorage::new(pool.clone()));
        let blob_store = Arc::new(BlobStore::new(pool, config.clone()));
        Arc::new(CheckpointService::with_config(storage, blob_store, config))
    };

    // Initialize workspace change journal (for injecting "user/external changes" into Agent prompts)
    let workspace_changes =
        std::sync::Arc::new(crate::agent::workspace_changes::WorkspaceChangeJournal::new());
    app.manage(std::sync::Arc::clone(&workspace_changes));

    let watcher = std::sync::Arc::new(
        crate::file_watcher::UnifiedFileWatcher::new().with_fs_sink(workspace_changes.fs_sender()),
    );
    app.manage(std::sync::Arc::clone(&watcher));

    // Initialize vector database state (and inject search_engine into the agent run executor for semantic_search)
    let vector_search_engine = {
        use crate::vector_db::commands::VectorDbState;
        use std::sync::Arc;

        let database = app
            .state::<Arc<crate::storage::DatabaseManager>>()
            .inner()
            .clone();
        match tauri::async_runtime::block_on(crate::vector_db::build_search_engine_from_database(
            database,
        )) {
            Ok(search_engine) => {
                let state = VectorDbState::new(Arc::clone(&search_engine));
                app.manage(state);
                Some(search_engine)
            }
            Err(e) => {
                warn!("Failed to initialize vector DB: {}", e);
                app.manage(VectorDbState::empty());
                None
            }
        }
    };

    // Initialize agent run executor state (with Checkpoint service)
    let agent_run_executor_state = {
        let database_manager = app
            .state::<Arc<crate::storage::DatabaseManager>>()
            .inner()
            .clone();
        let agent_persistence = Arc::new(crate::agent::persistence::AgentPersistence::new(
            Arc::clone(&database_manager),
        ));
        let cache = app
            .state::<Arc<crate::storage::UnifiedCache>>()
            .inner()
            .clone();
        let settings_manager = app
            .state::<Arc<crate::settings::SettingsManager>>()
            .inner()
            .clone();
        let mcp_registry = app
            .state::<Arc<crate::agent::mcp::McpRegistry>>()
            .inner()
            .clone();
        let lsp_manager = app.state::<Arc<crate::lsp::LspManager>>().inner().clone();

        let executor = Arc::new(
            crate::agent::core::AgentRunExecutor::with_checkpoint_service(
                crate::agent::core::executor::AgentRunExecutorServices {
                    database: Arc::clone(&database_manager),
                    cache: Arc::clone(&cache),
                    agent_persistence: Arc::clone(&agent_persistence),
                    settings_manager,
                    mcp_registry,
                    lsp_manager,
                    checkpoint_service: Some(Arc::clone(&checkpoint_service)),
                    workspace_changes: std::sync::Arc::clone(&workspace_changes),
                    vector_search_engine,
                },
            ),
        );

        crate::agent::core::commands::AgentRunExecutorState::new(executor)
    };
    app.manage(agent_run_executor_state);

    let window_state = AppWindowState::new().map_err(SetupError::WindowState)?;
    app.manage(window_state);

    // Reuse the previously created global_mux, don't call get_mux() again
    app.manage(global_mux);

    // Manage Terminal Channel State for streaming bytes via Tauri Channel
    let terminal_channel_state = TerminalChannelState::new();
    app.manage(terminal_channel_state);

    // Initialize Checkpoint state (reuse the previously created checkpoint_service)
    let checkpoint_state = {
        use crate::checkpoint::CheckpointState;
        CheckpointState::new(checkpoint_service)
    };
    app.manage(checkpoint_state);

    Ok(())
}

/// Setup application events and listeners
pub fn setup_app_events<R: tauri::Runtime>(app: &tauri::App<R>) {
    setup_unified_terminal_events(app.handle().clone());
    crate::agent::terminal::AgentTerminalManager::init();

    // Start system theme listener
    start_system_theme_listener(app.handle().clone());

    // Configure window close behavior: hide window on macOS, exit app on other platforms
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(target_os = "macos")]
        {
            // macOS: hide window when close button is clicked, app stays running in Dock
            // User can truly exit app via Command+Q or menu exit
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                use tauri::WindowEvent;
                if let WindowEvent::CloseRequested { api, .. } = event {
                    // Prevent default close behavior
                    api.prevent_close();

                    // Hide window instead of closing
                    if let Err(e) = window_clone.hide() {
                        warn!("Failed to hide window: {}", e);
                    }
                }
            });
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Other platforms: exit app and clean up resources when close button is clicked
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

/// Setup deep link handling
pub fn setup_deep_links<R: tauri::Runtime>(app: &tauri::App<R>) {
    #[cfg(desktop)]
    {
        use tauri_plugin_deep_link::DeepLinkExt;

        let app_handle = app.handle().clone();
        app.deep_link().on_open_url(move |event| {
            let urls = event.urls();
            for url in urls {
                if url.scheme() == "file" {
                    // Use url.to_file_path() method, which correctly handles Chinese characters
                    match url.to_file_path() {
                        Ok(path_buf) => {
                            let path_str = path_buf.to_string_lossy().to_string();

                            // Send to frontend
                            if let Some(window) = app_handle.get_webview_window("main") {
                                if let Err(err) = window.emit("file-dropped", path_str) {
                                    warn!("Failed to emit deep-link file event: {}", err);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse file path: {:?}, error: {:?}", url, e);

                            // Fallback: manually decode URL path
                            let file_path = url.path();
                            match urlencoding::decode(file_path) {
                                Ok(decoded_path) => {
                                    let path_str = decoded_path.to_string();

                                    if let Some(window) = app_handle.get_webview_window("main") {
                                        if let Err(err) = window.emit("file-dropped", path_str) {
                                            warn!(
                                                "Failed to emit fallback deep-link file event: {}",
                                                err
                                            );
                                        }
                                    }
                                }
                                Err(err) => {
                                    warn!("Failed to decode fallback deep-link path: {}", err);
                                }
                            }
                        }
                    }
                }
            }
        });

        // Register runtime deep links (development and Linux only)
        #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
        {
            if let Err(e) = app.deep_link().register_all() {
                warn!("Failed to register deep links: {}", e);
            }
        }
    }
}

/// Handle command line arguments at startup
pub fn handle_startup_args<R: tauri::Runtime>(app: &tauri::App<R>) {
    let env = app.env();
    if env.args_os.len() > 1 {
        let file_path = &env.args_os[1];
        if let Some(window) = app.get_webview_window("main") {
            let path_str = file_path.to_string_lossy().to_string();
            if let Err(err) = window.emit("startup-file", path_str) {
                warn!("Failed to emit startup file event: {}", err);
            }
        }
    }
}

/// Ensure main window is displayed correctly
pub fn ensure_main_window_visible<R: tauri::Runtime>(app: &tauri::App<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let window_clone = window.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            match window_clone.outer_position() {
                Ok(position) => {
                    let x = position.x;
                    let y = position.y;

                    if x < -500 || y < -500 || x > 5000 || y > 5000 {
                        if let Err(err) = window_clone.set_position(tauri::Position::Logical(
                            tauri::LogicalPosition { x: 100.0, y: 100.0 },
                        )) {
                            warn!("Failed to reset off-screen window position: {}", err);
                        }
                    }
                }
                Err(err) => {
                    warn!("Failed to inspect main window position: {}", err);
                }
            }

            if let Err(err) = window_clone.show() {
                warn!("Failed to show main window: {}", err);
            }
            if let Err(err) = window_clone.set_focus() {
                warn!("Failed to focus main window: {}", err);
            }
        });
    }
}

/// Setup unified terminal event handler
fn setup_unified_terminal_events<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>) {
    use crate::mux::singleton::get_mux;
    use crate::terminal::create_terminal_event_handler;

    let mux = get_mux();

    let terminal_context_state = app_handle.state::<TerminalContextState>();
    let registry = terminal_context_state.registry();

    // Subscribe to context events
    let context_event_receiver = registry.subscribe_events();

    // Subscribe to Shell events
    let shell_integration = mux.shell_integration();
    let shell_event_receiver = shell_integration.subscribe_events();

    match create_terminal_event_handler(
        app_handle.clone(),
        mux,
        context_event_receiver,
        shell_event_receiver,
    ) {
        Ok(handler) => {
            if app_handle
                .try_state::<crate::terminal::event_handler::TerminalEventHandler>()
                .is_none()
            {
                app_handle.manage(handler);
            } else {
                tracing::warn!("Terminal event handler state was already registered");
            }
        }
        Err(e) => {
            tracing::error!("Failed to start unified terminal event handler: {}", e);
        }
    }
}

/// Start system theme listener
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
                    // System theme updated (silent)
                }
            });
        }
    });

    // Store listener handle to prevent drop
    // Note: In actual applications, you may need to stop the listener when the app closes
}

/// Get fallback theme file list
fn get_fallback_theme_list() -> Vec<String> {
    vec![
        "catppuccin-latte.json".to_string(),
        "catppuccin-mocha.json".to_string(),
        "dark.json".to_string(),
        "dracula.json".to_string(),
        "github-dark.json".to_string(),
        "gruvbox-dark.json".to_string(),
        "light.json".to_string(),
        "material-dark.json".to_string(),
        "nord.json".to_string(),
        "one-dark.json".to_string(),
        "tokyo-night.json".to_string(),
    ]
}

fn resolve_debug_resource_root() -> Result<std::path::PathBuf, std::io::Error> {
    Ok(std::env::current_dir()?.join("..").join("config"))
}

/// Dynamically get all theme files from resource directory
async fn get_theme_files_from_resources<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    use tauri::path::BaseDirectory;

    let themes_resource_path = if cfg!(debug_assertions) {
        match resolve_debug_resource_root() {
            Ok(resource_root) => resource_root.join("themes"),
            Err(err) => {
                warn!(
                    "Failed to resolve debug theme resource root: {}. Falling back to bundled theme list.",
                    err
                );
                return Ok(get_fallback_theme_list());
            }
        }
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
                            .filter(|name| name.ends_with(".json"))
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
        Err(err) => {
            warn!(
                "Failed to read theme resource directory '{}': {}. Falling back to bundled theme list.",
                themes_resource_path.display(),
                err
            );
            Ok(get_fallback_theme_list())
        }
    }
}

/// Copy theme files from resource directory to user configuration directory
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
            let resource_root = resolve_debug_resource_root().map_err(|err| {
                format!("Failed to resolve debug theme resource root for '{theme_file}': {err}")
            })?;
            resource_root.join("themes").join(theme_file)
        } else {
            app_handle.path().resolve(
                format!("_up_/config/themes/{theme_file}"),
                BaseDirectory::Resource,
            )?
        };

        match std::fs::read_to_string(&source_path) {
            Ok(content) => {
                if let Err(err) = std::fs::write(&dest_path, content) {
                    warn!(
                        "Failed to write theme '{}' to '{}': {}",
                        theme_file,
                        dest_path.display(),
                        err
                    );
                }
            }
            Err(err) => {
                warn!(
                    "Failed to read theme resource '{}' from '{}': {}",
                    theme_file,
                    source_path.display(),
                    err
                );
            }
        }
    }

    Ok(())
}

async fn copy_config_from_resources<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::fs;
    use tauri::path::BaseDirectory;

    let app_dir = resolve_app_data_dir()
        .map_err(|err| -> Box<dyn std::error::Error + Send + Sync> { Box::new(err) })?;

    fs::create_dir_all(&app_dir)?;

    let dest_path = app_dir.join(crate::config::CONFIG_FILE_NAME);
    if dest_path.exists() {
        return Ok(());
    }

    let source_path = if cfg!(debug_assertions) {
        match resolve_debug_resource_root() {
            Ok(resource_root) => resource_root.join("config.json"),
            Err(err) => {
                warn!(
                    "Failed to resolve debug config resource root: {}. Falling back to compiled defaults.",
                    err
                );
                let default_config = crate::config::defaults::create_default_config();
                let json = serde_json::to_string_pretty(&default_config)?;
                std::fs::write(&dest_path, format!("{json}\n"))?;
                return Ok(());
            }
        }
    } else {
        app_handle
            .path()
            .resolve("_up_/config/config.json", BaseDirectory::Resource)?
    };

    match std::fs::read_to_string(&source_path) {
        Ok(content) => {
            std::fs::write(&dest_path, content)?;
            return Ok(());
        }
        Err(err) => {
            warn!(
                "Failed to read config template from '{}': {}. Falling back to compiled defaults.",
                source_path.display(),
                err
            );
        }
    }

    // Fallback: serialize the compiled defaults.
    let default_config = crate::config::defaults::create_default_config();
    let json = serde_json::to_string_pretty(&default_config)?;
    std::fs::write(&dest_path, format!("{json}\n"))?;
    Ok(())
}

/// Create a Tauri plugin for application initialization
pub fn init_plugin<R: tauri::Runtime>(name: &'static str) -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new(name).build()
}
