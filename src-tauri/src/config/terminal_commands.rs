/*!
 * 终端配置相关的 Tauri 命令
 *
 * 提供终端配置的获取、更新、验证等功能。
 * 使用新的TomlConfigManager作为底层实现。
 */

use crate::config::{
    commands::ConfigManagerState,
    defaults::create_default_terminal_config,
    types::TerminalConfig,
};
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};

use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::warn;

/// 终端配置验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalConfigValidationResult {
    /// 是否有效
    pub is_valid: bool,
    /// 错误信息列表
    pub errors: Vec<String>,
    /// 警告信息列表
    pub warnings: Vec<String>,
}

// Tauri 命令接口

/// 获取终端配置
#[tauri::command]
pub async fn terminal_config_get(state: State<'_, ConfigManagerState>) -> TauriApiResult<TerminalConfig> {
    match state.toml_manager.config_get().await {
        Ok(config) => {
            let terminal_config = config.terminal.clone();
            Ok(api_success!(terminal_config))
        }
        Err(_) => Ok(api_error!("config.get_failed")),
    }
}

/// 设置终端配置（全量覆盖）
#[tauri::command]
pub async fn terminal_config_set(
    terminal_config: TerminalConfig,
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    let result = state
        .toml_manager
        .config_update(|config| {
            config.terminal = terminal_config.clone();
            Ok(())
        })
        .await;

    match result {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.update_failed")),
    }
}

/// 验证终端配置
#[tauri::command]
pub async fn terminal_config_validate(
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<TerminalConfigValidationResult> {
    let config = match state.toml_manager.config_get().await {
        Ok(c) => c,
        Err(_) => return Ok(api_error!("config.get_failed")),
    };
    let terminal_config = &config.terminal;

    let mut errors = Vec::new();
    let warnings = Vec::new();

    // 验证滚动缓冲区
    if !(100..=100000).contains(&terminal_config.scrollback) {
        errors.push(format!(
            "滚动缓冲区行数必须在100-100000之间，当前值: {}",
            terminal_config.scrollback
        ));
    }

    // 验证Shell配置
    if terminal_config.shell.default_shell.is_empty() {
        errors.push("默认Shell不能为空".to_string());
    }

    // 验证光标配置
    if !(0.1..=5.0).contains(&terminal_config.cursor.thickness) {
        errors.push(format!(
            "光标粗细必须在0.1-5.0之间，当前值: {}",
            terminal_config.cursor.thickness
        ));
    }

    // 验证颜色格式
    if !terminal_config.cursor.color.starts_with('#') || terminal_config.cursor.color.len() != 7 {
        errors.push(format!(
            "光标颜色格式无效: {}",
            terminal_config.cursor.color
        ));
    }

    let is_valid = errors.is_empty();

    if !is_valid {
        warn!("终端配置验证失败: {:?}", errors);
    }

    Ok(api_success!(TerminalConfigValidationResult {
        is_valid,
        errors,
        warnings,
    }))
}

/// 重置终端配置为默认值
#[tauri::command]
pub async fn terminal_config_reset_to_defaults(
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    let default_terminal_config = create_default_terminal_config();

    // 更新配置
    let result = state
        .toml_manager
        .config_update(|config| {
            config.terminal = default_terminal_config.clone();
            Ok(())
        })
        .await;

    match result {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.reset_failed")),
    }
}
