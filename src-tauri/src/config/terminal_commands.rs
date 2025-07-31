/*!
 * 终端配置管理命令
 *
 * 提供终端相关配置的专用管理接口，包括 Shell 配置、光标配置、
 * 行为配置等的获取、更新和验证功能。
 */

use crate::config::{
    commands::ConfigManagerState,
    types::{CursorConfig, ShellConfig, TerminalBehaviorConfig, TerminalConfig},
};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{debug, error, info, warn};

/// 终端配置更新请求
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct TerminalConfigValidationResult {
    /// 是否有效
    pub is_valid: bool,
    /// 错误信息列表
    pub errors: Vec<String>,
    /// 警告信息列表
    pub warnings: Vec<String>,
    /// 验证的配置项
    pub validated_fields: Vec<String>,
}

/// Shell 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellInfo {
    /// Shell 名称
    pub name: String,
    /// Shell 路径
    pub path: String,
    /// 显示名称
    pub display_name: String,
    /// 是否可用
    pub available: bool,
}

/// 系统 Shell 检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemShellsResult {
    /// 可用的 Shell 列表
    pub available_shells: Vec<ShellInfo>,
    /// 默认 Shell
    pub default_shell: Option<ShellInfo>,
    /// 当前配置的 Shell
    pub current_shell: Option<ShellInfo>,
}

// ============================================================================
// 终端配置管理命令
// ============================================================================

/// 获取终端配置
///
/// # Returns
/// 返回当前的终端配置
#[tauri::command]
pub async fn get_terminal_config(
    state: State<'_, ConfigManagerState>,
) -> Result<TerminalConfig, String> {
    debug!("开始获取终端配置");

    let config = state.config.lock().await;
    let terminal_config = config.terminal.clone();

    info!("成功获取终端配置");
    Ok(terminal_config)
}

/// 更新终端配置
///
/// # Arguments
/// * `update_request` - 终端配置更新请求
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn update_terminal_config(
    update_request: TerminalConfigUpdateRequest,
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("开始更新终端配置: {:?}", update_request);

    // 验证更新请求
    validate_terminal_config_update(&update_request).map_err(|e| {
        error!("终端配置更新验证失败: {}", e);
        e.to_string()
    })?;

    // 更新配置
    {
        let mut config = state.config.lock().await;

        if let Some(scrollback) = update_request.scrollback {
            config.terminal.scrollback = scrollback;
        }

        if let Some(shell) = update_request.shell {
            config.terminal.shell = shell;
        }

        if let Some(cursor) = update_request.cursor {
            config.terminal.cursor = cursor;
        }

        if let Some(behavior) = update_request.behavior {
            config.terminal.behavior = behavior;
        }
    }

    // 保存配置到文件
    let config = state.config.lock().await;
    crate::config::commands::ConfigManagerState::save_config_to_file(&config, &state.config_path)
        .await
        .map_err(|e| {
            error!("保存终端配置失败: {}", e);
            e.to_string()
        })?;

    info!("终端配置更新成功");
    Ok(())
}

/// 验证终端配置
///
/// # Returns
/// 返回验证结果
#[tauri::command]
pub async fn validate_terminal_config(
    state: State<'_, ConfigManagerState>,
) -> Result<TerminalConfigValidationResult, String> {
    debug!("开始验证终端配置");

    let config = state.config.lock().await;
    let terminal_config = &config.terminal;

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut validated_fields = Vec::new();

    // 验证滚动缓冲区
    validated_fields.push("scrollback".to_string());
    if terminal_config.scrollback == 0 {
        warnings.push("滚动缓冲区设置为 0，可能影响使用体验".to_string());
    } else if terminal_config.scrollback > 100000 {
        warnings.push("滚动缓冲区过大，可能消耗大量内存".to_string());
    }

    // 验证 Shell 配置
    validated_fields.push("shell".to_string());
    if terminal_config.shell.default_shell.is_empty() {
        errors.push("Shell 路径不能为空".to_string());
    } else {
        // 验证 Shell 路径是否存在
        if !validate_shell_path_internal(&terminal_config.shell.default_shell).await {
            errors.push(format!(
                "Shell 路径不存在或不可执行: {}",
                terminal_config.shell.default_shell
            ));
        }
    }

    // 验证工作目录
    if terminal_config.shell.working_directory.is_empty() {
        errors.push("工作目录不能为空".to_string());
    }

    // 验证光标配置
    validated_fields.push("cursor".to_string());
    if terminal_config.cursor.thickness < 0.0 || terminal_config.cursor.thickness > 1.0 {
        errors.push("光标粗细必须在 0.0 到 1.0 之间".to_string());
    }

    // 验证光标颜色格式
    if !is_valid_color(&terminal_config.cursor.color) {
        errors.push(format!(
            "光标颜色格式无效: {}",
            terminal_config.cursor.color
        ));
    }

    validated_fields.push("behavior".to_string());

    let is_valid = errors.is_empty();

    let result = TerminalConfigValidationResult {
        is_valid,
        errors,
        warnings,
        validated_fields,
    };

    info!("终端配置验证完成，结果: {:?}", result);
    Ok(result)
}

/// 重置终端配置为默认值
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn reset_terminal_config_to_defaults(
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("开始重置终端配置为默认值");

    let default_terminal_config = crate::config::defaults::create_default_terminal_config();

    {
        let mut config = state.config.lock().await;
        config.terminal = default_terminal_config;
    }

    // 保存配置到文件
    let config = state.config.lock().await;
    crate::config::commands::ConfigManagerState::save_config_to_file(&config, &state.config_path)
        .await
        .map_err(|e| {
            error!("保存重置后的终端配置失败: {}", e);
            e.to_string()
        })?;

    info!("终端配置已重置为默认值");
    Ok(())
}

// ============================================================================
// Shell 管理命令
// ============================================================================

/// 检测系统可用的 Shell
///
/// # Returns
/// 返回系统 Shell 检测结果
#[tauri::command]
pub async fn detect_system_shells(
    state: State<'_, ConfigManagerState>,
) -> Result<SystemShellsResult, String> {
    debug!("开始检测系统可用的 Shell");

    let available_shells = detect_available_shells().await;
    let default_shell = get_system_default_shell().await;

    // 获取当前配置的 Shell
    let current_shell = {
        let config = state.config.lock().await;
        let current_shell_path = &config.terminal.shell.default_shell;

        available_shells
            .iter()
            .find(|shell| shell.path == *current_shell_path)
            .cloned()
    };

    let result = SystemShellsResult {
        available_shells,
        default_shell,
        current_shell,
    };

    info!(
        "系统 Shell 检测完成，找到 {} 个可用 Shell",
        result.available_shells.len()
    );
    Ok(result)
}

/// 验证 Shell 路径
///
/// # Arguments
/// * `shell_path` - Shell 路径
///
/// # Returns
/// 返回验证结果
#[tauri::command]
pub async fn validate_terminal_shell_path(shell_path: String) -> Result<bool, String> {
    debug!("验证 Shell 路径: {}", shell_path);

    let is_valid = validate_shell_path_internal(&shell_path).await;

    info!("Shell 路径验证结果: {} -> {}", shell_path, is_valid);
    Ok(is_valid)
}

/// 获取 Shell 信息
///
/// # Arguments
/// * `shell_path` - Shell 路径
///
/// # Returns
/// 返回 Shell 信息
#[tauri::command]
pub async fn get_shell_info(shell_path: String) -> Result<Option<ShellInfo>, String> {
    debug!("获取 Shell 信息: {}", shell_path);

    let shell_info = get_shell_info_by_path(&shell_path).await;

    match &shell_info {
        Some(info) => info!("获取到 Shell 信息: {:?}", info),
        None => warn!("未找到 Shell 信息: {}", shell_path),
    }

    Ok(shell_info)
}

// ============================================================================
// 光标配置命令
// ============================================================================

/// 更新光标配置
///
/// # Arguments
/// * `cursor_config` - 新的光标配置
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn update_cursor_config(
    cursor_config: CursorConfig,
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("开始更新光标配置: {:?}", cursor_config);

    // 验证光标配置
    validate_cursor_config(&cursor_config).map_err(|e| {
        error!("光标配置验证失败: {}", e);
        e.to_string()
    })?;

    // 更新配置
    {
        let mut config = state.config.lock().await;
        config.terminal.cursor = cursor_config;
    }

    // 保存配置到文件
    let config = state.config.lock().await;
    crate::config::commands::ConfigManagerState::save_config_to_file(&config, &state.config_path)
        .await
        .map_err(|e| {
            error!("保存光标配置失败: {}", e);
            e.to_string()
        })?;

    info!("光标配置更新成功");
    Ok(())
}

// ============================================================================
// 终端行为配置命令
// ============================================================================

/// 更新终端行为配置
///
/// # Arguments
/// * `behavior_config` - 新的终端行为配置
///
/// # Returns
/// 返回操作结果
#[tauri::command]
pub async fn update_terminal_behavior_config(
    behavior_config: TerminalBehaviorConfig,
    state: State<'_, ConfigManagerState>,
) -> Result<(), String> {
    debug!("开始更新终端行为配置: {:?}", behavior_config);

    // 更新配置
    {
        let mut config = state.config.lock().await;
        config.terminal.behavior = behavior_config;
    }

    // 保存配置到文件
    let config = state.config.lock().await;
    crate::config::commands::ConfigManagerState::save_config_to_file(&config, &state.config_path)
        .await
        .map_err(|e| {
            error!("保存终端行为配置失败: {}", e);
            e.to_string()
        })?;

    info!("终端行为配置更新成功");
    Ok(())
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 验证终端配置更新请求
fn validate_terminal_config_update(
    update_request: &TerminalConfigUpdateRequest,
) -> AnyhowResult<()> {
    if let Some(scrollback) = update_request.scrollback {
        if scrollback > 1000000 {
            return Err(anyhow!("滚动缓冲区不能超过 1,000,000 行"));
        }
    }

    if let Some(ref cursor) = update_request.cursor {
        validate_cursor_config(cursor).context("光标配置验证失败")?;
    }

    if let Some(ref shell) = update_request.shell {
        if shell.default_shell.is_empty() {
            return Err(anyhow!("Shell 路径不能为空"));
        }
        if shell.working_directory.is_empty() {
            return Err(anyhow!("工作目录不能为空"));
        }
    }

    Ok(())
}

/// 验证光标配置
fn validate_cursor_config(cursor_config: &CursorConfig) -> AnyhowResult<()> {
    if cursor_config.thickness < 0.0 || cursor_config.thickness > 1.0 {
        return Err(anyhow!("光标粗细必须在 0.0 到 1.0 之间"));
    }

    if !is_valid_color(&cursor_config.color) {
        return Err(anyhow!("光标颜色格式无效: {}", cursor_config.color));
    }

    Ok(())
}

/// 验证颜色格式
fn is_valid_color(color: &str) -> bool {
    // 简单的十六进制颜色验证
    if color.starts_with('#') && color.len() == 7 {
        color[1..].chars().all(|c| c.is_ascii_hexdigit())
    } else {
        false
    }
}

/// 验证 Shell 路径是否存在且可执行
async fn validate_shell_path_internal(shell_path: &str) -> bool {
    use std::path::Path;

    let path = Path::new(shell_path);
    if !path.exists() {
        return false;
    }

    // 在 Unix 系统上检查可执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = path.metadata() {
            let permissions = metadata.permissions();
            return permissions.mode() & 0o111 != 0;
        }
    }

    // 在 Windows 上简单检查文件存在性
    #[cfg(windows)]
    {
        return path.is_file();
    }

    false
}

/// 检测系统可用的 Shell
async fn detect_available_shells() -> Vec<ShellInfo> {
    let mut shells = Vec::new();

    // 常见的 Shell 路径
    let common_shells = [
        ("bash", "/bin/bash", "Bash"),
        ("zsh", "/bin/zsh", "Zsh"),
        ("fish", "/usr/bin/fish", "Fish"),
        ("sh", "/bin/sh", "Bourne Shell"),
        ("dash", "/bin/dash", "Dash"),
        ("tcsh", "/bin/tcsh", "Tcsh"),
        ("csh", "/bin/csh", "C Shell"),
    ];

    for (name, path, display_name) in &common_shells {
        let available = validate_shell_path_internal(path).await;
        shells.push(ShellInfo {
            name: name.to_string(),
            path: path.to_string(),
            display_name: display_name.to_string(),
            available,
        });
    }

    // 在 Windows 上添加 PowerShell 和 CMD
    #[cfg(windows)]
    {
        let windows_shells = [
            ("powershell", "powershell.exe", "PowerShell"),
            ("cmd", "cmd.exe", "Command Prompt"),
            ("pwsh", "pwsh.exe", "PowerShell Core"),
        ];

        for (name, path, display_name) in &windows_shells {
            let available = validate_shell_path_internal(path).await;
            shells.push(ShellInfo {
                name: name.to_string(),
                path: path.to_string(),
                display_name: display_name.to_string(),
                available,
            });
        }
    }

    // 只返回可用的 Shell
    shells.into_iter().filter(|shell| shell.available).collect()
}

/// 获取系统默认 Shell
async fn get_system_default_shell() -> Option<ShellInfo> {
    // 尝试从环境变量获取
    if let Ok(shell_path) = std::env::var("SHELL") {
        if let Some(shell_info) = get_shell_info_by_path(&shell_path).await {
            return Some(shell_info);
        }
    }

    // 回退到常见的默认 Shell
    let default_shells = ["/bin/zsh", "/bin/bash", "/bin/sh"];

    for shell_path in &default_shells {
        if validate_shell_path_internal(shell_path).await {
            if let Some(shell_info) = get_shell_info_by_path(shell_path).await {
                return Some(shell_info);
            }
        }
    }

    None
}

/// 根据路径获取 Shell 信息
async fn get_shell_info_by_path(shell_path: &str) -> Option<ShellInfo> {
    use std::path::Path;

    let path = Path::new(shell_path);
    if !validate_shell_path_internal(shell_path).await {
        return None;
    }

    let name = path.file_name()?.to_str()?.to_string();

    let display_name = match name.as_str() {
        "bash" => "Bash".to_string(),
        "zsh" => "Zsh".to_string(),
        "fish" => "Fish".to_string(),
        "sh" => "Bourne Shell".to_string(),
        "dash" => "Dash".to_string(),
        "tcsh" => "Tcsh".to_string(),
        "csh" => "C Shell".to_string(),
        "powershell.exe" => "PowerShell".to_string(),
        "cmd.exe" => "Command Prompt".to_string(),
        "pwsh.exe" => "PowerShell Core".to_string(),
        _ => name.clone(),
    };

    Some(ShellInfo {
        name,
        path: shell_path.to_string(),
        display_name,
        available: true,
    })
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::CursorStyle;

    #[tokio::test]
    async fn test_validate_terminal_config_update() {
        let update_request = TerminalConfigUpdateRequest {
            scrollback: Some(1000),
            shell: None,
            cursor: None,
            behavior: None,
        };

        assert!(validate_terminal_config_update(&update_request).is_ok());

        let invalid_request = TerminalConfigUpdateRequest {
            scrollback: Some(2000000), // 超过限制
            shell: None,
            cursor: None,
            behavior: None,
        };

        assert!(validate_terminal_config_update(&invalid_request).is_err());
    }

    #[tokio::test]
    async fn test_validate_cursor_config() {
        let valid_cursor = CursorConfig {
            style: CursorStyle::Block,
            blink: true,
            color: "#ffffff".to_string(),
            thickness: 0.5,
        };

        assert!(validate_cursor_config(&valid_cursor).is_ok());

        let invalid_cursor = CursorConfig {
            style: CursorStyle::Block,
            blink: true,
            color: "invalid_color".to_string(),
            thickness: 1.5, // 超过范围
        };

        assert!(validate_cursor_config(&invalid_cursor).is_err());
    }

    #[test]
    fn test_is_valid_color() {
        assert!(is_valid_color("#ffffff"));
        assert!(is_valid_color("#000000"));
        assert!(is_valid_color("#ff0000"));
        assert!(!is_valid_color("ffffff"));
        assert!(!is_valid_color("#fff"));
        assert!(!is_valid_color("#gggggg"));
    }

    #[tokio::test]
    async fn test_detect_available_shells() {
        let shells = detect_available_shells().await;
        // 至少应该有一个可用的 Shell
        assert!(!shells.is_empty());

        // 所有返回的 Shell 都应该是可用的
        for shell in shells {
            assert!(shell.available);
            assert!(!shell.name.is_empty());
            assert!(!shell.path.is_empty());
            assert!(!shell.display_name.is_empty());
        }
    }

    #[tokio::test]
    async fn test_get_system_default_shell() {
        let default_shell = get_system_default_shell().await;
        if let Some(shell) = default_shell {
            assert!(shell.available);
            assert!(!shell.name.is_empty());
            assert!(!shell.path.is_empty());
        }
    }
}
