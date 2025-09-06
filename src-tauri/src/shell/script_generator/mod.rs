//! Shell Integration Script Generator
//!
//! 为不同shell生成集成脚本，用于注入OSC序列和回调

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
            ShellType::Bash => bash::generate_script(&self.config),
            ShellType::Zsh => zsh::generate_script(&self.config),
            ShellType::Fish => fish::generate_script(&self.config),
            ShellType::PowerShell => powershell::generate_script(&self.config),
            _ => {
                return Ok(String::new());
            }
        };

        Ok(script)
    }

    /// 检查shell配置文件中是否已存在集成代码
    pub fn is_integration_already_setup(&self, shell_type: &ShellType) -> Result<bool> {
        let config_path = self.get_shell_config_path(shell_type)?;

        if !config_path.exists() {
            return Ok(false);
        }

        let mut content = String::new();
        let mut file = std::fs::File::open(&config_path)
            .with_context(|| format!("Failed to open shell config file: {:?}", config_path))?;
        file.read_to_string(&mut content)
            .with_context(|| format!("Failed to read shell config file: {:?}", config_path))?;

        let marker = match shell_type {
            ShellType::Bash | ShellType::Zsh => "# OrbitX Integration Start",
            ShellType::Fish => "# OrbitX Integration Start",
            ShellType::PowerShell => "# OrbitX Integration Start",
            _ => return Ok(false),
        };

        Ok(content.contains(marker))
    }

    /// 安装集成脚本到shell配置文件
    pub fn install_integration(&self, shell_type: &ShellType) -> Result<()> {
        let script_content = self.generate_integration_script(shell_type)?;
        let config_path = self.get_shell_config_path(shell_type)?;

        // 确保配置目录存在
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        // 检查是否已经安装
        if self.is_integration_already_setup(shell_type)? {
            return Ok(());
        }

        // 追加集成脚本到配置文件
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config_path)
            .with_context(|| {
                format!("Failed to open config file for writing: {:?}", config_path)
            })?;

        writeln!(file, "\n{}", script_content)
            .with_context(|| format!("Failed to write integration script to: {:?}", config_path))?;

        Ok(())
    }

    /// 从shell配置文件中卸载集成脚本
    pub fn uninstall_integration(&self, shell_type: &ShellType) -> Result<()> {
        let config_path = self.get_shell_config_path(shell_type)?;

        if !config_path.exists() {
            return Ok(());
        }

        // 读取现有配置文件内容
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read shell config file: {:?}", config_path))?;

        // 移除集成代码块
        let cleaned_content = self.remove_integration_block(&content, shell_type);

        // 写回清理后的内容
        fs::write(&config_path, cleaned_content)
            .with_context(|| format!("Failed to write cleaned config file: {:?}", config_path))?;

        Ok(())
    }

    /// 获取shell配置文件路径
    fn get_shell_config_path(&self, shell_type: &ShellType) -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;

        let config_file = match shell_type {
            ShellType::Bash => ".bashrc",
            ShellType::Zsh => ".zshrc",
            ShellType::Fish => ".config/fish/config.fish",
            ShellType::PowerShell => {
                if cfg!(windows) {
                    "Documents/PowerShell/Microsoft.PowerShell_profile.ps1"
                } else {
                    ".config/powershell/Microsoft.PowerShell_profile.ps1"
                }
            }
            _ => return Err(anyhow::anyhow!("Unsupported shell type: {:?}", shell_type)),
        };

        Ok(home.join(config_file))
    }

    /// 从内容中移除集成代码块
    fn remove_integration_block(&self, content: &str, shell_type: &ShellType) -> String {
        let (start_marker, end_marker) = match shell_type {
            ShellType::Bash | ShellType::Zsh => {
                ("# OrbitX Integration Start", "# OrbitX Integration End")
            }
            ShellType::Fish => ("# OrbitX Integration Start", "# OrbitX Integration End"),
            ShellType::PowerShell => ("# OrbitX Integration Start", "# OrbitX Integration End"),
            _ => return content.to_string(),
        };

        let lines: Vec<&str> = content.lines().collect();
        let mut result_lines = Vec::new();
        let mut in_integration_block = false;

        for line in lines {
            if line.trim() == start_marker {
                in_integration_block = true;
                continue;
            }

            if line.trim() == end_marker {
                in_integration_block = false;
                continue;
            }

            if !in_integration_block {
                result_lines.push(line);
            }
        }

        result_lines.join("\n")
    }

    /// 获取集成脚本安装状态
    pub fn get_integration_status(&self, shell_type: &ShellType) -> Result<bool> {
        self.is_integration_already_setup(shell_type)
    }

    /// 生成Shell环境变量
    pub fn generate_env_vars(
        &self,
        _shell_type: &ShellType,
    ) -> std::collections::HashMap<String, String> {
        let mut env_vars = std::collections::HashMap::new();

        // 添加基本的OrbitX环境变量
        env_vars.insert("ORBITX_SHELL_INTEGRATION".to_string(), "1".to_string());

        if self.config.enable_command_tracking {
            env_vars.insert("ORBITX_COMMAND_TRACKING".to_string(), "1".to_string());
        }

        if self.config.enable_cwd_sync {
            env_vars.insert("ORBITX_CWD_SYNC".to_string(), "1".to_string());
        }

        if self.config.enable_title_updates {
            env_vars.insert("ORBITX_TITLE_UPDATES".to_string(), "1".to_string());
        }

        // 添加自定义环境变量
        for (key, value) in &self.config.custom_env_vars {
            env_vars.insert(key.clone(), value.clone());
        }

        env_vars
    }
}

impl Default for ShellScriptGenerator {
    fn default() -> Self {
        Self::new(ShellIntegrationConfig::default())
    }
}

// 导出技术实现模块
pub mod bash;
pub mod fish;
pub mod powershell;
pub mod zsh;

// 重新导出主要类型，保持向后兼容
pub use bash::generate_script as generate_bash_script;
pub use fish::generate_script as generate_fish_script;
pub use powershell::generate_script as generate_powershell_script;
pub use zsh::generate_script as generate_zsh_script;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_type_detection() {
        assert_eq!(ShellType::from_program("bash"), ShellType::Bash);
        assert_eq!(ShellType::from_program("/bin/bash"), ShellType::Bash);
        assert_eq!(ShellType::from_program("zsh"), ShellType::Zsh);
        assert_eq!(
            ShellType::from_program("/usr/local/bin/zsh"),
            ShellType::Zsh
        );
        assert_eq!(ShellType::from_program("fish"), ShellType::Fish);
        assert_eq!(ShellType::from_program("powershell"), ShellType::PowerShell);
        assert_eq!(ShellType::from_program("pwsh"), ShellType::PowerShell);
    }

    #[test]
    fn test_shell_display_names() {
        assert_eq!(ShellType::Bash.display_name(), "Bash");
        assert_eq!(ShellType::Zsh.display_name(), "Zsh");
        assert_eq!(ShellType::Fish.display_name(), "Fish");
        assert_eq!(ShellType::PowerShell.display_name(), "PowerShell");
        assert_eq!(ShellType::Cmd.display_name(), "Command Prompt");
        assert_eq!(ShellType::Nushell.display_name(), "Nushell");
    }

    #[test]
    fn test_integration_support() {
        assert!(ShellType::Bash.supports_integration());
        assert!(ShellType::Zsh.supports_integration());
        assert!(ShellType::Fish.supports_integration());
        assert!(ShellType::PowerShell.supports_integration());
        assert!(!ShellType::Cmd.supports_integration());
        assert!(!ShellType::Nushell.supports_integration());
        assert!(!ShellType::Unknown("custom".to_string()).supports_integration());
    }

    #[test]
    fn test_shell_script_generator_creation() {
        let generator = ShellScriptGenerator::default();
        assert!(generator.config.enable_command_tracking);
        assert!(generator.config.enable_cwd_sync);
        assert!(generator.config.enable_title_updates);
        assert!(generator.config.custom_env_vars.is_empty());
    }
}
