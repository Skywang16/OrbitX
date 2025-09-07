//! PowerShell Integration Script Generator
//!
//! 为 PowerShell 生成集成脚本，包括命令跟踪、CWD同步等功能

use super::ShellIntegrationConfig;

/// 生成 PowerShell 集成脚本
pub fn generate_script(config: &ShellIntegrationConfig) -> String {
    let mut script = String::new();

    // 检查是否已经注入过
    script.push_str(
        r#"
# OrbitX Shell Integration for PowerShell
if ($env:ORBITX_SHELL_INTEGRATION_LOADED) {
    return
}
$env:ORBITX_SHELL_INTEGRATION_LOADED = "1"
"#,
    );

    // CWD同步功能
    if config.enable_cwd_sync {
        script.push_str(
            r#"
# CWD同步函数
function global:__orbitx_update_cwd {
    $hostname = [System.Environment]::MachineName
    $pwd = Get-Location
    Write-Host "`e]7;file://$hostname$pwd`e\" -NoNewline
}
"#,
        );
    }

    // 命令跟踪功能
    if config.enable_command_tracking {
        script.push_str(
            r#"
# Shell Integration支持 (OSC 133)
function global:__orbitx_preexec {
    Write-Host "`e]133;C`e\" -NoNewline
}

function global:__orbitx_postcmd {
    param($ExitCode)
    Write-Host "`e]133;D;$ExitCode`e\" -NoNewline
    __orbitx_update_cwd
    Write-Host "`e]133;A`e\" -NoNewline
}

# 设置提示符钩子
$global:__orbitx_original_prompt = $function:prompt

function global:prompt {
    # 调用原始提示符
    $result = & $__orbitx_original_prompt
    
    # 发送提示符开始标记
    Write-Host "`e]133;A`e\" -NoNewline
    
    # 在用户输入命令前发送B标记
    $Host.UI.RawUI.KeyAvailable | Out-Null
    
    return $result
}

# 使用PowerShell事件系统监控命令执行
$null = Register-EngineEvent -SourceIdentifier PowerShell.Exiting -Action {
    __orbitx_postcmd -ExitCode $LASTEXITCODE
}
"#,
        );
    } else if config.enable_cwd_sync {
        // 只启用CWD同步
        script.push_str(
            r#"
# 仅CWD同步 - 通过提示符函数
$global:__orbitx_original_prompt = $function:prompt

function global:prompt {
    __orbitx_update_cwd
    & $__orbitx_original_prompt
}
"#,
        );
    }

    // 窗口标题更新
    if config.enable_title_updates {
        script.push_str(
            r#"
# 窗口标题更新
function global:__orbitx_update_title {
    $title = "$env:USERNAME@$([System.Environment]::MachineName):$(Get-Location)"
    $Host.UI.RawUI.WindowTitle = $title
    Write-Host "`e]2;$title`e\" -NoNewline
}

# 添加到提示符函数
$global:__orbitx_title_prompt = $function:prompt

function global:prompt {
    __orbitx_update_title
    & $__orbitx_title_prompt
}
"#,
        );
    }

    // 添加自定义环境变量
    if !config.custom_env_vars.is_empty() {
        script.push_str("\n# 自定义环境变量\n");
        for (key, value) in &config.custom_env_vars {
            script.push_str(&format!("$env:{} = \"{}\"\n", key, value));
        }
    }

    // 初始化
    script.push_str(
        r#"
# 初始化CWD和标题
try {
    __orbitx_update_cwd
    if (Get-Command __orbitx_update_title -ErrorAction SilentlyContinue) {
        __orbitx_update_title
    }
} catch {
    # 静默忽略初始化错误
}
"#,
    );

    script
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_basic_powershell_script_generation() {
        let config = ShellIntegrationConfig::default();
        let script = generate_script(&config);

        assert!(script.contains("# OrbitX Shell Integration for PowerShell"));
        assert!(script.contains("ORBITX_SHELL_INTEGRATION_LOADED"));
        assert!(script.contains("$env:"));
    }

    #[test]
    fn test_command_tracking_enabled() {
        let config = ShellIntegrationConfig {
            enable_command_tracking: true,
            ..Default::default()
        };
        let script = generate_script(&config);

        assert!(script.contains("__orbitx_preexec"));
        assert!(script.contains("__orbitx_postcmd"));
        assert!(script.contains("Register-EngineEvent"));
    }

    #[test]
    fn test_cwd_sync_enabled() {
        let config = ShellIntegrationConfig {
            enable_cwd_sync: true,
            ..Default::default()
        };
        let script = generate_script(&config);

        assert!(script.contains("__orbitx_update_cwd"));
        assert!(script.contains("Get-Location"));
    }

    #[test]
    fn test_title_updates_enabled() {
        let config = ShellIntegrationConfig {
            enable_title_updates: true,
            ..Default::default()
        };
        let script = generate_script(&config);

        assert!(script.contains("__orbitx_update_title"));
        assert!(script.contains("WindowTitle"));
    }

    #[test]
    fn test_custom_env_vars() {
        let mut custom_vars = HashMap::new();
        custom_vars.insert("ORBITX_CUSTOM".to_string(), "test_value".to_string());

        let config = ShellIntegrationConfig {
            custom_env_vars: custom_vars,
            ..Default::default()
        };
        let script = generate_script(&config);

        assert!(script.contains("$env:ORBITX_CUSTOM = \"test_value\""));
    }
}
