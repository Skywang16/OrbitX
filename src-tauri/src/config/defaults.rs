/*!
 * 配置系统默认值
 *
 * 提供所有配置项的默认值和默认配置创建函数。
 */

use crate::config::types::*;

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

fn create_default_app_config() -> AppConfigApp {
    AppConfigApp {
        language: "zh-CN".to_string(),
        confirm_on_exit: true,
        startup_behavior: "restore".to_string(),
    }
}

fn create_default_appearance_config() -> AppearanceConfig {
    AppearanceConfig {
        ui_scale: 100,
        animations_enabled: true,
        theme_config: crate::config::theme::create_default_theme_config(),
        font: create_default_font_config(),
        opacity: 1.0,
    }
}

pub fn create_default_terminal_config() -> TerminalConfig {
    TerminalConfig {
        scrollback: 1000,
        shell: create_default_shell_config(),
        cursor: create_default_cursor_config(),
        behavior: create_default_terminal_behavior_config(),
    }
}

fn create_default_shell_config() -> ShellConfig {
    ShellConfig {
        default_shell: if cfg!(windows) {
            "bash.exe".to_string()
        } else {
            "zsh".to_string()
        },
        args: Vec::new(),
        working_directory: "~".to_string(),
    }
}

fn create_default_terminal_behavior_config() -> TerminalBehaviorConfig {
    TerminalBehaviorConfig {
        close_on_exit: true,
        confirm_close: false,
    }
}

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

fn create_default_cursor_config() -> CursorConfig {
    CursorConfig {
        style: CursorStyle::Block,
        blink: true,
        color: "#ffffff".to_string(),
        thickness: 0.15,
    }
}

pub fn create_default_shortcuts_config() -> ShortcutsConfig {
    vec![
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
            action: ShortcutAction::Simple("terminal_search".to_string()),
        },
        ShortcutBinding {
            key: "s".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("open_settings".to_string()),
        },
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
            key: "3".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("switch_to_tab_3".to_string()),
        },
        ShortcutBinding {
            key: "4".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("switch_to_tab_4".to_string()),
        },
        ShortcutBinding {
            key: "5".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("switch_to_tab_5".to_string()),
        },
        ShortcutBinding {
            key: "9".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("switch_to_last_tab".to_string()),
        },
        ShortcutBinding {
            key: "right".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("accept_completion".to_string()),
        },
        ShortcutBinding {
            key: "k".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("clear_terminal".to_string()),
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
            key: "i".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("toggle_ai_sidebar".to_string()),
        },
        ShortcutBinding {
            key: "p".to_string(),
            modifiers: vec!["cmd".to_string(), "shift".to_string()],
            action: ShortcutAction::Simple("toggle_window_pin".to_string()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_completeness() {
        let config = create_default_config();

        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.app.language, "zh-CN");
        assert!(config.app.confirm_on_exit);
        assert_eq!(config.app.startup_behavior, "restore");

        assert_eq!(config.appearance.ui_scale, 100);
        assert!(config.appearance.animations_enabled);
        assert_eq!(config.appearance.opacity, 1.0);
        assert_eq!(
            config.appearance.font.family,
            "Menlo, Monaco, \"SF Mono\", \"Microsoft YaHei UI\", \"PingFang SC\", \"Hiragino Sans GB\", \"Source Han Sans CN\", \"WenQuanYi Micro Hei\", \"Courier New\", monospace"
        );
        assert_eq!(config.appearance.font.size, 14.0);

        assert_eq!(config.appearance.theme_config.terminal_theme, "dark");
        assert_eq!(config.appearance.theme_config.light_theme, "light");
        assert_eq!(config.appearance.theme_config.dark_theme, "dark");
        assert!(config.appearance.theme_config.follow_system);

        assert_eq!(config.terminal.scrollback, 1000);
        assert_eq!(
            config.terminal.shell.default_shell,
            if cfg!(windows) { "bash.exe" } else { "zsh" }
        );
        assert!(config.terminal.behavior.close_on_exit);
        assert!(!config.terminal.behavior.confirm_close);

        assert!(!config.shortcuts.is_empty());
    }

    #[test]
    fn test_default_config_serialization() {
        let config = create_default_config();

        let json_string =
            serde_json::to_string_pretty(&config).expect("Failed to serialize config to JSON");

        assert!(json_string.contains("\"version\": \"1.0.0\""));
        assert!(json_string.contains("\"app\""));
        assert!(json_string.contains("\"language\""));
        assert!(json_string.contains("\"appearance\""));
        assert!(json_string.contains("\"terminal\""));
        assert!(json_string.contains("\"shortcuts\""));

        let _deserialized: AppConfig = serde_json::from_str(&json_string)
            .expect("Failed to deserialize JSON back to config");
    }

    #[test]
    fn test_individual_default_functions() {
        let app_config = create_default_app_config();
        assert_eq!(app_config.language, "zh-CN");

        let appearance_config = create_default_appearance_config();
        assert_eq!(appearance_config.ui_scale, 100);
        assert_eq!(appearance_config.opacity, 1.0);

        let terminal_config = create_default_terminal_config();
        assert_eq!(terminal_config.scrollback, 1000);

        let shortcuts_config = create_default_shortcuts_config();
        assert!(!shortcuts_config.is_empty());
    }
}
