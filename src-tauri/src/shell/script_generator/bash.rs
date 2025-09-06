//! Bash Shell Integration Script Generator
//!
//! 为 Bash shell 生成集成脚本，包括命令跟踪、CWD同步等功能

use super::ShellIntegrationConfig;

/// 生成 Bash 集成脚本
pub fn generate_script(config: &ShellIntegrationConfig) -> String {
    let mut script = String::new();

    // 检查是否已经注入过
    script.push_str(
        r#"
# OrbitX Integration Start
if [[ -z "$ORBITX_INTEGRATION_LOADED" ]]; then
    export ORBITX_INTEGRATION_LOADED=1

    # 原始 PS1 备份
    if [[ -z "$ORBITX_ORIGINAL_PS1" ]]; then
        export ORBITX_ORIGINAL_PS1="$PS1"
    fi

    # OSC 序列函数
    orbitx_osc() {
        printf "\e]6969;%s\a" "$1" >/dev/tty
    }

    # 命令开始标记
    orbitx_preexec() {
        orbitx_osc "BeforeCommand;$(pwd);$1"
    }

    # 命令结束标记和CWD同步
    orbitx_precmd() {
        local exit_code=$?
        orbitx_osc "AfterCommand;$(pwd);$exit_code"
    }
"#,
    );

    // 添加命令跟踪功能
    if config.enable_command_tracking {
        script.push_str(
            r#"
    # 设置 trap 来捕获命令执行
    if [[ -z "$ORBITX_PREEXEC_INSTALLED" ]]; then
        export ORBITX_PREEXEC_INSTALLED=1
        
        # 安装 preexec 和 precmd 钩子
        preexec() {
            orbitx_preexec "$1"
        }
        
        precmd() {
            orbitx_precmd
        }
        
        # 如果没有 preexec/precmd 支持，使用替代方案
        if ! command -v preexec >/dev/null 2>&1; then
            # 使用 DEBUG trap 作为替代
            trap 'orbitx_preexec "$BASH_COMMAND"' DEBUG
        fi
        
        # 设置 PROMPT_COMMAND
        if [[ -z "$PROMPT_COMMAND" ]]; then
            PROMPT_COMMAND="orbitx_precmd"
        else
            PROMPT_COMMAND="$PROMPT_COMMAND; orbitx_precmd"
        fi
    fi
"#,
        );
    }

    // 添加 CWD 同步功能
    if config.enable_cwd_sync {
        script.push_str(
            r#"
    # CWD 变化监控
    orbitx_cd() {
        builtin cd "$@"
        local result=$?
        orbitx_osc "DirectoryChanged;$(pwd)"
        return $result
    }
    
    # 覆盖 cd 命令
    alias cd='orbitx_cd'
"#,
        );
    }

    // 添加窗口标题更新功能
    if config.enable_title_updates {
        script.push_str(
            r#"
    # 窗口标题更新
    orbitx_update_title() {
        local title="$1"
        if [[ -n "$title" ]]; then
            printf "\e]0;%s\a" "$title" >/dev/tty
        fi
    }
    
    # 在 precmd 中更新标题
    orbitx_precmd_with_title() {
        orbitx_precmd
        orbitx_update_title "$(basename "$(pwd)") - Bash"
    }
    
    # 更新 PROMPT_COMMAND
    if [[ "$PROMPT_COMMAND" == *"orbitx_precmd"* ]]; then
        PROMPT_COMMAND="${PROMPT_COMMAND/orbitx_precmd/orbitx_precmd_with_title}"
    fi
"#,
        );
    }

    // 添加自定义环境变量
    if !config.custom_env_vars.is_empty() {
        script.push_str("\n    # 自定义环境变量\n");
        for (key, value) in &config.custom_env_vars {
            script.push_str(&format!("    export {}=\"{}\"\n", key, value));
        }
    }

    // 结束集成代码块
    script.push_str(
        r#"
    # 初始化完成通知
    orbitx_osc "ShellIntegrationReady;bash;$(pwd)"

fi
# OrbitX Integration End
"#,
    );

    script
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_basic_bash_script_generation() {
        let config = ShellIntegrationConfig::default();
        let script = generate_script(&config);
        
        assert!(script.contains("# OrbitX Integration Start"));
        assert!(script.contains("# OrbitX Integration End"));
        assert!(script.contains("ORBITX_INTEGRATION_LOADED"));
        assert!(script.contains("orbitx_osc"));
    }

    #[test]
    fn test_command_tracking_enabled() {
        let config = ShellIntegrationConfig {
            enable_command_tracking: true,
            ..Default::default()
        };
        let script = generate_script(&config);
        
        assert!(script.contains("orbitx_preexec"));
        assert!(script.contains("PROMPT_COMMAND"));
        assert!(script.contains("DEBUG trap"));
    }

    #[test]
    fn test_cwd_sync_enabled() {
        let config = ShellIntegrationConfig {
            enable_cwd_sync: true,
            ..Default::default()
        };
        let script = generate_script(&config);
        
        assert!(script.contains("orbitx_cd"));
        assert!(script.contains("alias cd="));
        assert!(script.contains("DirectoryChanged"));
    }

    #[test]
    fn test_title_updates_enabled() {
        let config = ShellIntegrationConfig {
            enable_title_updates: true,
            ..Default::default()
        };
        let script = generate_script(&config);
        
        assert!(script.contains("orbitx_update_title"));
        assert!(script.contains("orbitx_precmd_with_title"));
    }

    #[test]
    fn test_custom_env_vars() {
        let mut custom_vars = HashMap::new();
        custom_vars.insert("ORBITX_CUSTOM".to_string(), "test_value".to_string());
        custom_vars.insert("ANOTHER_VAR".to_string(), "another_value".to_string());
        
        let config = ShellIntegrationConfig {
            custom_env_vars: custom_vars,
            ..Default::default()
        };
        let script = generate_script(&config);
        
        assert!(script.contains("export ORBITX_CUSTOM=\"test_value\""));
        assert!(script.contains("export ANOTHER_VAR=\"another_value\""));
    }

    #[test]
    fn test_all_features_disabled() {
        let config = ShellIntegrationConfig {
            enable_command_tracking: false,
            enable_cwd_sync: false,
            enable_title_updates: false,
            custom_env_vars: HashMap::new(),
        };
        let script = generate_script(&config);
        
        // 仍应包含基本结构
        assert!(script.contains("# OrbitX Integration Start"));
        assert!(script.contains("# OrbitX Integration End"));
        assert!(script.contains("ORBITX_INTEGRATION_LOADED"));
        
        // 不应包含禁用的功能
        assert!(!script.contains("orbitx_preexec"));
        assert!(!script.contains("orbitx_cd"));
        assert!(!script.contains("orbitx_update_title"));
    }
}
