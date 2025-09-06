/*!
 * 配置系统 Tauri 命令接口
 *
 * 提供前端调用的配置管理命令的简化实现。
 */

use crate::config::{defaults::create_default_config, types::AppConfig, TomlConfigManager};

use crate::utils::error::{AppResult, TauriResult, ToTauriResult};
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
pub async fn get_config(state: State<'_, ConfigManagerState>) -> TauriResult<AppConfig> {
    state.toml_manager.get_config().await.to_tauri()
}

/// 更新配置
#[tauri::command]
pub async fn update_config(
    new_config: AppConfig,
    state: State<'_, ConfigManagerState>,
) -> TauriResult<()> {
    state
        .toml_manager
        .update_config(|config| {
            *config = new_config.clone();
            Ok(())
        })
        .await
        .to_tauri()
}

/// 保存配置（强制保存当前缓存的配置到文件）
#[tauri::command]
pub async fn save_config(state: State<'_, ConfigManagerState>) -> TauriResult<()> {
    // 这个命令主要用于强制保存当前缓存的配置到文件
    // 使用 update_config 确保原子性操作
    state
        .toml_manager
        .update_config(|_config| {
            // 不修改配置，只是触发保存操作
            Ok(())
        })
        .await
        .to_tauri()
}

/// 验证配置
#[tauri::command]
pub async fn validate_config(state: State<'_, ConfigManagerState>) -> TauriResult<()> {
    debug!("开始验证配置");
    let config = state.toml_manager.get_config().await.to_tauri()?;
    state.toml_manager.validate_config(&config).to_tauri()
}

/// 重置配置为默认值
#[tauri::command]
pub async fn reset_config_to_defaults(state: State<'_, ConfigManagerState>) -> TauriResult<()> {
    debug!("开始重置配置为默认值");
    let default_config = create_default_config();
    state
        .toml_manager
        .update_config(|config| {
            *config = default_config.clone();
            Ok(())
        })
        .await
        .to_tauri()
}

/// 获取配置文件路径
#[tauri::command]
pub async fn get_config_file_path(_state: State<'_, ConfigManagerState>) -> TauriResult<String> {
    Ok("config/config.toml".to_string())
}

/// 配置文件信息
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ConfigFileInfo {
    pub path: String,
    pub exists: bool,
}

/// 获取配置文件信息
#[tauri::command]
pub async fn get_config_file_info(
    _state: State<'_, ConfigManagerState>,
) -> TauriResult<ConfigFileInfo> {
    Ok(ConfigFileInfo {
        path: "config/config.toml".to_string(),
        exists: true,
    })
}

/// 打开配置文件
#[tauri::command]
pub async fn open_config_file<R: tauri::Runtime>(
    _app: tauri::AppHandle<R>,
    _state: State<'_, ConfigManagerState>,
) -> TauriResult<()> {
    debug!("打开配置文件功能需要重新实现");
    Ok(())
}

/// 订阅配置事件
#[tauri::command]
pub async fn subscribe_config_events(_state: State<'_, ConfigManagerState>) -> TauriResult<()> {
    debug!("订阅配置事件");
    Ok(())
}

/// 获取配置文件夹路径
#[tauri::command]
pub async fn get_config_folder_path(
    state: State<'_, ConfigManagerState>,
) -> TauriResult<String> {
    debug!("获取配置文件夹路径");

    // 通过toml_manager获取配置路径
    let config_path = state.toml_manager.get_config_path().await;

    // 获取配置文件的父目录（即配置文件夹）
    if let Some(config_dir) = config_path.parent() {
        Ok(config_dir.to_string_lossy().to_string())
    } else {
        Err("无法获取配置文件夹路径".to_string())
    }
}

/// 打开配置文件夹
#[tauri::command]
pub async fn open_config_folder<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, ConfigManagerState>,
) -> TauriResult<()> {
    debug!("打开配置文件夹");

    // 通过toml_manager获取配置路径
    let config_path = state.toml_manager.get_config_path().await;

    // 获取配置文件的父目录（即配置文件夹）
    let config_dir = if let Some(dir) = config_path.parent() {
        dir
    } else {
        return Err("无法获取配置文件夹路径".to_string());
    };

    // 确保配置目录存在
    if !config_dir.exists() {
        return Err("配置目录不存在".to_string());
    }

    // 使用 tauri-plugin-opener 打开文件夹
    use tauri_plugin_opener::OpenerExt;

    app.opener()
        .open_path(config_dir.to_string_lossy().to_string(), None::<String>)
        .map_err(|e| format!("无法打开配置文件夹: {}", e))?;

    Ok(())
}
