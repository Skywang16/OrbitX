/*!
 * 快捷键冲突检测器模块
 *
 * 提供快捷键冲突检测功能，检测不同类别间的快捷键冲突。
 */

use crate::config::types::{ShortcutBinding, ShortcutsConfig};
use crate::utils::error::AppResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 快捷键冲突检测器
pub struct ShortcutConflictDetector;

/// 快捷键冲突检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDetectionResult {
    /// 是否有冲突
    pub has_conflicts: bool,
    /// 冲突列表
    pub conflicts: Vec<ShortcutConflict>,
}

/// 快捷键冲突
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConflict {
    /// 冲突的快捷键组合
    pub key_combination: String,
    /// 冲突的快捷键绑定列表
    pub conflicting_shortcuts: Vec<ConflictingShortcut>,
}

/// 冲突的快捷键信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictingShortcut {
    /// 快捷键类别
    pub category: String,
    /// 快捷键绑定
    pub binding: ShortcutBinding,
}

impl ShortcutConflictDetector {
    /// 创建新的冲突检测器
    pub fn new() -> Self {
        Self
    }

    /// 检测快捷键配置中的冲突
    pub fn detect_conflicts(&self, config: &ShortcutsConfig) -> AppResult<ConflictDetectionResult> {
        let mut key_map: HashMap<String, Vec<ConflictingShortcut>> = HashMap::new();

        // 收集所有快捷键
        self.collect_shortcuts(&config.global, "global", &mut key_map);
        self.collect_shortcuts(&config.terminal, "terminal", &mut key_map);
        self.collect_shortcuts(&config.custom, "custom", &mut key_map);

        // 检测冲突
        let mut conflicts = Vec::new();
        for (key_combination, shortcuts) in key_map {
            if shortcuts.len() > 1 {
                conflicts.push(ShortcutConflict {
                    key_combination,
                    conflicting_shortcuts: shortcuts,
                });
            }
        }

        Ok(ConflictDetectionResult {
            has_conflicts: !conflicts.is_empty(),
            conflicts,
        })
    }

    /// 收集快捷键到映射表中
    fn collect_shortcuts(
        &self,
        shortcuts: &[ShortcutBinding],
        category: &str,
        key_map: &mut HashMap<String, Vec<ConflictingShortcut>>,
    ) {
        for shortcut in shortcuts {
            let key_combination = self.format_key_combination(&shortcut.key, &shortcut.modifiers);
            let conflicting_shortcut = ConflictingShortcut {
                category: category.to_string(),
                binding: shortcut.clone(),
            };

            key_map
                .entry(key_combination)
                .or_insert_with(Vec::new)
                .push(conflicting_shortcut);
        }
    }

    /// 格式化快捷键组合为字符串
    fn format_key_combination(&self, key: &str, modifiers: &[String]) -> String {
        let mut parts = modifiers.to_vec();
        parts.sort(); // 确保修饰键顺序一致
        parts.push(key.to_string());
        parts.join("+")
    }
}

impl Default for ShortcutConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::{ShortcutAction, ShortcutBinding, ShortcutsConfig};

    #[test]
    fn test_no_conflicts() {
        let detector = ShortcutConflictDetector::new();
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

        let result = detector.detect_conflicts(&config).unwrap();
        assert!(!result.has_conflicts);
        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn test_with_conflicts() {
        let detector = ShortcutConflictDetector::new();
        let config = ShortcutsConfig {
            global: vec![ShortcutBinding {
                key: "c".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("copy".to_string()),
            }],
            terminal: vec![ShortcutBinding {
                key: "c".to_string(),
                modifiers: vec!["cmd".to_string()],
                action: ShortcutAction::Simple("close_tab".to_string()),
            }],
            custom: vec![],
        };

        let result = detector.detect_conflicts(&config).unwrap();
        assert!(result.has_conflicts);
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].conflicting_shortcuts.len(), 2);
    }

    #[test]
    fn test_key_combination_formatting() {
        let detector = ShortcutConflictDetector::new();

        // 测试修饰键排序
        let combo1 =
            detector.format_key_combination("c", &["shift".to_string(), "cmd".to_string()]);
        let combo2 =
            detector.format_key_combination("c", &["cmd".to_string(), "shift".to_string()]);

        assert_eq!(combo1, combo2); // 应该相等，因为修饰键会被排序
        assert_eq!(combo1, "cmd+shift+c");
    }
}
