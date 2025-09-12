/*!
 * 配置系统 Tauri 命令接口
 *
 * 提供前端调用的配置管理命令的简化实现。
 */

use crate::config::{defaults::create_default_config, types::AppConfig, TomlConfigManager};

use crate::utils::error::AppResult;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use tracing::debug;

/// 配置管理器状态
pub struct ConfigManagerState {
    /// TOML配置管理器
    pub toml_manager: Arc<TomlConfigManager>,
    /// 主题管理器占位符
    pub theme_manager: Mutex<Option<()>>,
}

impl ConfigManagerState {
    /// 创建新的配置管理器状态
    pub async fn new() -> AppResult<Self> {
        let toml_manager = Arc::new(TomlConfigManager::new().await?);
        toml_manager.load_config().await?;

        Ok(Self {
            toml_manager,
            theme_manager: Mutex::new(None),
        })
    }
}

/// 获取当前配置
#[tauri::command]
pub async fn config_get(state: State<'_, ConfigManagerState>) -> TauriApiResult<AppConfig> {
    match state.toml_manager.config_get().await {
        Ok(config) => Ok(api_success!(config)),
        Err(_) => Ok(api_error!("config.get_failed")),
    }
}

/// 更新配置
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

/// 保存配置（强制保存当前缓存的配置到文件）
#[tauri::command]
pub async fn config_save(state: State<'_, ConfigManagerState>) -> TauriApiResult<EmptyData> {
    // 这个命令主要用于强制保存当前缓存的配置到文件
    // 使用 config_update 确保原子性操作
    match state
        .toml_manager
        .config_update(|_config| {
            // 不修改配置，只是触发保存操作
            Ok(())
        })
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.save_failed")),
    }
}

/// 验证配置
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

/// 重置配置为默认值
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

/// 获取配置文件路径
#[tauri::command]
pub async fn config_get_file_path(_state: State<'_, ConfigManagerState>) -> TauriApiResult<String> {
    Ok(api_success!("config/config.toml".to_string()))
}

/// 配置文件信息
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ConfigFileInfo {
    pub path: String,
    pub exists: bool,
}

/// 获取配置文件信息
#[tauri::command]
pub async fn config_get_file_info(
    _state: State<'_, ConfigManagerState>,
) -> TauriApiResult<ConfigFileInfo> {
    Ok(api_success!(ConfigFileInfo {
        path: "config/config.toml".to_string(),
        exists: true,
    }))
}

/// 打开配置文件
#[tauri::command]
pub async fn config_open_file<R: tauri::Runtime>(
    _app: tauri::AppHandle<R>,
    _state: State<'_, ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    debug!("打开配置文件功能需要重新实现");
    Ok(api_success!())
}

/// 订阅配置事件
#[tauri::command]
pub async fn config_subscribe_events(
    _state: State<'_, ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    debug!("订阅配置事件");
    Ok(api_success!())
}

/// 获取配置文件夹路径
#[tauri::command]
pub async fn config_get_folder_path(
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<String> {
    debug!("获取配置文件夹路径");

    // 通过toml_manager获取配置路径
    let config_path = state.toml_manager.get_config_path().await;

    // 获取配置文件的父目录（即配置文件夹）
    if let Some(config_dir) = config_path.parent() {
        Ok(api_success!(config_dir.to_string_lossy().to_string()))
    } else {
        Ok(api_error!("config.get_folder_path_failed"))
    }
}

/// 打开配置文件夹
#[tauri::command]
pub async fn config_open_folder<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    debug!("打开配置文件夹");

    // 通过toml_manager获取配置路径
    let config_path = state.toml_manager.get_config_path().await;

    // 获取配置文件的父目录（即配置文件夹）
    let config_dir = if let Some(dir) = config_path.parent() {
        dir
    } else {
        return Ok(api_error!("config.get_folder_path_failed"));
    };

    // 确保配置目录存在
    if !config_dir.exists() {
        return Ok(api_error!("config.get_folder_path_failed"));
    }

    // 使用 tauri-plugin-opener 打开文件夹
    use tauri_plugin_opener::OpenerExt;

    match app
        .opener()
        .open_path(config_dir.to_string_lossy().to_string(), None::<String>)
    {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.open_folder_failed")),
    }
}
