//! Shell Integration Script Generator
//!
//! 为不同shell生成集成脚本，用于注入OSC序列和回调

use crate::config::paths::ConfigPaths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

/// Shell类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Cmd,
    Nushell,
    Unknown(String),
}

impl ShellType {
    /// 从shell程序路径检测shell类型
    pub fn from_program(program: &str) -> Self {
        let program_name = std::path::Path::new(program)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(program)
            .to_lowercase();

        match program_name.as_str() {
            "bash" => Self::Bash,
            "zsh" => Self::Zsh,
            "fish" => Self::Fish,
            "powershell" | "pwsh" | "powershell.exe" | "pwsh.exe" => Self::PowerShell,
            "cmd" | "cmd.exe" => Self::Cmd,
            "nu" | "nushell" => Self::Nushell,
            _ => Self::Unknown(program_name),
        }
    }

    /// 获取shell的友好名称
    pub fn display_name(&self) -> &str {
        match self {
            Self::Bash => "Bash",
            Self::Zsh => "Zsh",
            Self::Fish => "Fish",
            Self::PowerShell => "PowerShell",
            Self::Cmd => "Command Prompt",
            Self::Nushell => "Nushell",
            Self::Unknown(name) => name,
        }
    }

    /// 检查是否支持Shell Integration
    pub fn supports_integration(&self) -> bool {
        matches!(self, Self::Bash | Self::Zsh | Self::Fish | Self::PowerShell)
    }
}

/// Shell Integration配置
#[derive(Debug, Clone)]
pub struct ShellIntegrationConfig {
    /// 是否启用命令跟踪
    pub enable_command_tracking: bool,
    /// 是否启用CWD同步
    pub enable_cwd_sync: bool,
    /// 是否启用窗口标题更新
    pub enable_title_updates: bool,
    /// 自定义环境变量
    pub custom_env_vars: HashMap<String, String>,
}

impl Default for ShellIntegrationConfig {
    fn default() -> Self {
        Self {
            enable_command_tracking: true,
            enable_cwd_sync: true,
            enable_title_updates: true,
            custom_env_vars: HashMap::new(),
        }
    }
}

/// Shell脚本生成器
pub struct ShellScriptGenerator {
    config: ShellIntegrationConfig,
}

impl ShellScriptGenerator {
    /// 创建新的脚本生成器
    pub fn new(config: ShellIntegrationConfig) -> Self {
        Self { config }
    }

    /// 生成给定shell类型的集成脚本
    pub fn generate_integration_script(&self, shell_type: &ShellType) -> Result<String> {
        let script = match shell_type {
            ShellType::Bash => self.generate_bash_script(),
            ShellType::Zsh => self.generate_zsh_script(),
            ShellType::Fish => self.generate_fish_script(),
            ShellType::PowerShell => self.generate_powershell_script(),
            _ => {
                return Ok(String::new());
            }
        };

        Ok(script)
    }

    /// 生成Bash集成脚本
    fn generate_bash_script(&self) -> String {
        let mut script = String::new();

        // 检查是否已经注入过
        script.push_str(
            r#"
# OrbitX Shell Integration for Bash
if [[ -n "$ORBITX_SHELL_INTEGRATION" ]]; then
    return 0
fi
export ORBITX_SHELL_INTEGRATION=1
"#,
        );

        // CWD同步功能
        if self.config.enable_cwd_sync {
            script.push_str(
                r#"
# CWD同步函数
__orbitx_update_cwd() {
    printf '\e]7;file://%s%s\e\\' "$HOSTNAME" "$PWD"
}
"#,
            );
        }

        // 命令跟踪功能（仅 VS Code 协议）
        if self.config.enable_command_tracking {
            script.push_str(
                r#"
# VS Code协议命令跟踪
__orbitx_preexec() {
    printf '\e]633;B\e\\'
    printf '\e]633;C\e\\'
}

__orbitx_precmd() {
    local exit_code=$?
    printf '\e]633;D;%d\e\\' "$exit_code"
    __orbitx_update_cwd
}
"#,
            );

            // 集成到Bash钩子系统
            script.push_str(
                r#"
# 集成到Bash提示符系统
if [[ -z "$PROMPT_COMMAND" ]]; then
    PROMPT_COMMAND="__orbitx_precmd"
else
    PROMPT_COMMAND="__orbitx_precmd;$PROMPT_COMMAND"
fi

# 设置preexec钩子
if [[ -z "$preexec_functions" ]]; then
    preexec_functions=(__orbitx_preexec)
else
    preexec_functions+=(__orbitx_preexec)
fi

# 如果没有preexec支持，使用trap DEBUG的fallback
if ! type preexec_invoke_exec &>/dev/null; then
    __orbitx_debug_trap() {
        if [[ "$BASH_COMMAND" != "__orbitx_precmd"* ]]; then
            __orbitx_preexec
        fi
    }
    trap '__orbitx_debug_trap' DEBUG
fi
"#,
            );
        } else if self.config.enable_cwd_sync {
            // 只启用CWD同步
            script.push_str(
                r#"
# 仅CWD同步
if [[ -z "$PROMPT_COMMAND" ]]; then
    PROMPT_COMMAND="__orbitx_update_cwd"
else
    PROMPT_COMMAND="__orbitx_update_cwd;$PROMPT_COMMAND"
fi
"#,
            );
        }

        // 窗口标题更新
        if self.config.enable_title_updates {
            script.push_str(
                r#"
# 窗口标题更新
__orbitx_update_title() {
    printf '\e]2;%s@%s:%s\e\\' "$USER" "$HOSTNAME" "${PWD/#$HOME/~}"
}

if [[ -z "$PROMPT_COMMAND" ]]; then
    PROMPT_COMMAND="__orbitx_update_title"
else
    PROMPT_COMMAND="__orbitx_update_title;$PROMPT_COMMAND"
fi
"#,
            );
        }

        script
    }

    /// 生成Zsh集成脚本
    fn generate_zsh_script(&self) -> String {
        let mut script = String::new();

        // Start of script and check for existing integration
        script.push_str(
            r#"
# OrbitX Shell Integration for Zsh
if [[ -n "$ORBITX_SHELL_INTEGRATION" ]]; then
    return 0
fi
export ORBITX_SHELL_INTEGRATION=1

# Shell-side logging for debugging
__orbitx_log() {
    echo "$(date): $1" >> "${HOME}/.orbitx_shell_debug.log"
}
__orbitx_log "Zsh integration script started."
"#,
        );

        // Define core functions
        script.push_str(
            r#"
__orbitx_update_cwd() {
    __orbitx_log "Updating CWD to $PWD"
    printf '\e]7;file://%s%s\e\\' "$HOSTNAME" "$PWD"
}

__orbitx_update_title() {
    __orbitx_log "Updating title."
    printf '\e]2;%s@%s:%s\e\\' "$USER" "$HOSTNAME" "${PWD/#$HOME/~}"
}
"#,
        );

        // Command tracking hooks
        if self.config.enable_command_tracking {
            script.push_str(
                r#"
__orbitx_prompt_start() { printf '\e]633;A\e\\'; }
__orbitx_prompt_end() { printf '\e]633;B\e\\'; }
__orbitx_command_start() { printf '\e]633;C\e\\'; }
__orbitx_command_end() { printf '\e]633;D;%d\e\\' "$?"; }
__orbitx_precmd() { __orbitx_command_end; }
__orbitx_preexec() { __orbitx_command_start; }
"#,
            );
        }

        // Register hooks
        script.push_str(
            r#"
# Ensure hook function arrays exist
if ! (( ${+precmd_functions} )); then precmd_functions=(); fi
if ! (( ${+preexec_functions} )); then preexec_functions=(); fi
if ! (( ${+chpwd_functions} )); then chpwd_functions=(); fi
"#,
        );

        let mut hooks: Vec<(&str, &str)> = Vec::new();
        if self.config.enable_command_tracking {
            hooks.push(("precmd_functions", "__orbitx_prompt_start"));
            hooks.push(("precmd_functions", "__orbitx_precmd"));
            hooks.push(("preexec_functions", "__orbitx_prompt_end"));
            hooks.push(("preexec_functions", "__orbitx_preexec"));
        }
        if self.config.enable_cwd_sync {
            hooks.push(("chpwd_functions", "__orbitx_update_cwd"));
        }
        if self.config.enable_title_updates {
            hooks.push(("chpwd_functions", "__orbitx_update_title"));
            hooks.push(("precmd_functions", "__orbitx_update_title"));
        }

        for (hook_array, function_name) in hooks {
            // Use a more robust method to add hooks, checking for existence before adding.
            let hook_line = format!(
                "if [[ ! \" ${{{hook_array}[@]}} \" =~ \" {function_name} \" ]]; then {hook_array}+=({function_name}); fi\n",
                hook_array = hook_array,
                function_name = function_name
            );
            script.push_str(&hook_line);
        }

        // Final setup
        script.push_str(
            r#"
__orbitx_log "Hook functions registered."

# Initial update
if [[ -n "$ZSH_VERSION" ]]; then
    __orbitx_update_cwd
    __orbitx_update_title
fi
"#,
        );

        script
    }

    /// 生成Fish集成脚本
    fn generate_fish_script(&self) -> String {
        let mut script = String::new();

        // 检查是否已经注入过
        script.push_str(
            r#"
# OrbitX Shell Integration for Fish
if set -q ORBITX_SHELL_INTEGRATION
    exit 0
end
set -g ORBITX_SHELL_INTEGRATION 1
"#,
        );

        // CWD同步功能
        if self.config.enable_cwd_sync {
            script.push_str(
                r#"
# CWD同步函数
function __orbitx_update_cwd
    printf '\e]7;file://%s%s\e\\' (hostname) (pwd)
end
"#,
            );
        }

        // 命令跟踪功能（仅 VS Code 协议）
        if self.config.enable_command_tracking {
            script.push_str(
                r#"
# VS Code协议命令跟踪
function __orbitx_preexec --on-event fish_preexec
    printf '\e]633;C\e\\'
end

function __orbitx_postexec --on-event fish_postexec
    printf '\e]633;D;%d\e\\' $status
    __orbitx_update_cwd
end
"#,
            );
        } else if self.config.enable_cwd_sync {
            // 只启用CWD同步
            script.push_str(
                r#"
# 仅CWD同步
function __orbitx_postexec --on-event fish_postexec
    __orbitx_update_cwd
end
"#,
            );
        }

        // 窗口标题更新
        if self.config.enable_title_updates {
            script.push_str(
                r#"
# 窗口标题更新
function __orbitx_update_title
    printf '\e]2;%s@%s:%s\e\\' (whoami) (hostname) (prompt_pwd)
end

function __orbitx_title_postexec --on-event fish_postexec
    __orbitx_update_title
end
"#,
            );
        }

        script
    }

    /// 生成PowerShell集成脚本
    fn generate_powershell_script(&self) -> String {
        let mut script = String::new();

        // 检查是否已经注入过
        script.push_str(
            r#"
# OrbitX Shell Integration for PowerShell
if ($env:ORBITX_SHELL_INTEGRATION) {
    return
}
$env:ORBITX_SHELL_INTEGRATION = "1"
"#,
        );

        // CWD同步功能
        if self.config.enable_cwd_sync {
            script.push_str(
                r#"
# CWD同步函数
function Update-OrbitXCwd {
    $hostname = $env:COMPUTERNAME
    $currentPath = (Get-Location).Path
    Write-Host -NoNewline "`e]7;file://$hostname$currentPath`e\"
}
"#,
            );
        }

        // 命令跟踪功能（仅 VS Code 协议）
        if self.config.enable_command_tracking {
            script.push_str(
                r#"
# VS Code协议命令跟踪
function Invoke-OrbitXPreCommand {
    Write-Host -NoNewline "`e]633;C`e\"
}

function Invoke-OrbitXPostCommand {
    param($exitCode)
    Write-Host -NoNewline "`e]633;D;$exitCode`e\"
    Update-OrbitXCwd
}
"#,
            );

            // 集成到PowerShell提示符系统
            script.push_str(
                r#"
# 重写prompt函数
if (Test-Path Function:\prompt) {
    $global:OriginalPrompt = Get-Content Function:\prompt
}

function prompt {
    $exitCode = $LASTEXITCODE
    
    # 调用原始prompt或默认行为
    $promptText = if ($global:OriginalPrompt) {
        & $global:OriginalPrompt
    } else {
        "PS $($executionContext.SessionState.Path.CurrentLocation)> "
    }
    
    # 发送Shell Integration序列
    Invoke-OrbitXPostCommand -exitCode $exitCode
    
    return $promptText
}

# 设置PSReadLine的命令执行钩子（如果可用）
if (Get-Module -Name PSReadLine -ListAvailable) {
    Set-PSReadLineOption -AddToHistoryHandler {
        param($command)
        Invoke-OrbitXPreCommand
        return $true
    }
}
"#,
            );
        } else if self.config.enable_cwd_sync {
            // 只启用CWD同步
            script.push_str(
                r#"
# 仅CWD同步
if (Test-Path Function:\prompt) {
    $global:OriginalPrompt = Get-Content Function:\prompt
}

function prompt {
    $promptText = if ($global:OriginalPrompt) {
        & $global:OriginalPrompt
    } else {
        "PS $($executionContext.SessionState.Path.CurrentLocation)> "
    }
    
    Update-OrbitXCwd
    return $promptText
}
"#,
            );
        }

        // 窗口标题更新
        if self.config.enable_title_updates {
            script.push_str(
                r#"
# 窗口标题更新
function Update-OrbitXTitle {
    $title = "$env:USERNAME@$env:COMPUTERNAME - $(Split-Path -Leaf (Get-Location))"
    Write-Host -NoNewline "`e]2;$title`e\"
}

# 集成到prompt函数
if (Test-Path Function:\prompt) {
    $existingPrompt = Get-Content Function:\prompt
    function prompt {
        $result = & $existingPrompt
        Update-OrbitXTitle
        return $result
    }
} else {
    function prompt {
        Update-OrbitXTitle
        return "PS $($executionContext.SessionState.Path.CurrentLocation)> "
    }
}
"#,
            );
        }

        script
    }

    /// 生成shell环境变量设置
    pub fn generate_env_vars(&self, shell_type: &ShellType) -> HashMap<String, String> {
        let mut env_vars = HashMap::new();

        // 基本环境变量
        env_vars.insert("ORBITX_SHELL_INTEGRATION".to_string(), "1".to_string());
        env_vars.insert("TERM_PROGRAM".to_string(), "OrbitX".to_string());

        // Shell特定环境变量
        match shell_type {
            ShellType::Bash | ShellType::Zsh => {
                env_vars.insert("SHELL_INTEGRATION_ENABLED".to_string(), "true".to_string());
            }
            ShellType::Fish => {
                env_vars.insert("FISH_SHELL_INTEGRATION".to_string(), "true".to_string());
            }
            ShellType::PowerShell => {
                env_vars.insert(
                    "POWERSHELL_SHELL_INTEGRATION".to_string(),
                    "true".to_string(),
                );
            }
            _ => {}
        }

        // 添加自定义环境变量
        for (key, value) in &self.config.custom_env_vars {
            env_vars.insert(key.clone(), value.clone());
        }

        env_vars
    }

    /// 检测给定路径中的shell配置文件
    pub fn setup_shell_integration(
        &self,
        shell_type: &ShellType,
        paths: &ConfigPaths,
    ) -> Result<()> {
        if !shell_type.supports_integration() {
            return Ok(());
        }

        let script_content = self.generate_integration_script(shell_type)?;
        if script_content.is_empty() {
            return Ok(());
        }

        let script_path =
            paths.shell_integration_script_path(shell_type.display_name().to_lowercase().as_str());

        // 写入脚本文件
        fs::create_dir_all(script_path.parent().unwrap())?;
        fs::write(&script_path, script_content).with_context(|| {
            format!(
                "Failed to write shell integration script to {:?}",
                script_path
            )
        })?;

        // 更新shell配置文件
        if let Some(config_path) = self.find_best_shell_config_file(shell_type) {
            self.source_script_in_config(&config_path, &script_path)?;
        }

        Ok(())
    }

    fn find_best_shell_config_file(&self, shell_type: &ShellType) -> Option<PathBuf> {
        let home_dir = dirs::home_dir()?;
        let config_files = Self::detect_shell_config_files(shell_type);

        for file_name in config_files {
            let config_path = home_dir.join(file_name);
            if config_path.exists() {
                return Some(config_path);
            }
        }

        // 如果找不到存在的配置文件，则为zsh和bash创建默认的
        match shell_type {
            ShellType::Zsh => Some(home_dir.join(".zshrc")),
            ShellType::Bash => Some(home_dir.join(".bashrc")),
            _ => None,
        }
    }

    fn source_script_in_config(&self, config_path: &PathBuf, script_path: &PathBuf) -> Result<()> {
        let source_command = format!(
            "\n# Source OrbitX Shell Integration\nsource '{}'\n",
            script_path.display()
        );

        let mut file_content = String::new();
        if config_path.exists() {
            let mut file = fs::File::open(config_path)?;
            file.read_to_string(&mut file_content)?;
        }

        if file_content.contains(&source_command.trim()) {
            return Ok(());
        }

        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(config_path)
            .with_context(|| {
                format!(
                    "Failed to open or create shell config file at {:?}",
                    config_path
                )
            })?;

        file.write_all(source_command.as_bytes()).with_context(|| {
            format!("Failed to write to shell config file at {:?}", config_path)
        })?;

        Ok(())
    }

    /// 检测给定路径中的shell配置文件
    pub fn detect_shell_config_files(shell_type: &ShellType) -> Vec<String> {
        match shell_type {
            ShellType::Bash => vec![
                ".bashrc".to_string(),
                ".bash_profile".to_string(),
                ".profile".to_string(),
            ],
            ShellType::Zsh => vec![
                ".zshrc".to_string(),
                ".zprofile".to_string(),
                ".zsh_profile".to_string(),
            ],
            ShellType::Fish => vec!["config.fish".to_string()],
            ShellType::PowerShell => vec![
                "Microsoft.PowerShell_profile.ps1".to_string(),
                "profile.ps1".to_string(),
            ],
            _ => vec![],
        }
    }
}

impl Default for ShellScriptGenerator {
    fn default() -> Self {
        Self::new(ShellIntegrationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_type_detection() {
        assert_eq!(ShellType::from_program("/bin/bash"), ShellType::Bash);
        assert_eq!(ShellType::from_program("/usr/bin/zsh"), ShellType::Zsh);
        assert_eq!(ShellType::from_program("fish"), ShellType::Fish);
        assert_eq!(
            ShellType::from_program("powershell.exe"),
            ShellType::PowerShell
        );
        assert_eq!(
            ShellType::from_program("unknown"),
            ShellType::Unknown("unknown".to_string())
        );
    }

    #[test]
    fn test_shell_support() {
        assert!(ShellType::Bash.supports_integration());
        assert!(ShellType::Zsh.supports_integration());
        assert!(ShellType::Fish.supports_integration());
        assert!(ShellType::PowerShell.supports_integration());
        assert!(!ShellType::Cmd.supports_integration());
    }

    #[test]
    fn test_script_generation() {
        let generator = ShellScriptGenerator::default();

        // 测试Bash脚本生成
        let bash_script = generator
            .generate_integration_script(&ShellType::Bash)
            .unwrap();
        assert!(bash_script.contains("ORBITX_SHELL_INTEGRATION"));
        assert!(bash_script.contains("__orbitx_update_cwd"));

        // 测试Zsh脚本生成
        let zsh_script = generator
            .generate_integration_script(&ShellType::Zsh)
            .unwrap();
        assert!(zsh_script.contains("ORBITX_SHELL_INTEGRATION"));
        assert!(zsh_script.contains("precmd"));

        // 测试Fish脚本生成
        let fish_script = generator
            .generate_integration_script(&ShellType::Fish)
            .unwrap();
        assert!(fish_script.contains("ORBITX_SHELL_INTEGRATION"));
        assert!(fish_script.contains("fish_postexec"));
    }

    #[test]
    fn test_env_vars_generation() {
        let generator = ShellScriptGenerator::default();
        let env_vars = generator.generate_env_vars(&ShellType::Bash);

        assert_eq!(
            env_vars.get("ORBITX_SHELL_INTEGRATION"),
            Some(&"1".to_string())
        );
        assert_eq!(env_vars.get("TERM_PROGRAM"), Some(&"OrbitX".to_string()));
    }

    #[test]
    fn test_config_file_detection() {
        let bash_files = ShellScriptGenerator::detect_shell_config_files(&ShellType::Bash);
        assert!(bash_files.contains(&".bashrc".to_string()));

        let zsh_files = ShellScriptGenerator::detect_shell_config_files(&ShellType::Zsh);
        assert!(zsh_files.contains(&".zshrc".to_string()));

        let fish_files = ShellScriptGenerator::detect_shell_config_files(&ShellType::Fish);
        assert!(fish_files.contains(&"config.fish".to_string()));
    }
}
