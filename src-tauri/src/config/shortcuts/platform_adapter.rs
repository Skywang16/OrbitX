/*!
 * 快捷键平台适配器模块
 *
 * 提供不同平台的快捷键适配功能，处理平台特定的修饰键映射。
 */

use crate::config::types::{ShortcutBinding, ShortcutsConfig};
use crate::utils::error::AppResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 支持的平台类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
}

/// 平台适配器
pub struct PlatformAdapter {
    /// 当前平台
    current_platform: Platform,
    /// 修饰键映射表
    modifier_mappings: HashMap<String, HashMap<Platform, String>>,
}

impl PlatformAdapter {
    /// 创建新的平台适配器
    pub fn new() -> Self {
        let current_platform = Self::detect_platform();
        let mut adapter = Self {
            current_platform,
            modifier_mappings: HashMap::new(),
        };

        adapter.initialize_modifier_mappings();
        adapter
    }

    /// 为指定平台适配快捷键配置
    pub fn adapt_shortcuts_for_platform(
        &self,
        config: &ShortcutsConfig,
        target_platform: Platform,
    ) -> AppResult<ShortcutsConfig> {
        Ok(ShortcutsConfig {
            global: self.adapt_shortcut_list(&config.global, target_platform)?,
            terminal: self.adapt_shortcut_list(&config.terminal, target_platform)?,
            custom: self.adapt_shortcut_list(&config.custom, target_platform)?,
        })
    }

    /// 适配快捷键列表
    fn adapt_shortcut_list(
        &self,
        shortcuts: &[ShortcutBinding],
        target_platform: Platform,
    ) -> AppResult<Vec<ShortcutBinding>> {
        shortcuts
            .iter()
            .map(|shortcut| self.adapt_single_shortcut(shortcut, target_platform))
            .collect()
    }

    /// 适配单个快捷键
    fn adapt_single_shortcut(
        &self,
        shortcut: &ShortcutBinding,
        target_platform: Platform,
    ) -> AppResult<ShortcutBinding> {
        let adapted_modifiers = shortcut
            .modifiers
            .iter()
            .map(|modifier| self.map_modifier(modifier, target_platform))
            .collect();

        Ok(ShortcutBinding {
            key: shortcut.key.clone(),
            modifiers: adapted_modifiers,
            action: shortcut.action.clone(),
        })
    }

    /// 映射修饰键到目标平台
    fn map_modifier(&self, modifier: &str, target_platform: Platform) -> String {
        if let Some(platform_mappings) = self.modifier_mappings.get(modifier) {
            if let Some(mapped_modifier) = platform_mappings.get(&target_platform) {
                return mapped_modifier.clone();
            }
        }

        // 如果没有找到映射，返回原始修饰键
        modifier.to_string()
    }

    /// 检测当前平台
    fn detect_platform() -> Platform {
        #[cfg(target_os = "windows")]
        return Platform::Windows;

        #[cfg(target_os = "macos")]
        return Platform::MacOS;

        #[cfg(target_os = "linux")]
        return Platform::Linux;

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        return Platform::Linux; // 默认为 Linux
    }

    /// 初始化修饰键映射
    fn initialize_modifier_mappings(&mut self) {
        // cmd 键的映射
        let mut cmd_mapping = HashMap::new();
        cmd_mapping.insert(Platform::MacOS, "cmd".to_string());
        cmd_mapping.insert(Platform::Windows, "ctrl".to_string());
        cmd_mapping.insert(Platform::Linux, "ctrl".to_string());
        self.modifier_mappings
            .insert("cmd".to_string(), cmd_mapping);

        // ctrl 键的映射
        let mut ctrl_mapping = HashMap::new();
        ctrl_mapping.insert(Platform::MacOS, "ctrl".to_string());
        ctrl_mapping.insert(Platform::Windows, "ctrl".to_string());
        ctrl_mapping.insert(Platform::Linux, "ctrl".to_string());
        self.modifier_mappings
            .insert("ctrl".to_string(), ctrl_mapping);

        // alt 键的映射
        let mut alt_mapping = HashMap::new();
        alt_mapping.insert(Platform::MacOS, "alt".to_string());
        alt_mapping.insert(Platform::Windows, "alt".to_string());
        alt_mapping.insert(Platform::Linux, "alt".to_string());
        self.modifier_mappings
            .insert("alt".to_string(), alt_mapping);

        // shift 键的映射
        let mut shift_mapping = HashMap::new();
        shift_mapping.insert(Platform::MacOS, "shift".to_string());
        shift_mapping.insert(Platform::Windows, "shift".to_string());
        shift_mapping.insert(Platform::Linux, "shift".to_string());
        self.modifier_mappings
            .insert("shift".to_string(), shift_mapping);

        // meta/super/win 键的映射
        let mut meta_mapping = HashMap::new();
        meta_mapping.insert(Platform::MacOS, "cmd".to_string());
        meta_mapping.insert(Platform::Windows, "win".to_string());
        meta_mapping.insert(Platform::Linux, "super".to_string());
        self.modifier_mappings
            .insert("meta".to_string(), meta_mapping.clone());
        self.modifier_mappings
            .insert("super".to_string(), meta_mapping.clone());
        self.modifier_mappings
            .insert("win".to_string(), meta_mapping);
    }

    /// 获取当前平台
    pub fn current_platform(&self) -> Platform {
        self.current_platform
    }

    /// 检查快捷键是否适用于当前平台
    pub fn is_shortcut_valid_for_platform(
        &self,
        shortcut: &ShortcutBinding,
        platform: Platform,
    ) -> bool {
        // 检查是否有平台特定的限制
        match platform {
            Platform::MacOS => {
                // macOS 特定检查
                !shortcut.modifiers.contains(&"win".to_string())
            }
            Platform::Windows => {
                // Windows 特定检查
                !shortcut.modifiers.contains(&"cmd".to_string())
            }
            Platform::Linux => {
                // Linux 特定检查
                !shortcut.modifiers.contains(&"cmd".to_string())
            }
        }
    }
}

impl Default for PlatformAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::{ShortcutAction, ShortcutBinding, ShortcutsConfig};

    #[test]
    fn test_platform_detection() {
        let adapter = PlatformAdapter::new();
        // 平台检测应该返回一个有效的平台
        match adapter.current_platform() {
            Platform::Windows | Platform::MacOS | Platform::Linux => {
                // 测试通过
            }
        }
    }

    #[test]
    fn test_cmd_key_adaptation() {
        let adapter = PlatformAdapter::new();
        let shortcut = ShortcutBinding {
            key: "c".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("copy".to_string()),
        };

        // 适配到 Windows
        let adapted = adapter
            .adapt_single_shortcut(&shortcut, Platform::Windows)
            .unwrap();
        assert_eq!(adapted.modifiers, vec!["ctrl"]);

        // 适配到 macOS
        let adapted = adapter
            .adapt_single_shortcut(&shortcut, Platform::MacOS)
            .unwrap();
        assert_eq!(adapted.modifiers, vec!["cmd"]);

        // 适配到 Linux
        let adapted = adapter
            .adapt_single_shortcut(&shortcut, Platform::Linux)
            .unwrap();
        assert_eq!(adapted.modifiers, vec!["ctrl"]);
    }

    #[test]
    fn test_shortcut_validity_for_platform() {
        let adapter = PlatformAdapter::new();

        let cmd_shortcut = ShortcutBinding {
            key: "c".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("copy".to_string()),
        };

        let win_shortcut = ShortcutBinding {
            key: "c".to_string(),
            modifiers: vec!["win".to_string()],
            action: ShortcutAction::Simple("copy".to_string()),
        };

        // cmd 快捷键在 macOS 上有效，在其他平台上无效
        assert!(adapter.is_shortcut_valid_for_platform(&cmd_shortcut, Platform::MacOS));
        assert!(!adapter.is_shortcut_valid_for_platform(&cmd_shortcut, Platform::Windows));
        assert!(!adapter.is_shortcut_valid_for_platform(&cmd_shortcut, Platform::Linux));

        // win 快捷键在 macOS 上无效
        assert!(!adapter.is_shortcut_valid_for_platform(&win_shortcut, Platform::MacOS));
    }

    #[test]
    fn test_shortcuts_config_adaptation() {
        let adapter = PlatformAdapter::new();
        let config = ShortcutsConfig {
            global: vec![ShortcutBinding {
                key: "c".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("copy".to_string()),
            }],
            terminal: vec![ShortcutBinding {
                key: "v".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("paste".to_string()),
            }],
            custom: vec![],
        };

        let adapted = adapter
            .adapt_shortcuts_for_platform(&config, Platform::Windows)
            .unwrap();

        // 所有 cmd 键应该被映射为 ctrl
        assert_eq!(adapted.global[0].modifiers, vec!["ctrl"]);
        assert_eq!(adapted.terminal[0].modifiers, vec!["ctrl"]);
    }
}
