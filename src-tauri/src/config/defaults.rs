/*!
 * 配置系统默认值
 *
 * 提供所有配置项的默认值和默认配置创建函数。
 */

use crate::config::types::*;

/// 创建默认配置
pub fn create_default_config() -> AppConfig {
    AppConfig {
        version: "1.0.0".to_string(),
        metadata: None,
        app: create_default_app_config(),
        appearance: create_default_appearance_config(),
        terminal: create_default_terminal_config(),
        shortcuts: create_default_shortcuts_config(),
    }
}

/// 创建默认应用配置
fn create_default_app_config() -> AppConfigApp {
    AppConfigApp {
        language: "zh-CN".to_string(),
        confirm_on_exit: true,
        startup_behavior: "restore".to_string(),
    }
}

/// 创建默认外观配置
fn create_default_appearance_config() -> AppearanceConfig {
    AppearanceConfig {
        ui_scale: 100,
        animations_enabled: true,
        theme_config: crate::config::theme::create_default_theme_config(),
        font: create_default_font_config(),
    }
}

/// 创建默认终端配置
pub fn create_default_terminal_config() -> TerminalConfig {
    TerminalConfig {
        scrollback: 1000,
        shell: create_default_shell_config(),
        cursor: create_default_cursor_config(),
        behavior: create_default_terminal_behavior_config(),
    }
}

/// 创建默认 Shell 配置
fn create_default_shell_config() -> ShellConfig {
    ShellConfig {
        default_shell: if cfg!(windows) {
            "powershell.exe".to_string()
        } else {
            "zsh".to_string()
        },
        args: Vec::new(),
        working_directory: "~".to_string(),
    }
}

/// 创建默认终端行为配置
fn create_default_terminal_behavior_config() -> TerminalBehaviorConfig {
    TerminalBehaviorConfig {
        close_on_exit: true,
        confirm_close: false,
    }
}

/// 创建默认字体配置
fn create_default_font_config() -> FontConfig {
    FontConfig {
        family: "Menlo, Monaco, \"SF Mono\", \"Microsoft YaHei UI\", \"PingFang SC\", \"Hiragino Sans GB\", \"Source Han Sans CN\", \"WenQuanYi Micro Hei\", \"Courier New\", monospace".to_string(),
        size: 14.0,
        weight: FontWeight::Normal,
        style: FontStyle::Normal,
        line_height: 1.2,
        letter_spacing: 0.0,
    }
}

/// 创建默认光标配置
fn create_default_cursor_config() -> CursorConfig {
    CursorConfig {
        style: CursorStyle::Block,
        blink: true,
        color: "#ffffff".to_string(),
        thickness: 0.15,
    }
}

/// 创建默认快捷键配置
pub fn create_default_shortcuts_config() -> ShortcutsConfig {
    ShortcutsConfig {
        global: vec![
            ShortcutBinding {
                key: "c".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("copy_to_clipboard".to_string()),
            },
            ShortcutBinding {
                key: "v".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("paste_from_clipboard".to_string()),
            },
            ShortcutBinding {
                key: "f".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("search_forward".to_string()),
            },
        ],
        terminal: vec![
            ShortcutBinding {
                key: "t".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("new_tab".to_string()),
            },
            ShortcutBinding {
                key: "w".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("close_tab".to_string()),
            },
            ShortcutBinding {
                key: "n".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("new_window".to_string()),
            },
            ShortcutBinding {
                key: "d".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("split_vertical".to_string()),
            },
            ShortcutBinding {
                key: "d".to_string(),
                modifiers: vec!["cmd".to_string(), "shift".to_string()],
                action: ShortcutAction::Simple("split_horizontal".to_string()),
            },
            ShortcutBinding {
                key: "=".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("increase_font_size".to_string()),
            },
            ShortcutBinding {
                key: "-".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("decrease_font_size".to_string()),
            },
            ShortcutBinding {
                key: "0".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("reset_font_size".to_string()),
            },
            ShortcutBinding {
                key: "1".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("switch_to_tab_1".to_string()),
            },
            ShortcutBinding {
                key: "2".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("switch_to_tab_2".to_string()),
            },
            ShortcutBinding {
                key: "9".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("switch_to_last_tab".to_string()),
            },
        ],
        custom: vec![
            ShortcutBinding {
                key: "l".to_string(),
                modifiers: vec!["cmd".to_string(), "shift".to_string()],
                action: ShortcutAction::Complex {
                    action_type: "send_text".to_string(),
                    text: Some("ls -la\n".to_string()),
                },
            },
            ShortcutBinding {
                key: "g".to_string(),
                modifiers: vec!["cmd".to_string(), "shift".to_string()],
                action: ShortcutAction::Complex {
                    action_type: "send_text".to_string(),
                    text: Some("git status\n".to_string()),
                },
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_completeness() {
        let config = create_default_config();

        // 验证版本
        assert_eq!(config.version, "1.0.0");

        // 验证应用配置
        assert_eq!(config.app.language, "zh-CN");
        assert!(config.app.confirm_on_exit);
        assert_eq!(config.app.startup_behavior, "restore");

        // 验证外观配置
        assert_eq!(config.appearance.ui_scale, 100);
        assert!(config.appearance.animations_enabled);
        assert_eq!(
            config.appearance.font.family,
            "Menlo, Monaco, \"SF Mono\", \"Microsoft YaHei UI\", \"PingFang SC\", \"Hiragino Sans GB\", \"Source Han Sans CN\", \"WenQuanYi Micro Hei\", \"Courier New\", monospace"
        );
        assert_eq!(config.appearance.font.size, 14.0);

        // 验证主题配置
        assert_eq!(config.appearance.theme_config.terminal_theme, "dark");
        assert_eq!(config.appearance.theme_config.light_theme, "light");
        assert_eq!(config.appearance.theme_config.dark_theme, "dark");
        assert!(config.appearance.theme_config.follow_system);

        // 验证终端配置
        assert_eq!(config.terminal.scrollback, 1000);
        assert_eq!(
            config.terminal.shell.default_shell,
            if cfg!(windows) {
                "powershell.exe"
            } else {
                "zsh"
            }
        );
        assert!(config.terminal.behavior.close_on_exit);
        assert!(!config.terminal.behavior.confirm_close);

        // AI配置已迁移到SQLite，不再在TOML配置中验证

        // 验证快捷键配置
        assert!(!config.shortcuts.global.is_empty());
        assert!(!config.shortcuts.terminal.is_empty());
        assert!(!config.shortcuts.custom.is_empty());
    }

    #[test]
    fn test_default_config_serialization() {
        let config = create_default_config();

        // 测试能否序列化为TOML
        let toml_string =
            toml::to_string_pretty(&config).expect("Failed to serialize config to TOML");

        // 验证关键字段在TOML中存在（AI配置已迁移到SQLite，因此不再包含 [ai]）
        assert!(toml_string.contains("version = \"1.0.0\""));
        assert!(toml_string.contains("[app]"));
        assert!(toml_string.contains("language = \"zh-CN\""));
        assert!(toml_string.contains("[appearance]"));
        assert!(toml_string.contains("[terminal]"));
        // 移除对 [ai] 的断言
        // Note: shortcuts might be serialized differently, let's check for the content instead
        assert!(toml_string.contains("global") || toml_string.contains("shortcuts"));

        // 测试能否反序列化
        let _deserialized: AppConfig =
            toml::from_str(&toml_string).expect("Failed to deserialize TOML back to config");
    }

    #[test]
    fn test_individual_default_functions() {
        // 测试各个默认配置函数
        let app_config = create_default_app_config();
        assert_eq!(app_config.language, "zh-CN");

        let appearance_config = create_default_appearance_config();
        assert_eq!(appearance_config.ui_scale, 100);

        let terminal_config = create_default_terminal_config();
        assert_eq!(terminal_config.scrollback, 1000);

        let shortcuts_config = create_default_shortcuts_config();
        assert!(!shortcuts_config.global.is_empty());
        assert!(!shortcuts_config.terminal.is_empty());
        assert!(!shortcuts_config.custom.is_empty());
    }
}
