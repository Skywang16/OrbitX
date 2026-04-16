use super::error::ShellScriptResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    Other(String),
}

impl ShellType {
    pub fn from_program(program: &str) -> Self {
        let program_name = std::path::Path::new(program)
            .file_name()
            .and_then(|n| n.to_str())
            .map_or_else(|| program.to_string(), ToString::to_string)
            .to_lowercase();

        match program_name.as_str() {
            "bash" => Self::Bash,
            "zsh" => Self::Zsh,
            "fish" => Self::Fish,
            name => Self::Other(name.to_string()),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Bash => "Bash",
            Self::Zsh => "Zsh",
            Self::Fish => "Fish",
            Self::Other(name) => name,
        }
    }

    pub fn supports_integration(&self) -> bool {
        matches!(self, Self::Bash | Self::Zsh | Self::Fish)
    }
}

#[derive(Debug, Clone)]
pub struct ShellIntegrationConfig {
    pub enable_command_tracking: bool,
    pub enable_cwd_sync: bool,
    pub enable_title_updates: bool,
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

pub struct ShellScriptGenerator {
    config: ShellIntegrationConfig,
}

impl ShellScriptGenerator {
    pub fn new(config: ShellIntegrationConfig) -> Self {
        Self { config }
    }

    pub fn generate_integration_script(&self, shell_type: &ShellType) -> ShellScriptResult<String> {
        let script = match shell_type {
            ShellType::Bash => bash::generate_script(&self.config),
            ShellType::Zsh => zsh::generate_script(&self.config),
            ShellType::Fish => fish::generate_script(&self.config),
            ShellType::Other(_) => String::new(),
        };

        Ok(script)
    }

    pub fn generate_env_vars(
        &self,
        _shell_type: &ShellType,
    ) -> std::collections::HashMap<String, String> {
        let mut env_vars = std::collections::HashMap::new();

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

pub mod bash;
pub mod fish;
pub mod zsh;

pub use bash::generate_script as generate_bash_script;
pub use fish::generate_script as generate_fish_script;
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
        assert_eq!(
            ShellType::from_program("/opt/homebrew/bin/fish"),
            ShellType::Fish
        );
        assert_eq!(
            ShellType::from_program("pwsh"),
            ShellType::Other("pwsh".to_string())
        );
    }

    #[test]
    fn test_shell_display_names() {
        assert_eq!(ShellType::Bash.display_name(), "Bash");
        assert_eq!(ShellType::Zsh.display_name(), "Zsh");
        assert_eq!(ShellType::Fish.display_name(), "Fish");
        assert_eq!(
            ShellType::Other("nushell".to_string()).display_name(),
            "nushell"
        );
    }

    #[test]
    fn test_integration_support() {
        assert!(ShellType::Bash.supports_integration());
        assert!(ShellType::Zsh.supports_integration());
        assert!(ShellType::Fish.supports_integration());
        assert!(!ShellType::Other("sh".to_string()).supports_integration());
    }

    #[test]
    fn test_other_shell_serialization() {
        let value = ShellType::Other("sh".to_string());
        let json = serde_json::to_string(&value).unwrap();
        assert!(json.contains("sh"));

        let deserialized: ShellType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, value);
    }
}
