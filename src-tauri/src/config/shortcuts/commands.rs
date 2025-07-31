/*!
 * 快捷键系统 Tauri 命令接口
 *
 * 提供前端调用的快捷键管理命令，包括获取、更新、验证、重置快捷键配置等功能。
 */

use crate::config::commands::ConfigManagerState;
use crate::config::shortcuts::{
    ConflictDetectionResult, Platform, PlatformAdapter, ShortcutCategory, ShortcutConflictDetector,
    ShortcutManager, ShortcutStatistics, ShortcutValidationResult, ShortcutValidator,
};
use crate::config::types::{ShortcutBinding, ShortcutsConfig};
use crate::utils::error::AppResult;
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{debug, error, info};

/// 获取快捷键配置
///
/// # Returns
/// 返回当前的快捷键配置
#[tauri::command]
pub async fn get_shortcuts_config(
    state: State<'_, ConfigManagerState>,
) -> Result<ShortcutsConfig, String> {
    debug!("开始获取快捷键配置");

    let config = state.config.lock().await;
    let shortcuts_config = config.shortcuts.clone();

    info!("快捷键配置获取成功");
    Ok(shortcuts_config)
}

/// 更新快捷键配置
///
/// # Arguments
/// * `shortcuts_config` - 新的快捷键配置
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn update_shortcuts_config(
    shortcuts_config: ShortcutsConfig,
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("开始更新快捷键配置");

    // 验证快捷键配置
    let validator = ShortcutValidator::new();
    let validation_result = validator
        .validate_shortcuts_config(&shortcuts_config)
        .map_err(|e| {
            error!("快捷键配置验证失败: {}", e);
            e.to_string()
        })?;

    if !validation_result.is_valid {
        let error_msg = format!(
            "快捷键配置验证失败: {}",
            validation_result
                .errors
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        error!("{}", error_msg);
        return Err(error_msg);
    }

    // 更新配置
    {
        let mut config = state.config.lock().await;
        config.shortcuts = shortcuts_config;
    }

    // 保存配置到文件
    let config = state.config.lock().await;
    ConfigManagerState::save_config_to_file(&config, &state.config_path)
        .await
        .map_err(|e| {
            error!("保存快捷键配置失败: {}", e);
            e.to_string()
        })?;

    info!("快捷键配置更新成功");
    Ok(())
}

/// 验证快捷键配置
///
/// # Arguments
/// * `shortcuts_config` - 要验证的快捷键配置
///
/// # Returns
/// 返回验证结果
#[tauri::command]
pub async fn validate_shortcuts_config(
    shortcuts_config: ShortcutsConfig,
) -> Result<ShortcutValidationResult, String> {
    debug!("开始验证快捷键配置");

    let validator = ShortcutValidator::new();
    let result = validator
        .validate_shortcuts_config(&shortcuts_config)
        .map_err(|e| {
            error!("快捷键配置验证失败: {}", e);
            e.to_string()
        })?;

    info!("快捷键配置验证完成");
    Ok(result)
}

/// 验证单个快捷键绑定
///
/// # Arguments
/// * `shortcut_binding` - 要验证的快捷键绑定
///
/// # Returns
/// 返回验证结果
#[tauri::command]
pub async fn validate_shortcut_binding(
    shortcut_binding: ShortcutBinding,
) -> Result<ShortcutValidationResult, String> {
    debug!("开始验证快捷键绑定: {:?}", shortcut_binding);

    let validator = ShortcutValidator::new();
    let result = validator
        .validate_shortcut_binding(&shortcut_binding)
        .map_err(|e| {
            error!("快捷键绑定验证失败: {}", e);
            e.to_string()
        })?;

    info!("快捷键绑定验证完成");
    Ok(result)
}

/// 检测快捷键冲突
///
/// # Arguments
/// * `shortcuts_config` - 要检测的快捷键配置
///
/// # Returns
/// 返回冲突检测结果
#[tauri::command]
pub async fn detect_shortcut_conflicts(
    shortcuts_config: ShortcutsConfig,
) -> Result<ConflictDetectionResult, String> {
    debug!("开始检测快捷键冲突");

    let detector = ShortcutConflictDetector::new();
    let result = detector.detect_conflicts(&shortcuts_config).map_err(|e| {
        error!("快捷键冲突检测失败: {}", e);
        e.to_string()
    })?;

    info!("快捷键冲突检测完成，发现 {} 个冲突", result.conflicts.len());
    Ok(result)
}

/// 适配快捷键到指定平台
///
/// # Arguments
/// * `shortcuts_config` - 要适配的快捷键配置
/// * `target_platform` - 目标平台
///
/// # Returns
/// 返回适配后的快捷键配置
#[tauri::command]
pub async fn adapt_shortcuts_for_platform(
    shortcuts_config: ShortcutsConfig,
    target_platform: Platform,
) -> Result<ShortcutsConfig, String> {
    debug!("开始适配快捷键到平台: {:?}", target_platform);

    let adapter = PlatformAdapter::new();
    let result = adapter
        .adapt_shortcuts_for_platform(&shortcuts_config, target_platform)
        .map_err(|e| {
            error!("快捷键平台适配失败: {}", e);
            e.to_string()
        })?;

    info!("快捷键平台适配完成");
    Ok(result)
}

/// 获取当前平台信息
///
/// # Returns
/// 返回当前平台
#[tauri::command]
pub async fn get_current_platform() -> Result<Platform, String> {
    debug!("获取当前平台信息");

    let adapter = PlatformAdapter::new();
    let platform = adapter.current_platform();

    info!("当前平台: {:?}", platform);
    Ok(platform)
}

/// 重置快捷键配置到默认值
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn reset_shortcuts_to_defaults(
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("开始重置快捷键配置到默认值");

    // 创建默认快捷键配置
    let default_shortcuts = crate::config::defaults::create_default_shortcuts_config();

    // 更新配置
    {
        let mut config = state.config.lock().await;
        config.shortcuts = default_shortcuts;
    }

    // 保存配置到文件
    let config = state.config.lock().await;
    ConfigManagerState::save_config_to_file(&config, &state.config_path)
        .await
        .map_err(|e| {
            error!("保存默认快捷键配置失败: {}", e);
            e.to_string()
        })?;

    info!("快捷键配置已重置到默认值");
    Ok(())
}

/// 获取快捷键统计信息
///
/// # Returns
/// 返回快捷键统计信息
#[tauri::command]
pub async fn get_shortcuts_statistics(
    state: State<'_, ConfigManagerState>,
) -> Result<ShortcutStatistics, String> {
    debug!("开始获取快捷键统计信息");

    let config = state.config.lock().await;
    let manager = ShortcutManager::new(config.shortcuts.clone());
    let statistics = manager.get_statistics();

    info!("快捷键统计信息获取成功");
    Ok(statistics)
}

/// 添加快捷键
///
/// # Arguments
/// * `category` - 快捷键类别
/// * `shortcut` - 要添加的快捷键
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn add_shortcut(
    category: ShortcutCategory,
    shortcut: ShortcutBinding,
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("开始添加快捷键: {:?} 到类别 {:?}", shortcut, category);

    // 验证快捷键
    let validator = ShortcutValidator::new();
    let validation_result = validator
        .validate_shortcut_binding(&shortcut)
        .map_err(|e| {
            error!("快捷键验证失败: {}", e);
            e.to_string()
        })?;

    if !validation_result.is_valid {
        let error_msg = format!(
            "快捷键验证失败: {}",
            validation_result
                .errors
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        error!("{}", error_msg);
        return Err(error_msg);
    }

    // 添加快捷键
    {
        let mut config = state.config.lock().await;
        let mut manager = ShortcutManager::new(config.shortcuts.clone());
        manager.add_shortcut(category, shortcut).map_err(|e| {
            error!("添加快捷键失败: {}", e);
            e.to_string()
        })?;
        config.shortcuts = manager.get_config().clone();
    }

    // 保存配置到文件
    let config = state.config.lock().await;
    ConfigManagerState::save_config_to_file(&config, &state.config_path)
        .await
        .map_err(|e| {
            error!("保存快捷键配置失败: {}", e);
            e.to_string()
        })?;

    info!("快捷键添加成功");
    Ok(())
}

/// 删除快捷键
///
/// # Arguments
/// * `category` - 快捷键类别
/// * `index` - 要删除的快捷键索引
///
/// # Returns
/// 返回被删除的快捷键
#[tauri::command]
pub async fn remove_shortcut(
    category: ShortcutCategory,
    index: usize,
    state: State<'_, ConfigManagerState>,
) -> Result<ShortcutBinding, String> {
    debug!("开始删除快捷键: 类别 {:?}, 索引 {}", category, index);

    let removed_shortcut = {
        let mut config = state.config.lock().await;
        let mut manager = ShortcutManager::new(config.shortcuts.clone());
        let removed = manager.remove_shortcut(category, index).map_err(|e| {
            error!("删除快捷键失败: {}", e);
            e.to_string()
        })?;
        config.shortcuts = manager.get_config().clone();
        removed
    };

    // 保存配置到文件
    let config = state.config.lock().await;
    ConfigManagerState::save_config_to_file(&config, &state.config_path)
        .await
        .map_err(|e| {
            error!("保存快捷键配置失败: {}", e);
            e.to_string()
        })?;

    info!("快捷键删除成功");
    Ok(removed_shortcut)
}

/// 更新快捷键
///
/// # Arguments
/// * `category` - 快捷键类别
/// * `index` - 要更新的快捷键索引
/// * `shortcut` - 新的快捷键
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn update_shortcut(
    category: ShortcutCategory,
    index: usize,
    shortcut: ShortcutBinding,
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!(
        "开始更新快捷键: 类别 {:?}, 索引 {}, 新快捷键 {:?}",
        category, index, shortcut
    );

    // 验证快捷键
    let validator = ShortcutValidator::new();
    let validation_result = validator
        .validate_shortcut_binding(&shortcut)
        .map_err(|e| {
            error!("快捷键验证失败: {}", e);
            e.to_string()
        })?;

    if !validation_result.is_valid {
        let error_msg = format!(
            "快捷键验证失败: {}",
            validation_result
                .errors
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        error!("{}", error_msg);
        return Err(error_msg);
    }

    // 更新快捷键
    {
        let mut config = state.config.lock().await;
        let mut manager = ShortcutManager::new(config.shortcuts.clone());
        manager
            .update_shortcut(category, index, shortcut)
            .map_err(|e| {
                error!("更新快捷键失败: {}", e);
                e.to_string()
            })?;
        config.shortcuts = manager.get_config().clone();
    }

    // 保存配置到文件
    let config = state.config.lock().await;
    ConfigManagerState::save_config_to_file(&config, &state.config_path)
        .await
        .map_err(|e| {
            error!("保存快捷键配置失败: {}", e);
            e.to_string()
        })?;

    info!("快捷键更新成功");
    Ok(())
}
