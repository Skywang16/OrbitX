/*!
 * 快捷键系统 Tauri 命令接口
 *
 * 提供前端调用的快捷键管理API
 */

use super::core::ShortcutManager;
use super::types::*;
use crate::config::commands::ConfigManagerState;
use crate::config::types::{ShortcutBinding, ShortcutsConfig};
use crate::utils::error::ToTauriResult;

use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use tracing::{debug, info};

/// 快捷键管理器状态
pub struct ShortcutManagerState {
    /// 快捷键管理器实例
    pub manager: Arc<Mutex<ShortcutManager>>,
}

impl ShortcutManagerState {
    /// 创建新的快捷键管理器状态
    pub async fn new(config_state: &ConfigManagerState) -> crate::utils::error::AppResult<Self> {
        let manager = ShortcutManager::new(Arc::clone(&config_state.toml_manager)).await?;
        Ok(Self {
            manager: Arc::new(Mutex::new(manager)),
        })
    }
}

// ============================================================================
// Tauri 命令
// ============================================================================

/// 获取快捷键配置
#[tauri::command]
pub async fn get_shortcuts_config(
    state: State<'_, ShortcutManagerState>,
) -> Result<ShortcutsConfig, String> {
    debug!("获取快捷键配置");

    let manager = state.manager.lock().await;
    let config = manager.get_config().await.to_tauri()?;

    info!("获取快捷键配置成功");
    Ok(config)
}

/// 更新快捷键配置
#[tauri::command]
pub async fn update_shortcuts_config(
    config: ShortcutsConfig,
    state: State<'_, ShortcutManagerState>,
) -> Result<(), String> {
    debug!("更新快捷键配置");

    let manager = state.manager.lock().await;
    manager.update_config(config).await.to_tauri()?;

    info!("快捷键配置更新成功");
    Ok(())
}

/// 验证快捷键配置
#[tauri::command]
pub async fn validate_shortcuts_config(
    config: ShortcutsConfig,
    state: State<'_, ShortcutManagerState>,
) -> Result<ValidationResult, String> {
    debug!("验证快捷键配置");

    let manager = state.manager.lock().await;
    let result = manager.validate_config(&config).await.to_tauri()?;

    debug!("快捷键配置验证完成");
    Ok(result)
}

/// 检测快捷键冲突
#[tauri::command]
pub async fn detect_shortcuts_conflicts(
    config: ShortcutsConfig,
    state: State<'_, ShortcutManagerState>,
) -> Result<ConflictResult, String> {
    debug!("检测快捷键冲突");

    let manager = state.manager.lock().await;
    let result = manager.detect_conflicts(&config).await.to_tauri()?;

    debug!("快捷键冲突检测完成，发现 {} 个冲突", result.conflicts.len());
    Ok(result)
}

/// 添加快捷键
#[tauri::command]
pub async fn add_shortcut(
    category: String,
    binding: ShortcutBinding,
    state: State<'_, ShortcutManagerState>,
) -> Result<(), String> {
    debug!("添加快捷键: {:?} 到类别 {}", binding, category);

    let category = category
        .parse::<ShortcutCategory>()
        .map_err(|e| e.to_string())?;
    let manager = state.manager.lock().await;
    manager.add_shortcut(category, binding).await.to_tauri()?;

    info!("添加快捷键成功");
    Ok(())
}

/// 删除快捷键
#[tauri::command]
pub async fn remove_shortcut(
    category: String,
    index: usize,
    state: State<'_, ShortcutManagerState>,
) -> Result<ShortcutBinding, String> {
    debug!("删除快捷键: 类别 {}, 索引 {}", category, index);

    let category = category
        .parse::<ShortcutCategory>()
        .map_err(|e| e.to_string())?;
    let manager = state.manager.lock().await;
    let removed = manager.remove_shortcut(category, index).await.to_tauri()?;

    info!("删除快捷键成功");
    Ok(removed)
}

/// 更新快捷键
#[tauri::command]
pub async fn update_shortcut(
    category: String,
    index: usize,
    binding: ShortcutBinding,
    state: State<'_, ShortcutManagerState>,
) -> Result<(), String> {
    debug!(
        "更新快捷键: 类别 {}, 索引 {}, 新绑定 {:?}",
        category, index, binding
    );

    let category = category
        .parse::<ShortcutCategory>()
        .map_err(|e| e.to_string())?;
    let manager = state.manager.lock().await;
    manager
        .update_shortcut(category, index, binding)
        .await
        .to_tauri()?;

    info!("更新快捷键成功");
    Ok(())
}

/// 重置快捷键到默认配置
#[tauri::command]
pub async fn reset_shortcuts_to_defaults(
    state: State<'_, ShortcutManagerState>,
) -> Result<(), String> {
    debug!("重置快捷键到默认配置");

    let manager = state.manager.lock().await;
    manager.reset_to_defaults().await.to_tauri()?;

    info!("快捷键重置成功");
    Ok(())
}

/// 获取快捷键统计信息
#[tauri::command]
pub async fn get_shortcuts_statistics(
    state: State<'_, ShortcutManagerState>,
) -> Result<ShortcutStatistics, String> {
    debug!("获取快捷键统计信息");

    let manager = state.manager.lock().await;
    let stats = manager.get_statistics().await.to_tauri()?;

    debug!("获取快捷键统计信息成功");
    Ok(stats)
}

/// 搜索快捷键
#[tauri::command]
pub async fn search_shortcuts(
    options: SearchOptions,
    state: State<'_, ShortcutManagerState>,
) -> Result<SearchResult, String> {
    debug!("搜索快捷键: {:?}", options);

    let manager = state.manager.lock().await;
    let result = manager.search_shortcuts(options).await.to_tauri()?;

    debug!("快捷键搜索完成，找到 {} 个匹配项", result.total);
    Ok(result)
}

/// 执行快捷键动作
#[tauri::command]
pub async fn execute_shortcut_action(
    action: crate::config::types::ShortcutAction,
    key_combination: String,
    active_terminal_id: Option<String>,
    metadata: Option<HashMap<String, serde_json::Value>>,
    state: State<'_, ShortcutManagerState>,
) -> Result<OperationResult<serde_json::Value>, String> {
    debug!("执行快捷键动作: {:?}", action);

    // 解析按键组合
    let parts: Vec<&str> = key_combination.split('+').collect();
    let key = parts.last().map(|s| s.to_string()).unwrap_or_default();
    let modifiers: Vec<String> = parts
        .iter()
        .take(parts.len().saturating_sub(1))
        .map(|s| s.to_string())
        .collect();

    let context = ActionContext {
        key_combination: KeyCombination::new(key, modifiers),
        active_terminal_id,
        metadata: metadata.unwrap_or_default(),
    };

    let manager = state.manager.lock().await;
    let result = manager.execute_action(&action, &context).await;

    debug!("快捷键动作执行完成");
    Ok(result)
}

/// 获取当前平台信息
#[tauri::command]
pub async fn get_current_platform() -> Result<Platform, String> {
    debug!("获取当前平台信息");

    let platform = if cfg!(target_os = "macos") {
        Platform::MacOS
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else {
        Platform::Linux
    };

    Ok(platform)
}

/// 导出快捷键配置
#[tauri::command]
pub async fn export_shortcuts_config(
    state: State<'_, ShortcutManagerState>,
) -> Result<String, String> {
    debug!("导出快捷键配置");

    let manager = state.manager.lock().await;
    let config = manager.get_config().await.to_tauri()?;

    let json_config =
        serde_json::to_string_pretty(&config).map_err(|e| format!("序列化配置失败: {}", e))?;

    info!("快捷键配置导出成功");
    Ok(json_config)
}

/// 导入快捷键配置
#[tauri::command]
pub async fn import_shortcuts_config(
    config_json: String,
    state: State<'_, ShortcutManagerState>,
) -> Result<(), String> {
    debug!("导入快捷键配置");

    let config: ShortcutsConfig =
        serde_json::from_str(&config_json).map_err(|e| format!("解析配置失败: {}", e))?;

    let manager = state.manager.lock().await;
    manager.update_config(config).await.to_tauri()?;

    info!("快捷键配置导入成功");
    Ok(())
}

/// 获取已注册的动作列表
#[tauri::command]
pub async fn get_registered_actions(
    state: State<'_, ShortcutManagerState>,
) -> Result<Vec<String>, String> {
    debug!("获取已注册的动作列表");

    let manager = state.manager.lock().await;
    let registry = manager.get_action_registry().await;
    let registry_guard = registry.read().await;
    let actions = registry_guard.get_registered_actions().await;

    debug!("获取已注册动作列表成功，共 {} 个动作", actions.len());
    Ok(actions)
}

/// 获取动作元数据
#[tauri::command]
pub async fn get_action_metadata(
    action_name: String,
    state: State<'_, ShortcutManagerState>,
) -> Result<Option<super::actions::ActionMetadata>, String> {
    debug!("获取动作元数据: {}", action_name);

    let manager = state.manager.lock().await;
    let registry = manager.get_action_registry().await;
    let registry_guard = registry.read().await;
    let metadata = registry_guard.get_action_metadata(&action_name).await;

    Ok(metadata)
}

/// 检查快捷键组合是否有效
#[tauri::command]
pub async fn validate_key_combination(
    key: String,
    modifiers: Vec<String>,
) -> Result<ValidationResult, String> {
    debug!("验证快捷键组合: {} + {:?}", key, modifiers);

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // 验证按键
    if key.is_empty() {
        errors.push(ValidationError {
            error_type: ValidationErrorType::EmptyKey,
            message: "按键不能为空".to_string(),
            key_combination: None,
        });
    }

    // 验证修饰键
    let valid_modifiers = ["ctrl", "alt", "shift", "cmd", "meta", "super"];
    for modifier in &modifiers {
        if !valid_modifiers.contains(&modifier.to_lowercase().as_str()) {
            errors.push(ValidationError {
                error_type: ValidationErrorType::InvalidModifier,
                message: format!("无效的修饰键: {}", modifier),
                key_combination: Some(KeyCombination::new(key.clone(), modifiers.clone())),
            });
        }
    }

    // 检查是否为系统保留快捷键
    let system_reserved = [
        ("alt", "f4"),  // Windows关闭窗口
        ("cmd", "q"),   // macOS退出应用
        ("cmd", "tab"), // macOS切换应用
        ("alt", "tab"), // Windows/Linux切换窗口
    ];

    for (mod_key, reserved_key) in system_reserved {
        if modifiers.contains(&mod_key.to_string()) && key.to_lowercase() == reserved_key {
            warnings.push(ValidationWarning {
                warning_type: ValidationWarningType::PlatformSpecific,
                message: format!("{}+{} 是系统保留快捷键", mod_key, reserved_key),
                key_combination: Some(KeyCombination::new(key.clone(), modifiers.clone())),
            });
        }
    }

    let result = ValidationResult {
        is_valid: errors.is_empty(),
        errors,
        warnings,
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = if cfg!(target_os = "macos") {
            Platform::MacOS
        } else if cfg!(target_os = "windows") {
            Platform::Windows
        } else {
            Platform::Linux
        };

        // 测试平台检测逻辑
        match platform {
            Platform::MacOS => assert!(cfg!(target_os = "macos")),
            Platform::Windows => assert!(cfg!(target_os = "windows")),
            Platform::Linux => assert!(cfg!(target_os = "linux")),
        }
    }

    #[tokio::test]
    async fn test_key_combination_validation() {
        let result = validate_key_combination("c".to_string(), vec!["cmd".to_string()])
            .await
            .unwrap();

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_invalid_key_validation() {
        let result = validate_key_combination("".to_string(), vec!["invalid".to_string()])
            .await
            .unwrap();

        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }
}
