use crate::config::error::ConfigResult;
use crate::config::{defaults::create_default_config, types::AppConfig, TomlConfigManager};
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::Mutex;

/// 配置管理器状态
pub struct ConfigManagerState {
    pub toml_manager: Arc<TomlConfigManager>,
    pub theme_manager: Mutex<Option<()>>,
}

impl ConfigManagerState {
    pub async fn new() -> ConfigResult<Self> {
        let toml_manager = Arc::new(TomlConfigManager::new().await?);
        toml_manager.load_config().await?;

        Ok(Self {
            toml_manager,
            theme_manager: Mutex::new(None),
        })
    }
}

#[tauri::command]
pub async fn config_get(state: State<'_, ConfigManagerState>) -> TauriApiResult<AppConfig> {
    match state.toml_manager.config_get().await {
        Ok(config) => Ok(api_success!(config)),
        Err(_) => Ok(api_error!("config.get_failed")),
    }
}

#[tauri::command]
pub async fn config_set(
    new_config: AppConfig,
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    match state
        .toml_manager
        .config_update(|config| {
            *config = new_config.clone();
            Ok(())
        })
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.update_failed")),
    }
}

#[tauri::command]
pub async fn config_reset_to_defaults(
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    let default_config = create_default_config();
    match state
        .toml_manager
        .config_update(|config| {
            *config = default_config.clone();
            Ok(())
        })
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.reset_failed")),
    }
}

#[tauri::command]
pub async fn config_open_folder<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    let config_path = state.toml_manager.get_config_path().await;

    let config_dir = if let Some(dir) = config_path.parent() {
        dir
    } else {
        return Ok(api_error!("config.get_folder_path_failed"));
    };

    if !config_dir.exists() {
        return Ok(api_error!("config.get_folder_path_failed"));
    }

    use tauri_plugin_opener::OpenerExt;

    match app
        .opener()
        .open_path(config_dir.to_string_lossy().to_string(), None::<String>)
    {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.open_folder_failed")),
    }
}
