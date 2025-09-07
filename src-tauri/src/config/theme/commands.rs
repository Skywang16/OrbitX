/*!
 * 主题相关的 Tauri 命令
 *
 * 提供给前端调用的主题管理接口，包括获取当前主题、
 * 切换主题、获取主题列表等功能。
 */

use super::service::{SystemThemeDetector, ThemeService};
use super::types::{Theme, ThemeConfig};
use crate::config::TomlConfigManager;
use crate::utils::error::{AppResult, TauriResult, ToTauriResult};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tracing::{debug, info};

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
pub async fn get_theme_config_status(
    config_manager: State<'_, Arc<TomlConfigManager>>,
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriResult<ThemeConfigStatus> {
    let config = config_manager.get_config().await.to_tauri()?;

    let theme_config = &config.appearance.theme_config;
    let is_system_dark = SystemThemeDetector::is_dark_mode();

    // 获取当前主题名称
    let current_theme_name = theme_service.get_current_theme_name(theme_config, is_system_dark);

    // 获取所有可用主题
    let theme_list = theme_service
        .theme_manager()
        .list_themes()
        .await
        .to_tauri()?;

    let available_themes = theme_list
        .into_iter()
        .map(|theme_entry| ThemeInfo {
            name: theme_entry.name.clone(),
            theme_type: theme_entry.theme_type,
            is_current: theme_entry.name == current_theme_name,
        })
        .collect();

    Ok(ThemeConfigStatus {
        current_theme_name,
        theme_config: theme_config.clone(),
        is_system_dark,
        available_themes,
    })
}

/// 获取当前主题数据
#[tauri::command]
pub async fn get_current_theme(
    config_manager: State<'_, Arc<TomlConfigManager>>,
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriResult<Theme> {
    let config = config_manager.get_config().await.to_tauri()?;

    let theme_config = &config.appearance.theme_config;
    let is_system_dark = SystemThemeDetector::is_dark_mode();

    theme_service
        .load_current_theme(theme_config, is_system_dark)
        .await
        .to_tauri()
}

/// 设置终端主题（手动模式）
#[tauri::command]
pub async fn set_terminal_theme(
    theme_name: String,
    app_handle: AppHandle,
    config_manager: State<'_, Arc<TomlConfigManager>>,
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriResult<()> {
    // 验证主题是否存在
    if !theme_service.theme_exists(&theme_name).await {
        return Err(format!("主题不存在: {}", theme_name));
    }

    // 更新配置
    config_manager
        .update_config(|config| {
            config.appearance.theme_config.terminal_theme = theme_name.clone();
            config.appearance.theme_config.follow_system = false; // 切换到手动模式
            Ok(())
        })
        .await
        .to_tauri()?;

    // 发送主题变化事件，确保前端能立即响应
    app_handle
        .emit("theme-changed", &theme_name)
        .context("发送事件失败")
        .to_tauri()?;

    Ok(())
}

/// 设置跟随系统主题
#[tauri::command]
pub async fn set_follow_system_theme(
    follow_system: bool,
    light_theme: Option<String>,
    dark_theme: Option<String>,
    app_handle: AppHandle,
    config_manager: State<'_, Arc<TomlConfigManager>>,
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriResult<()> {
    // 验证主题是否存在
    if let Some(ref light) = light_theme {
        if !theme_service.theme_exists(light).await {
            return Err(format!("浅色主题不存在: {}", light));
        }
    }

    if let Some(ref dark) = dark_theme {
        if !theme_service.theme_exists(dark).await {
            return Err(format!("深色主题不存在: {}", dark));
        }
    }

    // 更新配置
    config_manager
        .update_config(|config| {
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
        .to_tauri()?;

    // 如果启用跟随系统主题，需要获取当前应该使用的主题并发送事件
    if follow_system {
        // 获取当前系统主题状态
        let config = config_manager.get_config().await.to_tauri()?;
        let is_system_dark = SystemThemeDetector::is_dark_mode();
        let current_theme_name =
            theme_service.get_current_theme_name(&config.appearance.theme_config, is_system_dark);

        // 发送主题变化事件
        app_handle
            .emit("theme-changed", &current_theme_name)
            .context("发送事件失败")
        .to_tauri()?;
    }

    Ok(())
}

/// 获取所有可用主题列表
#[tauri::command]
pub async fn get_available_themes(
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriResult<Vec<ThemeInfo>> {
    let theme_list = theme_service
        .theme_manager()
        .list_themes()
        .await
        .to_tauri()?;

    let themes = theme_list
        .into_iter()
        .map(|theme_entry| ThemeInfo {
            name: theme_entry.name,
            theme_type: theme_entry.theme_type,
            is_current: false, // 这里不设置当前状态，由前端决定
        })
        .collect();

    Ok(themes)
}

/// 系统主题变化处理
pub async fn handle_system_theme_change<R: tauri::Runtime>(
    app_handle: &AppHandle<R>,
    is_dark: bool,
) -> AppResult<()> {
    debug!("系统主题变化: {}", if is_dark { "深色" } else { "浅色" });

    let config_manager = app_handle.state::<Arc<TomlConfigManager>>();
    let theme_service = app_handle.state::<Arc<ThemeService>>();

    let config = config_manager.get_config().await?;

    // 只有在跟随系统主题时才处理
    if config.appearance.theme_config.follow_system {
        let current_theme_name =
            theme_service.get_current_theme_name(&config.appearance.theme_config, Some(is_dark));

        info!("系统主题变化，切换到主题: {}", current_theme_name);

        // 通知前端主题已更改
        app_handle
            .emit("theme-changed", &current_theme_name)
            .context("发送主题变化事件失败")?;
    }

    Ok(())
}
