//! Bash集成脚本生成器

use super::ShellIntegrationConfig;

/// Node.js 版本检测脚本（兼容 Bash 和 Zsh）
const NODE_VERSION_DETECTION: &str = r#"
    # Node.js 版本检测
    __orbitx_last_node_version=""

    __orbitx_detect_node_version() {
        if command -v node >/dev/null 2>&1; then
            local current_version=$(node -v 2>/dev/null | tr -d '\n')
            if [[ -n "$current_version" && "$current_version" != "$__orbitx_last_node_version" ]]; then
                __orbitx_last_node_version="$current_version"
                printf '\e]1337;OrbitXNodeVersion=%s\e\\' "$current_version"
            fi
        elif [[ -n "$__orbitx_last_node_version" ]]; then
            __orbitx_last_node_version=""
            printf '\e]1337;OrbitXNodeVersion=\e\\'
        fi
    }
"#;

/// 生成 Bash 集成脚本
pub fn generate_script(config: &ShellIntegrationConfig) -> String {
    let mut script = String::new();

    script.push_str(
        r#"
# OrbitX Integration Start
if [[ -z "$ORBITX_SHELL_INTEGRATION" ]]; then
    export ORBITX_SHELL_INTEGRATION=1
    export ORBITX_INTEGRATION_LOADED=1

    # 原始 PS1 备份
    if [[ -z "$ORBITX_ORIGINAL_PS1" ]]; then
        export ORBITX_ORIGINAL_PS1="$PS1"
    fi
"#,
    );

    // 添加 Node 版本检测函数
    script.push_str(NODE_VERSION_DETECTION);

    // 只有启用命令跟踪时才添加相关函数（使用标准 OSC 133 标记）
    if config.enable_command_tracking {
        script.push_str(
            r#"
    # Shell Integration 支持 - OSC 133 标记
    __orbitx_preexec() {
        # C: 命令执行开始（提示符结束）
        printf '\e]133;C\e\\' >/dev/tty
    }

    __orbitx_precmd() {
        local exit_code=$?
        # D: 命令完成，包含退出码
        printf '\e]133;D;%d\e\\' "$exit_code" >/dev/tty
        # A: 提示符开始
        printf '\e]133;A\e\\' >/dev/tty
        # B: 命令开始（提示符结束，准备接收用户输入）
        printf '\e]133;B\e\\' >/dev/tty
        # 在 A/B 之后再上报 Node 版本，避免 UI 在 A 时清空
        __orbitx_detect_node_version
    }
"#,
        );
    }

    // 添加命令跟踪功能：通过 DEBUG trap 和 PROMPT_COMMAND（Bash 通用做法）
    if config.enable_command_tracking {
        script.push_str(
            r#"
    if [[ -z "$ORBITX_PREEXEC_INSTALLED" ]]; then
        export ORBITX_PREEXEC_INSTALLED=1

        # 使用 DEBUG trap 模拟 preexec
        trap '__orbitx_preexec "$BASH_COMMAND"' DEBUG

        # 在提示符渲染前运行 precmd
        if [[ -z "$PROMPT_COMMAND" ]]; then
            PROMPT_COMMAND="__orbitx_precmd"
        else
            PROMPT_COMMAND="$PROMPT_COMMAND; __orbitx_precmd"
        fi
    fi
"#,
        );
    }

    //（已移除 CWD 同步别名，依赖标准 OSC 7 由其他逻辑处理，如需可单独实现）

    //（已移除窗口标题更新以保持最小实现，如需可使用标准 OSC 2 单独实现）

    // 添加自定义环境变量
    if !config.custom_env_vars.is_empty() {
        script.push_str("\n    # 自定义环境变量\n");
        for (key, value) in &config.custom_env_vars {
            script.push_str(&format!("    export {}=\"{}\"\n", key, value));
        }
    }

    // 如果没有启用命令跟踪，需要单独设置 Node 版本检测
    if !config.enable_command_tracking {
        script.push_str(
            r#"
    # Node 版本检测（无命令跟踪时）
    if [[ -z "$PROMPT_COMMAND" ]]; then
        PROMPT_COMMAND="__orbitx_detect_node_version"
    else
        PROMPT_COMMAND="$PROMPT_COMMAND; __orbitx_detect_node_version"
    fi
"#,
        );
    }

    script.push_str(
        r#"
    # 初始化时立即检测 Node 版本
    __orbitx_detect_node_version 2>/dev/null || true

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
        assert!(script.contains("__orbitx_detect_node_version"));
    }

    #[test]
    fn test_command_tracking_enabled() {
        let config = ShellIntegrationConfig {
            enable_command_tracking: true,
            ..Default::default()
        };
        let script = generate_script(&config);

        assert!(script.contains("__orbitx_preexec"));
        assert!(script.contains("__orbitx_precmd"));
        assert!(script.contains("PROMPT_COMMAND"));
        assert!(script.contains("trap '__orbitx_preexec"));
    }

    #[test]
    fn test_cwd_sync_enabled() {
        let config = ShellIntegrationConfig {
            enable_cwd_sync: true,
            ..Default::default()
        };
        let script = generate_script(&config);
        // 简化后不通过别名同步 CWD，仍应包含节点版本检测逻辑
        assert!(script.contains("__orbitx_detect_node_version"));
    }

    #[test]
    fn test_title_updates_enabled() {
        let config = ShellIntegrationConfig {
            enable_title_updates: true,
            ..Default::default()
        };
        let script = generate_script(&config);
        // 简化后不更新窗口标题，仍应包含节点版本检测逻辑
        assert!(script.contains("__orbitx_detect_node_version"));
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
