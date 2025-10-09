use crate::config::error::ConfigResult;
use crate::config::{defaults::create_default_config, types::AppConfig, TomlConfigManager};
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use tracing::debug;

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
pub async fn config_update(
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
pub async fn config_save(state: State<'_, ConfigManagerState>) -> TauriApiResult<EmptyData> {
    match state.toml_manager.config_update(|_config| Ok(())).await {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.save_failed")),
    }
}

#[tauri::command]
pub async fn config_validate(state: State<'_, ConfigManagerState>) -> TauriApiResult<EmptyData> {
    debug!("开始验证配置");
    let config = match state.toml_manager.config_get().await {
        Ok(config) => config,
        Err(_) => return Ok(api_error!("config.get_failed")),
    };
    match state.toml_manager.config_validate(&config) {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.validate_failed")),
    }
}

#[tauri::command]
pub async fn config_reset_to_defaults(
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    debug!("开始重置配置为默认值");
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
pub async fn config_get_file_path() -> TauriApiResult<String> {
    Ok(api_success!("config/config.toml".to_string()))
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ConfigFileInfo {
    pub path: String,
    pub exists: bool,
}

#[tauri::command]
pub async fn config_get_file_info() -> TauriApiResult<ConfigFileInfo> {
    Ok(api_success!(ConfigFileInfo {
        path: "config/config.toml".to_string(),
        exists: true,
    }))
}

#[tauri::command]
pub async fn config_open_file() -> TauriApiResult<EmptyData> {
    debug!("打开配置文件功能需要重新实现");
    Ok(api_success!())
}

#[tauri::command]
pub async fn config_subscribe_events() -> TauriApiResult<EmptyData> {
    debug!("订阅配置事件");
    Ok(api_success!())
}

#[tauri::command]
pub async fn config_get_folder_path(
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<String> {
    debug!("获取配置文件夹路径");

    let config_path = state.toml_manager.get_config_path().await;

    if let Some(config_dir) = config_path.parent() {
        Ok(api_success!(config_dir.to_string_lossy().to_string()))
    } else {
        Ok(api_error!("config.get_folder_path_failed"))
    }
}

#[tauri::command]
pub async fn config_open_folder<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    debug!("打开配置文件夹");

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
