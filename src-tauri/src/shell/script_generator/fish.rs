//! Fish Shell Integration Script Generator
//!
//! 为 Fish shell 生成集成脚本，包括命令跟踪、CWD同步等功能

use super::ShellIntegrationConfig;

/// 生成 Fish 集成脚本
pub fn generate_script(config: &ShellIntegrationConfig) -> String {
    let mut script = String::new();

    // 检查是否已经注入过
    script.push_str(
        r#"
# OrbitX Shell Integration for Fish
if set -q ORBITX_SHELL_INTEGRATION_LOADED
    exit 0
end
set -g ORBITX_SHELL_INTEGRATION_LOADED 1
"#,
    );

    // CWD同步功能
    if config.enable_cwd_sync {
        script.push_str(
            r#"
# CWD同步函数
function __orbitx_update_cwd --on-variable PWD
    printf '\e]7;file://%s%s\e\\' (hostname) (pwd)
end
"#,
        );
    }

    // 命令跟踪功能
    if config.enable_command_tracking {
        script.push_str(
            r#"
# Shell Integration支持 (OSC 133)
function __orbitx_preexec --on-event fish_preexec
    printf '\e]133;C\e\\'
end

function __orbitx_postcmd --on-event fish_postexec
    printf '\e]133;D;%d\e\\' $status
    __orbitx_update_cwd
    printf '\e]133;A\e\\'
end

# 函数在提示符显示时执行
function __orbitx_prompt_start --on-event fish_prompt
    printf '\e]133;A\e\\'
end

# 用户开始输入命令时
function __orbitx_prompt_end --on-event fish_preexec
    printf '\e]133;B\e\\'
end
"#,
        );
    } else if config.enable_cwd_sync {
        // Fish的PWD变化监控已经在CWD同步函数中处理
        script.push_str("# CWD同步已在上面的__orbitx_update_cwd函数中启用\n");
    }

    // 窗口标题更新
    if config.enable_title_updates {
        script.push_str(
            r#"
# 窗口标题更新
function __orbitx_update_title --on-variable PWD
    set -l title "$USER@"(hostname)":"(string replace -r "^$HOME" "~" (pwd))
    printf '\e]2;%s\e\\' "$title"
end
"#,
        );
    }

    // 添加自定义环境变量
    if !config.custom_env_vars.is_empty() {
        script.push_str("\n# 自定义环境变量\n");
        for (key, value) in &config.custom_env_vars {
            script.push_str(&format!("set -gx {} \"{}\"\n", key, value));
        }
    }

    // 初始化
    script.push_str(
        r#"
# 初始化CWD和标题
__orbitx_update_cwd 2>/dev/null; or true
if functions -q __orbitx_update_title
    __orbitx_update_title 2>/dev/null; or true
end
"#,
    );

    script
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_basic_fish_script_generation() {
        let config = ShellIntegrationConfig::default();
        let script = generate_script(&config);
        
        assert!(script.contains("# OrbitX Shell Integration for Fish"));
        assert!(script.contains("ORBITX_SHELL_INTEGRATION_LOADED"));
        assert!(script.contains("set -g"));
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
        assert!(script.contains("fish_preexec"));
        assert!(script.contains("fish_postexec"));
    }

    #[test]
    fn test_cwd_sync_enabled() {
        let config = ShellIntegrationConfig {
            enable_cwd_sync: true,
            ..Default::default()
        };
        let script = generate_script(&config);
        
        assert!(script.contains("__orbitx_update_cwd"));
        assert!(script.contains("--on-variable PWD"));
    }

    #[test]
    fn test_title_updates_enabled() {
        let config = ShellIntegrationConfig {
            enable_title_updates: true,
            ..Default::default()
        };
        let script = generate_script(&config);
        
        assert!(script.contains("__orbitx_update_title"));
        assert!(script.contains("--on-variable PWD"));
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
        
        assert!(script.contains("set -gx ORBITX_CUSTOM \"test_value\""));
    }
}
