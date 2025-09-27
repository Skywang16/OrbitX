/*!
 * 快捷键系统 Tauri 命令接口
 *
 * 提供前端调用的快捷键管理API
 */

use super::core::ShortcutManager;
use super::types::*;
use crate::config::commands::ConfigManagerState;
use crate::config::types::{ShortcutBinding, ShortcutsConfig};
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};

use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use tracing::{debug, info};

pub struct ShortcutManagerState {
    pub manager: Arc<Mutex<ShortcutManager>>,
}

impl ShortcutManagerState {
    pub async fn new(config_state: &ConfigManagerState) -> crate::utils::error::AppResult<Self> {
        let manager = ShortcutManager::new(Arc::clone(&config_state.toml_manager)).await?;
        Ok(Self {
            manager: Arc::new(Mutex::new(manager)),
        })
    }
}

// Tauri 命令
#[tauri::command]
pub async fn shortcuts_get_config(
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<ShortcutsConfig> {
    debug!("获取快捷键配置");

    let manager = state.manager.lock().await;
    match manager.config_get().await {
        Ok(config) => {
            info!("获取快捷键配置成功");
            Ok(api_success!(config))
        }
        Err(_) => Ok(api_error!("shortcuts.get_failed")),
    }
}

#[tauri::command]
pub async fn shortcuts_update_config(
    config: ShortcutsConfig,
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<EmptyData> {
    debug!("更新快捷键配置");

    let manager = state.manager.lock().await;
    match manager.config_update(config).await {
        Ok(_) => {
            info!("快捷键配置更新成功");
            Ok(api_success!())
        }
        Err(_) => Ok(api_error!("shortcuts.update_failed")),
    }
}

#[tauri::command]
pub async fn shortcuts_validate_config(
    config: ShortcutsConfig,
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<ValidationResult> {
    debug!("验证快捷键配置");

    let manager = state.manager.lock().await;
    match manager.config_validate(&config).await {
        Ok(result) => {
            debug!("快捷键配置验证完成");
            Ok(api_success!(result))
        }
        Err(_) => Ok(api_error!("shortcuts.validate_failed")),
    }
}

#[tauri::command]
pub async fn shortcuts_detect_conflicts(
    config: ShortcutsConfig,
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<ConflictResult> {
    debug!("检测快捷键冲突");

    let manager = state.manager.lock().await;
    match manager.detect_conflicts(&config).await {
        Ok(result) => {
            debug!("快捷键冲突检测完成，发现 {} 个冲突", result.conflicts.len());
            Ok(api_success!(result))
        }
        Err(_) => Ok(api_error!("shortcuts.detect_conflicts_failed")),
    }
}

#[tauri::command]
pub async fn shortcuts_add(
    binding: ShortcutBinding,
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<EmptyData> {
    debug!("添加快捷键: {:?}", binding);

    let manager = state.manager.lock().await;
    match manager.shortcuts_add(binding).await {
        Ok(_) => {
            info!("添加快捷键成功");
            Ok(api_success!())
        }
        Err(_) => Ok(api_error!("shortcuts.add_failed")),
    }
}

#[tauri::command]
pub async fn shortcuts_remove(
    index: usize,
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<ShortcutBinding> {
    debug!("删除快捷键: 索引 {}", index);

    let manager = state.manager.lock().await;
    match manager.shortcuts_remove(index).await {
        Ok(removed) => {
            info!("删除快捷键成功");
            Ok(api_success!(removed))
        }
        Err(_) => Ok(api_error!("shortcuts.remove_failed")),
    }
}

#[tauri::command]
pub async fn shortcuts_update(
    index: usize,
    binding: ShortcutBinding,
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<EmptyData> {
    debug!("更新快捷键: 索引 {}, 新绑定 {:?}", index, binding);

    let manager = state.manager.lock().await;
    match manager.shortcuts_update(index, binding).await {
        Ok(_) => {
            info!("更新快捷键成功");
            Ok(api_success!())
        }
        Err(_) => Ok(api_error!("shortcuts.update_failed")),
    }
}

#[tauri::command]
pub async fn shortcuts_reset_to_defaults(
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<EmptyData> {
    debug!("重置快捷键到默认配置");

    let manager = state.manager.lock().await;
    match manager.reset_to_defaults().await {
        Ok(_) => {
            info!("快捷键重置成功");
            Ok(api_success!())
        }
        Err(_) => Ok(api_error!("shortcuts.reset_failed")),
    }
}

#[tauri::command]
pub async fn shortcuts_get_statistics(
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<ShortcutStatistics> {
    debug!("获取快捷键统计信息");

    let manager = state.manager.lock().await;
    match manager.get_statistics().await {
        Ok(stats) => {
            debug!("获取快捷键统计信息成功");
            Ok(api_success!(stats))
        }
        Err(_) => Ok(api_error!("shortcuts.get_stats_failed")),
    }
}

#[tauri::command]
pub async fn shortcuts_search(
    options: SearchOptions,
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<SearchResult> {
    debug!("搜索快捷键: {:?}", options);

    let manager = state.manager.lock().await;
    match manager.shortcuts_search(options).await {
        Ok(result) => {
            debug!("快捷键搜索完成，找到 {} 个匹配项", result.total);
            Ok(api_success!(result))
        }
        Err(_) => Ok(api_error!("shortcuts.search_failed")),
    }
}

#[tauri::command]
pub async fn shortcuts_execute_action(
    action: crate::config::types::ShortcutAction,
    key_combination: String,
    active_terminal_id: Option<String>,
    metadata: Option<HashMap<String, serde_json::Value>>,
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<OperationResult<serde_json::Value>> {
    debug!("执行快捷键动作: {:?}", action);
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
    Ok(api_success!(result))
}

#[tauri::command]
pub async fn shortcuts_get_current_platform() -> TauriApiResult<Platform> {
    debug!("获取当前平台信息");

    let platform = if cfg!(target_os = "macos") {
        Platform::MacOS
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else {
        Platform::Linux
    };

    Ok(api_success!(platform))
}

#[tauri::command]
pub async fn shortcuts_export_config(
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<String> {
    debug!("导出快捷键配置");

    let manager = state.manager.lock().await;
    let config = match manager.config_get().await {
        Ok(c) => c,
        Err(_) => return Ok(api_error!("shortcuts.get_failed")),
    };

    let json_config = match serde_json::to_string_pretty(&config) {
        Ok(s) => s,
        Err(_) => return Ok(api_error!("shortcuts.export_failed")),
    };

    info!("快捷键配置导出成功");
    Ok(api_success!(json_config))
}

#[tauri::command]
pub async fn shortcuts_import_config(
    config_json: String,
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<EmptyData> {
    debug!("导入快捷键配置");

    let config: ShortcutsConfig = match serde_json::from_str(&config_json) {
        Ok(c) => c,
        Err(_) => return Ok(api_error!("shortcuts.import_parse_failed")),
    };

    let manager = state.manager.lock().await;
    match manager.config_update(config).await {
        Ok(_) => {
            info!("快捷键配置导入成功");
            Ok(api_success!())
        }
        Err(_) => Ok(api_error!("shortcuts.import_failed")),
    }
}

#[tauri::command]
pub async fn shortcuts_get_registered_actions(
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<Vec<String>> {
    debug!("获取已注册的动作列表");

    let manager = state.manager.lock().await;
    let registry = manager.get_action_registry().await;
    let registry_guard = registry.read().await;
    let actions = registry_guard.shortcuts_get_registered_actions().await;

    debug!("获取已注册动作列表成功，共 {} 个动作", actions.len());
    Ok(api_success!(actions))
}

#[tauri::command]
pub async fn shortcuts_get_action_metadata(
    action_name: String,
    state: State<'_, ShortcutManagerState>,
) -> TauriApiResult<Option<super::actions::ActionMetadata>> {
    debug!("获取动作元数据: {}", action_name);

    let manager = state.manager.lock().await;
    let registry = manager.get_action_registry().await;
    let registry_guard = registry.read().await;
    let metadata = registry_guard
        .shortcuts_get_action_metadata(&action_name)
        .await;

    Ok(api_success!(metadata))
}

#[tauri::command]
pub async fn shortcuts_validate_key_combination(
    key: String,
    modifiers: Vec<String>,
) -> TauriApiResult<ValidationResult> {
    debug!("验证快捷键组合: {} + {:?}", key, modifiers);

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    if key.is_empty() {
        errors.push(ValidationError {
            error_type: ValidationErrorType::EmptyKey,
            message: "按键不能为空".to_string(),
            key_combination: None,
        });
    }

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

    let system_reserved = [("alt", "f4"), ("cmd", "q"), ("cmd", "tab"), ("alt", "tab")];

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

    Ok(api_success!(result))
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

        match platform {
            Platform::MacOS => assert!(cfg!(target_os = "macos")),
            Platform::Windows => assert!(cfg!(target_os = "windows")),
            Platform::Linux => assert!(cfg!(target_os = "linux")),
        }
    }

    #[tokio::test]
    async fn test_key_combination_validation() {
        let resp = shortcuts_validate_key_combination("c".to_string(), vec!["cmd".to_string()])
            .await
            .unwrap();
        let result = resp.data.expect("data should be present");

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_invalid_key_validation() {
        let resp = shortcuts_validate_key_combination("".to_string(), vec!["invalid".to_string()])
            .await
            .unwrap();
        let result = resp.data.expect("data should be present");

        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }
}
