/*!
 * 快捷键验证器模块
 *
 * 提供快捷键配置的验证功能，包括按键格式验证、修饰键组合验证、
 * 动作类型验证等功能。
 */

use crate::config::types::{ShortcutAction, ShortcutBinding, ShortcutsConfig};
use crate::utils::error::AppResult;
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 快捷键验证器
pub struct ShortcutValidator {
    /// 支持的按键列表
    valid_keys: HashSet<String>,
    /// 支持的修饰键列表
    valid_modifiers: HashSet<String>,
    /// 支持的动作类型列表
    valid_action_types: HashSet<String>,
}

/// 快捷键验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutValidationResult {
    /// 是否通过验证
    pub is_valid: bool,
    /// 验证错误列表
    pub errors: Vec<ShortcutValidationError>,
    /// 验证警告列表
    pub warnings: Vec<ShortcutValidationWarning>,
}

/// 快捷键验证错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutValidationError {
    /// 错误类型
    pub error_type: String,
    /// 错误消息
    pub message: String,
    /// 相关的快捷键绑定（可选）
    pub shortcut: Option<ShortcutBinding>,
}

/// 快捷键验证警告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutValidationWarning {
    /// 警告类型
    pub warning_type: String,
    /// 警告消息
    pub message: String,
    /// 相关的快捷键绑定（可选）
    pub shortcut: Option<ShortcutBinding>,
}

impl ShortcutValidator {
    /// 创建新的快捷键验证器
    pub fn new() -> Self {
        let mut validator = Self {
            valid_keys: HashSet::new(),
            valid_modifiers: HashSet::new(),
            valid_action_types: HashSet::new(),
        };

        validator.initialize_valid_keys();
        validator.initialize_valid_modifiers();
        validator.initialize_valid_action_types();
        validator
    }

    /// 验证完整的快捷键配置
    pub fn validate_shortcuts_config(
        &self,
        config: &ShortcutsConfig,
    ) -> AppResult<ShortcutValidationResult> {
        let mut result = ShortcutValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // 验证全局快捷键
        for shortcut in &config.global {
            self.validate_single_shortcut(shortcut, "global", &mut result)?;
        }

        // 验证终端快捷键
        for shortcut in &config.terminal {
            self.validate_single_shortcut(shortcut, "terminal", &mut result)?;
        }

        // 验证自定义快捷键
        for shortcut in &config.custom {
            self.validate_single_shortcut(shortcut, "custom", &mut result)?;
        }

        // 如果有错误，标记为无效
        if !result.errors.is_empty() {
            result.is_valid = false;
        }

        Ok(result)
    }

    /// 验证单个快捷键绑定
    pub fn validate_shortcut_binding(
        &self,
        binding: &ShortcutBinding,
    ) -> AppResult<ShortcutValidationResult> {
        let mut result = ShortcutValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        self.validate_single_shortcut(binding, "single", &mut result)?;

        if !result.errors.is_empty() {
            result.is_valid = false;
        }

        Ok(result)
    }

    /// 验证单个快捷键
    fn validate_single_shortcut(
        &self,
        shortcut: &ShortcutBinding,
        category: &str,
        result: &mut ShortcutValidationResult,
    ) -> AppResult<()> {
        // 验证按键
        self.validate_key(&shortcut.key, shortcut, category, result)?;

        // 验证修饰键
        self.validate_modifiers(&shortcut.modifiers, shortcut, category, result)?;

        // 验证动作
        self.validate_action(&shortcut.action, shortcut, category, result)?;

        // 验证修饰键组合的合理性
        self.validate_modifier_combination(&shortcut.modifiers, shortcut, category, result)?;

        Ok(())
    }

    /// 验证按键
    fn validate_key(
        &self,
        key: &str,
        shortcut: &ShortcutBinding,
        category: &str,
        result: &mut ShortcutValidationResult,
    ) -> AppResult<()> {
        if key.is_empty() {
            result.errors.push(ShortcutValidationError {
                error_type: "empty_key".to_string(),
                message: format!("{}类别中的快捷键按键不能为空", category),
                shortcut: Some(shortcut.clone()),
            });
            return Ok(());
        }

        if !self.valid_keys.contains(key) {
            result.errors.push(ShortcutValidationError {
                error_type: "invalid_key".to_string(),
                message: format!("{}类别中的按键 '{}' 不是有效的按键", category, key),
                shortcut: Some(shortcut.clone()),
            });
        }

        Ok(())
    }

    /// 验证修饰键
    fn validate_modifiers(
        &self,
        modifiers: &[String],
        shortcut: &ShortcutBinding,
        category: &str,
        result: &mut ShortcutValidationResult,
    ) -> AppResult<()> {
        if modifiers.is_empty() {
            result.warnings.push(ShortcutValidationWarning {
                warning_type: "no_modifiers".to_string(),
                message: format!("{}类别中的快捷键没有修饰键，可能与系统快捷键冲突", category),
                shortcut: Some(shortcut.clone()),
            });
        }

        for modifier in modifiers {
            if !self.valid_modifiers.contains(modifier) {
                result.errors.push(ShortcutValidationError {
                    error_type: "invalid_modifier".to_string(),
                    message: format!("{}类别中的修饰键 '{}' 不是有效的修饰键", category, modifier),
                    shortcut: Some(shortcut.clone()),
                });
            }
        }

        // 检查重复的修饰键
        let mut unique_modifiers = HashSet::new();
        for modifier in modifiers {
            if !unique_modifiers.insert(modifier) {
                result.errors.push(ShortcutValidationError {
                    error_type: "duplicate_modifier".to_string(),
                    message: format!("{}类别中的修饰键 '{}' 重复", category, modifier),
                    shortcut: Some(shortcut.clone()),
                });
            }
        }

        Ok(())
    }

    /// 验证动作
    fn validate_action(
        &self,
        action: &ShortcutAction,
        shortcut: &ShortcutBinding,
        category: &str,
        result: &mut ShortcutValidationResult,
    ) -> AppResult<()> {
        match action {
            ShortcutAction::Simple(action_str) => {
                if action_str.is_empty() {
                    result.errors.push(ShortcutValidationError {
                        error_type: "empty_action".to_string(),
                        message: format!("{}类别中的快捷键动作不能为空", category),
                        shortcut: Some(shortcut.clone()),
                    });
                }
            }
            ShortcutAction::Complex { action_type, text } => {
                if !self.valid_action_types.contains(action_type) {
                    result.errors.push(ShortcutValidationError {
                        error_type: "invalid_action_type".to_string(),
                        message: format!(
                            "{}类别中的动作类型 '{}' 不是有效的动作类型",
                            category, action_type
                        ),
                        shortcut: Some(shortcut.clone()),
                    });
                }

                // 对于需要文本的动作类型，验证文本是否存在
                if action_type == "send_text"
                    && (text.is_none() || text.as_ref().unwrap().is_empty())
                {
                    result.errors.push(ShortcutValidationError {
                        error_type: "missing_text".to_string(),
                        message: format!("{}类别中的 'send_text' 动作需要提供文本内容", category),
                        shortcut: Some(shortcut.clone()),
                    });
                }
            }
        }

        Ok(())
    }

    /// 验证修饰键组合的合理性
    fn validate_modifier_combination(
        &self,
        modifiers: &[String],
        shortcut: &ShortcutBinding,
        category: &str,
        result: &mut ShortcutValidationResult,
    ) -> AppResult<()> {
        // 检查是否同时包含 cmd 和 ctrl（通常不合理）
        let has_cmd = modifiers.contains(&"cmd".to_string());
        let has_ctrl = modifiers.contains(&"ctrl".to_string());

        if has_cmd && has_ctrl {
            result.warnings.push(ShortcutValidationWarning {
                warning_type: "cmd_ctrl_combination".to_string(),
                message: format!(
                    "{}类别中的快捷键同时包含 cmd 和 ctrl，这在大多数情况下是不合理的",
                    category
                ),
                shortcut: Some(shortcut.clone()),
            });
        }

        // 检查修饰键数量是否过多
        if modifiers.len() > 3 {
            result.warnings.push(ShortcutValidationWarning {
                warning_type: "too_many_modifiers".to_string(),
                message: format!(
                    "{}类别中的快捷键包含过多修饰键（{}个），可能难以使用",
                    category,
                    modifiers.len()
                ),
                shortcut: Some(shortcut.clone()),
            });
        }

        Ok(())
    }

    /// 初始化有效按键列表
    fn initialize_valid_keys(&mut self) {
        // 字母键
        for c in 'a'..='z' {
            self.valid_keys.insert(c.to_string());
        }
        for c in 'A'..='Z' {
            self.valid_keys.insert(c.to_string());
        }

        // 数字键
        for n in '0'..='9' {
            self.valid_keys.insert(n.to_string());
        }

        // 功能键
        for i in 1..=12 {
            self.valid_keys.insert(format!("F{}", i));
        }

        // 特殊键
        let special_keys = [
            "Space",
            "Enter",
            "Return",
            "Tab",
            "Backspace",
            "Delete",
            "Escape",
            "Esc",
            "ArrowUp",
            "ArrowDown",
            "ArrowLeft",
            "ArrowRight",
            "Up",
            "Down",
            "Left",
            "Right",
            "Home",
            "End",
            "PageUp",
            "PageDown",
            "Insert",
            "=",
            "-",
            "+",
            "[",
            "]",
            "\\",
            ";",
            "'",
            ",",
            ".",
            "/",
            "`",
            "~",
            "!",
            "@",
            "#",
            "$",
            "%",
            "^",
            "&",
            "*",
            "(",
            ")",
            "_",
            "{",
            "}",
            "|",
            ":",
            "\"",
            "<",
            ">",
            "?",
        ];

        for key in &special_keys {
            self.valid_keys.insert(key.to_string());
        }
    }

    /// 初始化有效修饰键列表
    fn initialize_valid_modifiers(&mut self) {
        let modifiers = ["cmd", "ctrl", "alt", "shift", "meta", "super", "win"];
        for modifier in &modifiers {
            self.valid_modifiers.insert(modifier.to_string());
        }
    }

    /// 初始化有效动作类型列表
    fn initialize_valid_action_types(&mut self) {
        let action_types = [
            "send_text",
            "run_command",
            "execute_script",
            "open_url",
            "copy_to_clipboard",
            "paste_from_clipboard",
        ];
        for action_type in &action_types {
            self.valid_action_types.insert(action_type.to_string());
        }
    }
}

impl Default for ShortcutValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::{ShortcutAction, ShortcutBinding};

    #[test]
    fn test_validator_creation() {
        let validator = ShortcutValidator::new();
        assert!(!validator.valid_keys.is_empty());
        assert!(!validator.valid_modifiers.is_empty());
        assert!(!validator.valid_action_types.is_empty());
    }

    #[test]
    fn test_valid_shortcut_binding() {
        let validator = ShortcutValidator::new();
        let binding = ShortcutBinding {
            key: "c".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("copy_to_clipboard".to_string()),
        };

        let result = validator.validate_shortcut_binding(&binding).unwrap();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_invalid_key() {
        let validator = ShortcutValidator::new();
        let binding = ShortcutBinding {
            key: "invalid_key".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("copy_to_clipboard".to_string()),
        };

        let result = validator.validate_shortcut_binding(&binding).unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
        assert_eq!(result.errors[0].error_type, "invalid_key");
    }

    #[test]
    fn test_invalid_modifier() {
        let validator = ShortcutValidator::new();
        let binding = ShortcutBinding {
            key: "c".to_string(),
            modifiers: vec!["invalid_modifier".to_string()],
            action: ShortcutAction::Simple("copy_to_clipboard".to_string()),
        };

        let result = validator.validate_shortcut_binding(&binding).unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
        assert_eq!(result.errors[0].error_type, "invalid_modifier");
    }

    #[test]
    fn test_empty_key() {
        let validator = ShortcutValidator::new();
        let binding = ShortcutBinding {
            key: "".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("copy_to_clipboard".to_string()),
        };

        let result = validator.validate_shortcut_binding(&binding).unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
        assert_eq!(result.errors[0].error_type, "empty_key");
    }

    #[test]
    fn test_duplicate_modifiers() {
        let validator = ShortcutValidator::new();
        let binding = ShortcutBinding {
            key: "c".to_string(),
            modifiers: vec!["cmd".to_string(), "cmd".to_string()],
            action: ShortcutAction::Simple("copy_to_clipboard".to_string()),
        };

        let result = validator.validate_shortcut_binding(&binding).unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
        assert_eq!(result.errors[0].error_type, "duplicate_modifier");
    }

    #[test]
    fn test_complex_action_validation() {
        let validator = ShortcutValidator::new();

        // 有效的复杂动作
        let valid_binding = ShortcutBinding {
            key: "l".to_string(),
            modifiers: vec!["cmd".to_string(), "shift".to_string()],
            action: ShortcutAction::Complex {
                action_type: "send_text".to_string(),
                text: Some("ls -la".to_string()),
            },
        };

        let result = validator.validate_shortcut_binding(&valid_binding).unwrap();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());

        // 无效的动作类型
        let invalid_binding = ShortcutBinding {
            key: "l".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Complex {
                action_type: "invalid_action".to_string(),
                text: Some("test".to_string()),
            },
        };

        let result = validator
            .validate_shortcut_binding(&invalid_binding)
            .unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
        assert_eq!(result.errors[0].error_type, "invalid_action_type");
    }

    #[test]
    fn test_send_text_without_text() {
        let validator = ShortcutValidator::new();
        let binding = ShortcutBinding {
            key: "l".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Complex {
                action_type: "send_text".to_string(),
                text: None,
            },
        };

        let result = validator.validate_shortcut_binding(&binding).unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
        assert_eq!(result.errors[0].error_type, "missing_text");
    }

    #[test]
    fn test_no_modifiers_warning() {
        let validator = ShortcutValidator::new();
        let binding = ShortcutBinding {
            key: "a".to_string(),
            modifiers: vec![],
            action: ShortcutAction::Simple("some_action".to_string()),
        };

        let result = validator.validate_shortcut_binding(&binding).unwrap();
        assert!(result.is_valid); // 仍然有效，但有警告
        assert!(!result.warnings.is_empty());
        assert_eq!(result.warnings[0].warning_type, "no_modifiers");
    }

    #[test]
    fn test_cmd_ctrl_combination_warning() {
        let validator = ShortcutValidator::new();
        let binding = ShortcutBinding {
            key: "c".to_string(),
            modifiers: vec!["cmd".to_string(), "ctrl".to_string()],
            action: ShortcutAction::Simple("copy_to_clipboard".to_string()),
        };

        let result = validator.validate_shortcut_binding(&binding).unwrap();
        assert!(result.is_valid); // 仍然有效，但有警告
        assert!(!result.warnings.is_empty());
        assert_eq!(result.warnings[0].warning_type, "cmd_ctrl_combination");
    }

    #[test]
    fn test_too_many_modifiers_warning() {
        let validator = ShortcutValidator::new();
        let binding = ShortcutBinding {
            key: "c".to_string(),
            modifiers: vec![
                "cmd".to_string(),
                "ctrl".to_string(),
                "alt".to_string(),
                "shift".to_string(),
            ],
            action: ShortcutAction::Simple("copy_to_clipboard".to_string()),
        };

        let result = validator.validate_shortcut_binding(&binding).unwrap();
        assert!(result.is_valid); // 仍然有效，但有警告
        assert!(!result.warnings.is_empty());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.warning_type == "too_many_modifiers"));
    }
}
