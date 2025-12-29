/*!
 * 终端配置相关的 Tauri 命令
 *
 * 提供终端配置的获取、更新、验证等功能。
 * 使用新的TomlConfigManager作为底层实现。
 */

use crate::config::{
    commands::ConfigManagerState,
    defaults::create_default_terminal_config,
    types::{CursorConfig, ShellConfig, TerminalBehaviorConfig, TerminalConfig},
};
use crate::mux::ShellManager;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};

use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::warn;

/// 终端配置更新请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalConfigUpdateRequest {
    /// 滚动缓冲区行数
    pub scrollback: Option<u32>,
    /// Shell 配置
    pub shell: Option<ShellConfig>,
    /// 光标配置
    pub cursor: Option<CursorConfig>,
    /// 终端行为配置
    pub behavior: Option<TerminalBehaviorConfig>,
}

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

/// 系统Shell检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemShellsResult {
    /// 可用的Shell列表
    pub available_shells: Vec<String>,
    /// 默认Shell
    pub default_shell: String,
    /// 当前用户的Shell
    pub user_shell: String,
}

// Tauri 命令接口

/// 获取终端配置
#[tauri::command]
pub async fn config_terminal_get(
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<TerminalConfig> {
    match state.toml_manager.config_get().await {
        Ok(config) => {
            let terminal_config = config.terminal.clone();
            Ok(api_success!(terminal_config))
        }
        Err(_) => Ok(api_error!("config.get_failed")),
    }
}

/// 更新终端配置
#[tauri::command]
pub async fn config_terminal_update(
    update_request: TerminalConfigUpdateRequest,
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    // 使用config_update方法更新配置
    let result = state
        .toml_manager
        .config_update(|config| {
            // 更新滚动缓冲区
            if let Some(scrollback) = update_request.scrollback {
                config.terminal.scrollback = scrollback;
            }

            // 更新Shell配置
            if let Some(shell) = update_request.shell {
                config.terminal.shell = shell;
            }

            // 更新光标配置
            if let Some(cursor) = update_request.cursor {
                config.terminal.cursor = cursor;
            }

            // 更新终端行为配置
            if let Some(behavior) = update_request.behavior {
                config.terminal.behavior = behavior;
            }

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
pub async fn config_terminal_validate(
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
pub async fn config_terminal_reset_to_defaults(
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

/// 检测系统可用的Shell
#[tauri::command]
pub async fn config_terminal_detect_system_shells() -> TauriApiResult<SystemShellsResult> {
    let mut available_shells = Vec::new();

    // 常见的Shell路径
    let shell_paths = [
        "/bin/bash",
        "/bin/zsh",
        "/bin/fish",
        "/usr/bin/bash",
        "/usr/bin/zsh",
        "/usr/bin/fish",
        "/usr/local/bin/bash",
        "/usr/local/bin/zsh",
        "/usr/local/bin/fish",
        "/opt/homebrew/bin/bash",
        "/opt/homebrew/bin/zsh",
        "/opt/homebrew/bin/fish",
    ];

    for shell_path in &shell_paths {
        if std::path::Path::new(shell_path).exists() {
            if let Some(shell_name) = std::path::Path::new(shell_path).file_name() {
                if let Some(name_str) = shell_name.to_str() {
                    if !available_shells.contains(&name_str.to_string()) {
                        available_shells.push(name_str.to_string());
                    }
                }
            }
        }
    }

    let default_shell = if cfg!(windows) {
        "powershell.exe".to_string()
    } else {
        "zsh".to_string()
    };

    let user_shell = if cfg!(windows) {
        // Windows平台通常没有SHELL环境变量，使用默认shell
        default_shell.clone()
    } else {
        // Unix平台从SHELL环境变量获取
        std::env::var("SHELL")
            .unwrap_or_else(|_| default_shell.clone())
            .split('/')
            .last()
            .unwrap_or(&default_shell)
            .to_string()
    };

    Ok(api_success!(SystemShellsResult {
        available_shells,
        default_shell,
        user_shell,
    }))
}

/// 更新光标配置
#[tauri::command]
pub async fn config_terminal_update_cursor(
    cursor_config: CursorConfig,
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    // 更新配置
    let result = state
        .toml_manager
        .config_update(|config| {
            config.terminal.cursor = cursor_config.clone();
            Ok(())
        })
        .await;

    match result {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.update_failed")),
    }
}

/// 更新终端行为配置
#[tauri::command]
pub async fn config_terminal_update_behavior(
    behavior_config: TerminalBehaviorConfig,
    state: State<'_, ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    // 更新配置
    let result = state
        .toml_manager
        .config_update(|config| {
            config.terminal.behavior = behavior_config.clone();
            Ok(())
        })
        .await;

    match result {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.update_failed")),
    }
}
/// 获取Shell信息
#[tauri::command]
pub async fn config_terminal_get_shell_info() -> TauriApiResult<String> {
    let info = ShellManager::get_cached_default_shell();
    Ok(api_success!(info.path))
}

/// 验证终端Shell路径（存根实现）
#[tauri::command]
pub async fn config_terminal_validate_shell_path(path: Option<String>) -> TauriApiResult<bool> {
    let Some(value) = path.filter(|p| !p.trim().is_empty()) else {
        return Ok(api_error!("common.invalid_params"));
    };

    let is_valid = ShellManager::validate_shell(value.trim());
    Ok(api_success!(is_valid))
}
