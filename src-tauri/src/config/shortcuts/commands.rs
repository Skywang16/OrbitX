/*!
 * 快捷键配置相关的 Tauri 命令
 *
 * 提供快捷键配置的获取、更新等基本功能。
 */

use crate::config::commands::ConfigManagerState;
use crate::config::types::{ShortcutBinding, ShortcutsConfig};
use tauri::State;
use tracing::{debug, info};

/// 获取快捷键配置
#[tauri::command]
pub async fn get_shortcuts_config(
    state: State<'_, ConfigManagerState>,
) -> Result<ShortcutsConfig, String> {
    debug!("开始获取快捷键配置");

    let config = state
        .toml_manager
        .get_config()
        .await
        .map_err(|e| e.to_string())?;
    let shortcuts_config = config.shortcuts.clone();

    info!("快捷键配置获取成功");
    Ok(shortcuts_config)
}

/// 更新快捷键配置
#[tauri::command]
pub async fn update_shortcuts_config(
    shortcuts_config: ShortcutsConfig,
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("开始更新快捷键配置");

    // 更新配置
    state
        .toml_manager
        .update_config(|config| {
            config.shortcuts = shortcuts_config.clone();
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?;

    info!("快捷键配置更新成功");
    Ok(())
}

/// 重置快捷键配置到默认值
#[tauri::command]
pub async fn reset_shortcuts_to_defaults(
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("开始重置快捷键配置到默认值");

    let default_shortcuts = crate::config::defaults::create_default_shortcuts_config();

    // 更新配置
    state
        .toml_manager
        .update_config(|config| {
            config.shortcuts = default_shortcuts.clone();
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?;

    info!("快捷键配置已重置为默认值");
    Ok(())
}

/// 添加快捷键
#[tauri::command]
pub async fn add_shortcut(
    _category: String, // 简化参数类型
    shortcut: ShortcutBinding,
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("开始添加快捷键: {:?}", shortcut);

    // 简化实现：直接添加到custom类别
    state
        .toml_manager
        .update_config(|config| {
            config.shortcuts.custom.push(shortcut.clone());
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?;

    info!("快捷键添加成功");
    Ok(())
}

/// 删除快捷键
#[tauri::command]
pub async fn remove_shortcut(
    _category: String,
    index: usize,
    state: State<'_, ConfigManagerState>,
) -> Result<ShortcutBinding, String> {
    debug!("开始删除快捷键: 索引 {}", index);

    let mut removed_shortcut = None;

    // 从custom类别删除
    state
        .toml_manager
        .update_config(|config| {
            if index < config.shortcuts.custom.len() {
                removed_shortcut = Some(config.shortcuts.custom.remove(index));
            }
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?;

    match removed_shortcut {
        Some(shortcut) => {
            info!("快捷键删除成功");
            Ok(shortcut)
        }
        None => Err("快捷键索引无效".to_string()),
    }
}

/// 更新快捷键
#[tauri::command]
pub async fn update_shortcut(
    _category: String,
    index: usize,
    shortcut: ShortcutBinding,
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("开始更新快捷键: 索引 {}, 新快捷键 {:?}", index, shortcut);

    // 更新custom类别中的快捷键
    state
        .toml_manager
        .update_config(|config| {
            if index < config.shortcuts.custom.len() {
                config.shortcuts.custom[index] = shortcut.clone();
            } else {
                return Err(anyhow::anyhow!("快捷键索引无效"));
            }
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?;

    info!("快捷键更新成功");
    Ok(())
}

// ============================================================================
// 兼容性存根函数（为了保持向后兼容）
// ============================================================================

/// 验证快捷键配置（存根实现）
#[tauri::command]
pub async fn validate_shortcuts_config(
    _state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("验证快捷键配置");
    Ok(())
}

/// 验证快捷键绑定（存根实现）
#[tauri::command]
pub async fn validate_shortcut_binding(
    _binding: ShortcutBinding,
    _state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("验证快捷键绑定");
    Ok(())
}

/// 检测快捷键冲突（存根实现）
#[tauri::command]
pub async fn detect_shortcut_conflicts(
    _state: State<'_, ConfigManagerState>,
) -> Result<Vec<String>, String> {
    debug!("检测快捷键冲突");
    Ok(Vec::new())
}

/// 为平台适配快捷键（存根实现）
#[tauri::command]
pub async fn adapt_shortcuts_for_platform(
    _state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("为平台适配快捷键");
    Ok(())
}

/// 获取当前平台（存根实现）
#[tauri::command]
pub async fn get_current_platform() -> Result<String, String> {
    debug!("获取当前平台");
    Ok(if cfg!(target_os = "macos") {
        "macos".to_string()
    } else if cfg!(target_os = "windows") {
        "windows".to_string()
    } else {
        "linux".to_string()
    })
}
/// 获取快捷键统计信息（存根实现）
#[tauri::command]
pub async fn get_shortcuts_statistics(
    _state: State<'_, ConfigManagerState>,
) -> Result<String, String> {
    debug!("获取快捷键统计信息");
    Ok("统计信息".to_string())
}
