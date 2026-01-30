//! Zsh集成脚本生成器

use super::ShellIntegrationConfig;

/// Node.js 版本检测脚本（与 bash.rs 中相同）
const NODE_VERSION_DETECTION: &str = r#"
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

/// 生成 Zsh 集成脚本
pub fn generate_script(config: &ShellIntegrationConfig) -> String {
    let mut script = String::new();

    script.push_str(
        r#"
# OrbitX Shell Integration for Zsh
if [[ -n "$ORBITX_SHELL_INTEGRATION_LOADED" ]]; then
    return 0
fi
export ORBITX_SHELL_INTEGRATION_LOADED=1
"#,
    );

    // CWD同步功能
    if config.enable_cwd_sync {
        script.push_str(
            r#"
# CWD同步函数
__orbitx_update_cwd() {
    printf '\e]7;file://%s%s\e\\' "$HOST" "$PWD"
}
"#,
        );
    }

    // 添加 Node 版本检测函数
    script.push_str(NODE_VERSION_DETECTION);

    // 命令跟踪功能
    if config.enable_command_tracking {
        script.push_str(
            r#"
# Shell Integration支持 - OSC 133序列
__orbitx_preexec() {
    # C: 命令执行开始，携带命令内容
    # $1 是 zsh preexec 传入的完整命令行
    printf '\e]133;C;%s\e\\' "$1"
}

__orbitx_precmd() {
    local exit_code=$?
    # D: 命令完成，包含退出码
    printf '\e]133;D;%d\e\\' "$exit_code"
    __orbitx_update_cwd 2>/dev/null || true
    # A: 提示符开始
    printf '\e]133;A\e\\'
    # B: 命令输入区开始
    printf '\e]133;B\e\\'
    __orbitx_detect_node_version
}

# 保持原始PS1不变，不直接嵌入OSC序列
if [[ -z "$ORBITX_ORIGINAL_PS1" ]]; then
    export ORBITX_ORIGINAL_PS1="$PS1"
fi

# 添加钩子函数
if [[ -z "${precmd_functions[(r)__orbitx_precmd]}" ]]; then
    precmd_functions+=(__orbitx_precmd)
fi

if [[ -z "${preexec_functions[(r)__orbitx_preexec]}" ]]; then
    preexec_functions+=(__orbitx_preexec)
fi
"#,
        );
    } else {
        // 没有命令跟踪，但仍然需要检测 Node 版本
        script.push_str(
            r#"
# Node 版本检测钩子
__orbitx_node_version_precmd() {
    __orbitx_detect_node_version
}

if [[ -z "${precmd_functions[(r)__orbitx_node_version_precmd]}" ]]; then
    precmd_functions+=(__orbitx_node_version_precmd)
fi
"#,
        );

        if config.enable_cwd_sync {
            script.push_str(
                r#"
# CWD同步
if [[ -z "${precmd_functions[(r)__orbitx_update_cwd]}" ]]; then
    precmd_functions+=(__orbitx_update_cwd)
fi
"#,
            );
        }
    }

    // 窗口标题更新
    if config.enable_title_updates {
        script.push_str(
            r#"
# 窗口标题更新
__orbitx_update_title() {
    printf '\e]2;%s@%s:%s\e\\' "$USER" "$HOST" "${PWD/#$HOME/~}"
}

if [[ -z "${precmd_functions[(r)__orbitx_update_title]}" ]]; then
    precmd_functions+=(__orbitx_update_title)
fi
"#,
        );
    }

    // 添加自定义环境变量
    if !config.custom_env_vars.is_empty() {
        script.push_str("\n# 自定义环境变量\n");
        for (key, value) in &config.custom_env_vars {
            script.push_str(&format!("export {key}=\"{value}\"\n"));
        }
    }

    // 加载用户原始配置
    script.push_str(
        r#"
# 初始化CWD和标题
__orbitx_update_cwd 2>/dev/null || true
[[ "$(type -w __orbitx_update_title 2>/dev/null)" == *"function"* ]] && __orbitx_update_title 2>/dev/null || true

# 启动后检测 Node 版本（后台静默运行）
{
    for i in 1 2 3 4 5; do
        __orbitx_detect_node_version 2>/dev/null
        if [[ -n "$__orbitx_last_node_version" ]]; then
            break
        fi
        sleep 0.2
    done
} &!
"#,
    );

    script
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_basic_zsh_script_generation() {
        let config = ShellIntegrationConfig::default();
        let script = generate_script(&config);

        assert!(script.contains("# OrbitX Shell Integration for Zsh"));
        assert!(script.contains("ORBITX_SHELL_INTEGRATION_LOADED"));
        assert!(script.contains("precmd_functions"));
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
        assert!(script.contains("preexec_functions"));
        assert!(script.contains("precmd_functions"));
    }

    #[test]
    fn test_cwd_sync_enabled() {
        let config = ShellIntegrationConfig {
            enable_cwd_sync: true,
            ..Default::default()
        };
        let script = generate_script(&config);

        assert!(script.contains("__orbitx_update_cwd"));
        assert!(script.contains("precmd_functions"));
    }

    #[test]
    fn test_title_updates_enabled() {
        let config = ShellIntegrationConfig {
            enable_title_updates: true,
            ..Default::default()
        };
        let script = generate_script(&config);

        assert!(script.contains("__orbitx_update_title"));
        assert!(script.contains("precmd_functions"));
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

        assert!(script.contains("export ORBITX_CUSTOM=\"test_value\""));
    }
}
