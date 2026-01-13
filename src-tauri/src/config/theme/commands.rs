/*!
 * 主题相关的 Tauri 命令
 *
 * 提供给前端调用的主题管理接口，包括获取当前主题、
 * 切换主题、获取主题列表等功能。
 */

use super::service::{SystemThemeDetector, ThemeService};
use super::types::{Theme, ThemeConfig};
use crate::config::error::{ConfigCommandError, ConfigCommandResult};
use crate::config::ConfigManager;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

/// 主题信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeInfo {
    /// 主题名称
    pub name: String,

    /// 主题类型
    pub theme_type: String,

    /// 是否为当前主题
    pub is_current: bool,
}

/// 主题配置状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeConfigStatus {
    /// 当前使用的主题名称
    pub current_theme_name: String,

    /// 主题配置
    pub theme_config: ThemeConfig,

    /// 系统是否为深色模式
    pub is_system_dark: Option<bool>,

    /// 所有可用主题
    pub available_themes: Vec<ThemeInfo>,
}

/// 获取当前主题配置状态
#[tauri::command]
pub async fn theme_get_config_status(
    config_manager: State<'_, Arc<ConfigManager>>,
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriApiResult<ThemeConfigStatus> {
    let config = match config_manager.config_get().await {
        Ok(config) => config,
        Err(_) => return Ok(api_error!("config.get_failed")),
    };

    let theme_config = &config.appearance.theme_config;
    let is_system_dark = SystemThemeDetector::is_dark_mode();

    let current_theme_name = theme_service.get_current_theme_name(theme_config, is_system_dark);

    let theme_list = match theme_service.theme_manager().list_themes().await {
        Ok(list) => list,
        Err(_) => return Ok(api_error!("config.get_failed")),
    };

    let available_themes = theme_list
        .into_iter()
        .map(|theme_entry| ThemeInfo {
            name: theme_entry.name.clone(),
            theme_type: theme_entry.theme_type,
            is_current: theme_entry.name == current_theme_name,
        })
        .collect();

    Ok(api_success!(ThemeConfigStatus {
        current_theme_name,
        theme_config: theme_config.clone(),
        is_system_dark,
        available_themes,
    }))
}

/// 获取当前主题数据
#[tauri::command]
pub async fn theme_get_current(
    config_manager: State<'_, Arc<ConfigManager>>,
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriApiResult<Theme> {
    let config = match config_manager.config_get().await {
        Ok(config) => config,
        Err(_) => return Ok(api_error!("config.get_failed")),
    };

    let theme_config = &config.appearance.theme_config;
    let is_system_dark = SystemThemeDetector::is_dark_mode();

    match theme_service
        .load_current_theme(theme_config, is_system_dark)
        .await
    {
        Ok(theme) => Ok(api_success!(theme)),
        Err(_) => Ok(api_error!("config.get_failed")),
    }
}

/// 设置终端主题（手动模式）
#[tauri::command]
pub async fn theme_set_terminal<R: Runtime>(
    theme_name: String,
    app_handle: AppHandle<R>,
    config_manager: State<'_, Arc<ConfigManager>>,
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriApiResult<EmptyData> {
    // 验证主题是否存在
    if !theme_service.theme_exists(&theme_name).await {
        return Ok(api_error!("common.not_found"));
    }

    // 更新配置
    if let Err(_) = config_manager
        .config_update(|config| {
            config.appearance.theme_config.terminal_theme = theme_name.clone();
            config.appearance.theme_config.follow_system = false; // 切换到手动模式
            Ok(())
        })
        .await
    {
        return Ok(api_error!("config.update_failed"));
    }

    // 发送主题变化事件，确保前端能立即响应
    if let Err(_) = app_handle.emit("theme-changed", &theme_name) {
        return Ok(api_error!("config.update_failed"));
    }

    Ok(api_success!())
}

/// 设置跟随系统主题
#[tauri::command]
pub async fn theme_set_follow_system<R: Runtime>(
    follow_system: bool,
    light_theme: Option<String>,
    dark_theme: Option<String>,
    app_handle: AppHandle<R>,
    config_manager: State<'_, Arc<ConfigManager>>,
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriApiResult<EmptyData> {
    // 验证主题是否存在
    if let Some(ref light) = light_theme {
        if !theme_service.theme_exists(light).await {
            return Ok(api_error!("common.not_found"));
        }
    }

    if let Some(ref dark) = dark_theme {
        if !theme_service.theme_exists(dark).await {
            return Ok(api_error!("common.not_found"));
        }
    }

    // 更新配置
    if let Err(_) = config_manager
        .config_update(|config| {
            config.appearance.theme_config.follow_system = follow_system;

            if let Some(light) = light_theme {
                config.appearance.theme_config.light_theme = light;
            }

            if let Some(dark) = dark_theme {
                config.appearance.theme_config.dark_theme = dark;
            }

            Ok(())
        })
        .await
    {
        return Ok(api_error!("config.update_failed"));
    }

    if follow_system {
        let config = match config_manager.config_get().await {
            Ok(config) => config,
            Err(_) => return Ok(api_error!("config.get_failed")),
        };
        let is_system_dark = SystemThemeDetector::is_dark_mode();
        let current_theme_name =
            theme_service.get_current_theme_name(&config.appearance.theme_config, is_system_dark);

        // 发送主题变化事件
        if let Err(_) = app_handle.emit("theme-changed", &current_theme_name) {
            return Ok(api_error!("config.update_failed"));
        }
    }

    Ok(api_success!())
}

/// 获取所有可用主题列表
#[tauri::command]
pub async fn theme_get_available(
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriApiResult<Vec<ThemeInfo>> {
    let theme_list = match theme_service.theme_manager().list_themes().await {
        Ok(list) => list,
        Err(_) => return Ok(api_error!("config.get_failed")),
    };

    let themes = theme_list
        .into_iter()
        .map(|theme_entry| ThemeInfo {
            name: theme_entry.name,
            theme_type: theme_entry.theme_type,
            is_current: false, // 这里不设置当前状态，由前端决定
        })
        .collect();

    Ok(api_success!(themes))
}

/// 系统主题变化处理
pub async fn handle_system_theme_change<R: tauri::Runtime>(
    app_handle: &AppHandle<R>,
    is_dark: bool,
) -> ConfigCommandResult<()> {
    let config_manager = app_handle.state::<Arc<ConfigManager>>();
    let theme_service = app_handle.state::<Arc<ThemeService>>();

    let config = config_manager
        .config_get()
        .await
        .map_err(|err| ConfigCommandError::Internal(err.to_string()))?;

    // 只有在跟随系统主题时才处理
    if config.appearance.theme_config.follow_system {
        let current_theme_name =
            theme_service.get_current_theme_name(&config.appearance.theme_config, Some(is_dark));

        // 通知前端主题已更改
        app_handle
            .emit("theme-changed", &current_theme_name)
            .map_err(|err| ConfigCommandError::Internal(err.to_string()))?;
    }

    Ok(())
}
