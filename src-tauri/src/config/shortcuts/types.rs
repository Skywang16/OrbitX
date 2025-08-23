/*!
 * 快捷键系统类型定义
 * 
 * 定义快捷键系统的核心数据结构和枚举
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 快捷键类别
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShortcutCategory {
    Global,
    Terminal, 
    Custom,
}

impl std::fmt::Display for ShortcutCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShortcutCategory::Global => write!(f, "global"),
            ShortcutCategory::Terminal => write!(f, "terminal"),
            ShortcutCategory::Custom => write!(f, "custom"),
        }
    }
}

impl std::str::FromStr for ShortcutCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "global" => Ok(ShortcutCategory::Global),
            "terminal" => Ok(ShortcutCategory::Terminal),
            "custom" => Ok(ShortcutCategory::Custom),
            _ => Err(format!("未知的快捷键类别: {}", s)),
        }
    }
}

/// 平台类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
}

/// 快捷键按键组合
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCombination {
    /// 主按键
    pub key: String,
    /// 修饰键列表
    pub modifiers: Vec<String>,
}

impl KeyCombination {
    /// 创建新的按键组合
    pub fn new(key: String, modifiers: Vec<String>) -> Self {
        let mut sorted_modifiers = modifiers;
        sorted_modifiers.sort(); // 确保修饰键顺序一致
        Self {
            key,
            modifiers: sorted_modifiers,
        }
    }

    /// 格式化为字符串表示
    pub fn to_string(&self) -> String {
        if self.modifiers.is_empty() {
            self.key.clone()
        } else {
            format!("{}+{}", self.modifiers.join("+"), self.key)
        }
    }

    /// 从快捷键绑定创建
    pub fn from_binding(binding: &crate::config::types::ShortcutBinding) -> Self {
        Self::new(binding.key.clone(), binding.modifiers.clone())
    }
}

/// 快捷键动作执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionContext {
    /// 触发的按键组合
    pub key_combination: KeyCombination,
    /// 当前活动的终端标签
    pub active_terminal_id: Option<String>,
    /// 附加数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 快捷键验证错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// 错误类型
    pub error_type: ValidationErrorType,
    /// 错误消息  
    pub message: String,
    /// 相关的按键组合
    pub key_combination: Option<KeyCombination>,
}

/// 验证错误类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationErrorType {
    EmptyKey,
    InvalidModifier,
    InvalidAction,
    DuplicateBinding,
    SystemReserved,
}

/// 快捷键验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// 是否通过验证
    pub is_valid: bool,
    /// 错误列表
    pub errors: Vec<ValidationError>,
    /// 警告列表
    pub warnings: Vec<ValidationWarning>,
}

/// 快捷键验证警告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// 警告类型
    pub warning_type: ValidationWarningType,
    /// 警告消息
    pub message: String,
    /// 相关的按键组合
    pub key_combination: Option<KeyCombination>,
}

/// 验证警告类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationWarningType {
    UnregisteredAction,
    PlatformSpecific,
    Performance,
}

/// 快捷键冲突信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// 冲突的按键组合
    pub key_combination: KeyCombination,
    /// 冲突的绑定列表
    pub conflicting_bindings: Vec<ConflictingBinding>,
}

/// 冲突的绑定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictingBinding {
    /// 类别
    pub category: ShortcutCategory,
    /// 动作名称
    pub action: String,
    /// 在类别中的索引
    pub index: usize,
}

/// 冲突检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResult {
    /// 是否有冲突
    pub has_conflicts: bool,
    /// 冲突列表
    pub conflicts: Vec<ConflictInfo>,
}

/// 快捷键统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutStatistics {
    /// 各类别的快捷键数量
    pub category_counts: HashMap<ShortcutCategory, usize>,
    /// 总数量
    pub total_count: usize,
    /// 冲突数量
    pub conflict_count: usize,
    /// 最常用的修饰键
    pub popular_modifiers: Vec<String>,
}

/// 快捷键操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult<T = ()> {
    /// 是否成功
    pub success: bool,
    /// 结果数据
    pub data: Option<T>,
    /// 错误消息
    pub error: Option<String>,
}

impl<T> OperationResult<T> {
    /// 创建成功结果
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// 创建成功结果（无数据）
    pub fn success_empty() -> OperationResult<()> {
        OperationResult {
            success: true,
            data: Some(()),
            error: None,
        }
    }

    /// 创建失败结果
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// 快捷键事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutEventType {
    KeyPressed,
    ActionExecuted,
    ActionFailed,
    ConfigUpdated,
    ConflictDetected,
}

/// 快捷键事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutEvent {
    /// 事件类型
    pub event_type: ShortcutEventType,
    /// 按键组合
    pub key_combination: Option<KeyCombination>,
    /// 执行的动作
    pub action: Option<String>,
    /// 事件数据
    pub data: HashMap<String, serde_json::Value>,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 快捷键搜索选项
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchOptions {
    /// 搜索关键词
    pub query: Option<String>,
    /// 限制的类别
    pub categories: Option<Vec<ShortcutCategory>>,
    /// 限制的按键
    pub key: Option<String>,
    /// 限制的修饰键
    pub modifiers: Option<Vec<String>>,
    /// 限制的动作
    pub action: Option<String>,
}

/// 快捷键搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 匹配的快捷键
    pub matches: Vec<SearchMatch>,
    /// 总匹配数
    pub total: usize,
}

/// 搜索匹配项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    /// 类别
    pub category: ShortcutCategory,
    /// 索引
    pub index: usize,
    /// 快捷键绑定
    pub binding: crate::config::types::ShortcutBinding,
    /// 匹配得分 (0.0-1.0)
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
    fn test_shortcut_category_parsing() {
        assert_eq!("global".parse::<ShortcutCategory>().unwrap(), ShortcutCategory::Global);
        assert_eq!("terminal".parse::<ShortcutCategory>().unwrap(), ShortcutCategory::Terminal);
        assert_eq!("custom".parse::<ShortcutCategory>().unwrap(), ShortcutCategory::Custom);
        assert!("invalid".parse::<ShortcutCategory>().is_err());
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