/*!
 * 快捷键系统类型定义
 *
 * 定义快捷键系统的核心数据结构和枚举
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCombination {
    pub key: String,
    pub modifiers: Vec<String>,
}

impl KeyCombination {
    pub fn new(key: String, modifiers: Vec<String>) -> Self {
        let mut sorted_modifiers = modifiers;
        sorted_modifiers.sort();
        Self {
            key,
            modifiers: sorted_modifiers,
        }
    }

    pub fn from_binding(binding: &crate::config::types::ShortcutBinding) -> Self {
        Self::new(binding.key.clone(), binding.modifiers.clone())
    }
}

impl fmt::Display for KeyCombination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.is_empty() {
            write!(f, "{}", self.key)
        } else {
            write!(f, "{}+{}", self.modifiers.join("+"), self.key)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionContext {
    pub key_combination: KeyCombination,
    pub active_terminal_id: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub error_type: ValidationErrorType,
    pub message: String,
    pub key_combination: Option<KeyCombination>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationErrorType {
    EmptyKey,
    InvalidModifier,
    InvalidAction,
    DuplicateBinding,
    SystemReserved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub warning_type: ValidationWarningType,
    pub message: String,
    pub key_combination: Option<KeyCombination>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationWarningType {
    UnregisteredAction,
    PlatformSpecific,
    Performance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub key_combination: KeyCombination,
    pub conflicting_bindings: Vec<ConflictingBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictingBinding {
    pub action: String,
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResult {
    pub has_conflicts: bool,
    pub conflicts: Vec<ConflictInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutStatistics {
    pub total_count: usize,
    pub conflict_count: usize,
    pub popular_modifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult<T = ()> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> OperationResult<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn success_empty() -> OperationResult<()> {
        OperationResult {
            success: true,
            data: Some(()),
            error: None,
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutEventType {
    KeyPressed,
    ActionExecuted,
    ActionFailed,
    ConfigUpdated,
    ConflictDetected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutEvent {
    pub event_type: ShortcutEventType,
    pub key_combination: Option<KeyCombination>,
    pub action: Option<String>,
    pub data: HashMap<String, serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchOptions {
    pub query: Option<String>,
    pub key: Option<String>,
    pub modifiers: Option<Vec<String>>,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub matches: Vec<SearchMatch>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    pub index: usize,
    pub binding: crate::config::types::ShortcutBinding,
    pub score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_combination_formatting() {
        let combo = KeyCombination::new(
            "c".to_string(),
            vec!["cmd".to_string(), "shift".to_string()],
        );
        assert_eq!(combo.to_string(), "cmd+shift+c");
    }

    #[test]
    fn test_operation_result() {
        let success_result = OperationResult::success("test".to_string());
        assert!(success_result.success);
        assert_eq!(success_result.data, Some("test".to_string()));

        let failure_result = OperationResult::<String>::failure("error".to_string());
        assert!(!failure_result.success);
        assert_eq!(failure_result.error, Some("error".to_string()));
    }
}
