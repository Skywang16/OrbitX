use crate::config::defaults::create_default_config;
use crate::config::manager::ConfigManager;
use crate::config::types::AppConfig;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn config_get(state: State<'_, Arc<ConfigManager>>) -> TauriApiResult<AppConfig> {
    match state.config_get().await {
        Ok(config) => Ok(api_success!(config)),
        Err(_) => Ok(api_error!("config.get_failed")),
    }
}

#[tauri::command]
pub async fn config_set(
    new_config: AppConfig,
    state: State<'_, Arc<ConfigManager>>,
) -> TauriApiResult<EmptyData> {
    match state.config_set(new_config).await {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.update_failed")),
    }
}

#[tauri::command]
pub async fn config_reset_to_defaults(
    state: State<'_, Arc<ConfigManager>>,
) -> TauriApiResult<EmptyData> {
    let default_config = create_default_config();
    match state.config_set(default_config).await {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.reset_failed")),
    }
}

#[tauri::command]
pub async fn config_open_folder<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, Arc<ConfigManager>>,
) -> TauriApiResult<EmptyData> {
    let config_path = state.path().to_path_buf();

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
