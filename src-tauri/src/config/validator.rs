/*!
 * 配置验证器模块
 *
 * 提供配置数据的基本类型验证和范围检查。
 */

use super::types::*;
use crate::utils::error::AppResult;
use anyhow::{bail, Context};

use serde_json::Value;
use std::collections::HashMap;

/// 简化的配置验证器
///
/// 负责验证配置数据的基本正确性。
pub struct ConfigValidator {
    /// 内置验证规则
    builtin_rules: HashMap<String, fn(&Value) -> AppResult<()>>,
}

/// 简化的验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 是否通过验证
    pub is_valid: bool,
    /// 验证错误列表
    pub errors: Vec<ValidationError>,
}

/// 验证错误
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// 字段路径
    pub field_path: String,
    /// 错误消息
    pub message: String,
}

impl ConfigValidator {
    /// 创建新的配置验证器
    pub fn new() -> Self {
        let mut validator = Self {
            builtin_rules: HashMap::new(),
        };

        validator.register_builtin_rules();
        validator
    }

    /// 验证完整配置
    pub fn validate_config(&self, config: &AppConfig) -> AppResult<ValidationResult> {
        let config_value = serde_json::to_value(config).context("配置序列化失败")?;
        self.validate_value(&config_value, "")
    }

    /// 验证配置值
    pub fn validate_value(&self, value: &Value, field_path: &str) -> AppResult<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
        };

        // 递归验证所有字段
        self.validate_recursive(value, field_path, &mut result)?;

        Ok(result)
    }

    /// 递归验证配置值
    fn validate_recursive(
        &self,
        value: &Value,
        field_path: &str,
        result: &mut ValidationResult,
    ) -> AppResult<()> {
        // 检查当前字段是否有验证规则
        if let Some(rule) = self.builtin_rules.get(field_path) {
            if let Err(e) = rule(value) {
                result.errors.push(ValidationError {
                    field_path: field_path.to_string(),
                    message: e.to_string(),
                });
                result.is_valid = false;
            }
        }

        // 递归处理子字段
        if let Value::Object(obj) = value {
            for (key, val) in obj {
                let child_path = if field_path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", field_path, key)
                };
                self.validate_recursive(val, &child_path, result)?;
            }
        }

        Ok(())
    }

    /// 注册内置验证规则
    fn register_builtin_rules(&mut self) {
        // 应用配置验证
        self.builtin_rules
            .insert("app.language".to_string(), Self::validate_language);

        // 字体配置验证
        self.builtin_rules
            .insert("appearance.font.size".to_string(), Self::validate_font_size);

        self.builtin_rules.insert(
            "appearance.font.line_height".to_string(),
            Self::validate_line_height,
        );

        // 光标配置验证
        self.builtin_rules.insert(
            "terminal.cursor.blink".to_string(),
            Self::validate_cursor_blink,
        );

        self.builtin_rules.insert(
            "terminal.cursor.thickness".to_string(),
            Self::validate_cursor_thickness,
        );

        // 滚动配置验证
        self.builtin_rules
            .insert("terminal.scrollback".to_string(), Self::validate_scrollback);




    }

    // ============================================================================
    // 简化的验证函数
    // ============================================================================

    /// 验证语言设置
    fn validate_language(value: &Value) -> AppResult<()> {
        if let Some(lang) = value.as_str() {
            let supported_languages = ["en", "zh-CN", "zh-TW", "ja", "ko", "fr", "de", "es"];
            if supported_languages.contains(&lang) {
                Ok(())
            } else {
                bail!("不支持的语言: {}", lang)
            }
        } else {
            bail!("语言设置必须是字符串类型")
        }
    }

    /// 验证字体大小
    fn validate_font_size(value: &Value) -> AppResult<()> {
        if let Some(size) = value.as_f64() {
            if (8.0..=72.0).contains(&size) {
                Ok(())
            } else {
                bail!("字体大小必须在 8.0-72.0 之间，当前值: {}", size)
            }
        } else {
            bail!("字体大小必须是数字类型")
        }
    }

    /// 验证行高
    fn validate_line_height(value: &Value) -> AppResult<()> {
        if let Some(height) = value.as_f64() {
            if (0.5..=3.0).contains(&height) {
                Ok(())
            } else {
                bail!("行高必须在 0.5-3.0 之间，当前值: {}", height)
            }
        } else {
            bail!("行高必须是数字类型")
        }
    }

    /// 验证光标闪烁
    fn validate_cursor_blink(value: &Value) -> AppResult<()> {
        if value.as_bool().is_some() {
            Ok(())
        } else {
            bail!("光标闪烁设置必须是布尔类型")
        }
    }

    /// 验证光标粗细
    fn validate_cursor_thickness(value: &Value) -> AppResult<()> {
        if let Some(thickness) = value.as_f64() {
            if (0.1..=5.0).contains(&thickness) {
                Ok(())
            } else {
                bail!("光标粗细必须在 0.1-5.0 之间，当前值: {}", thickness)
            }
        } else {
            bail!("光标粗细必须是数字类型")
        }
    }

    /// 验证滚动缓冲区
    fn validate_scrollback(value: &Value) -> AppResult<()> {
        if let Some(size) = value.as_u64() {
            if (100..=100000).contains(&size) {
                Ok(())
            } else {
                bail!("滚动缓冲区行数必须在 100-100000 之间，当前值: {}", size)
            }
        } else {
            bail!("滚动缓冲区行数必须是正整数")
        }
    }


}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 验证结果实现
// ============================================================================

impl ValidationResult {
    /// 创建成功的验证结果
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
        }
    }

    /// 创建失败的验证结果
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            errors,
        }
    }

    /// 添加错误
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
        self.is_valid = false;
    }

    /// 合并验证结果
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.is_valid = self.is_valid && other.is_valid;
    }

    /// 获取错误摘要
    pub fn error_summary(&self) -> String {
        if self.errors.is_empty() {
            "无错误".to_string()
        } else {
            format!("发现 {} 个错误", self.errors.len())
        }
    }

    /// 获取详细错误信息
    pub fn detailed_errors(&self) -> Vec<String> {
        self.errors
            .iter()
            .map(|e| format!("{}: {}", e.field_path, e.message))
            .collect()
    }
}

impl ValidationError {
    /// 创建新的验证错误
    pub fn new(field_path: String, message: String) -> Self {
        Self {
            field_path,
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::defaults::create_default_config;

    #[test]
    fn test_validator_creation() {
        let validator = ConfigValidator::new();
        assert!(!validator.builtin_rules.is_empty());
    }

    #[test]
    fn test_validate_default_config() {
        let validator = ConfigValidator::new();
        let config = create_default_config();

        let result = validator.validate_config(&config).unwrap();
        assert!(result.is_valid, "默认配置应该通过验证");
        assert!(result.errors.is_empty(), "默认配置不应该有错误");
    }

    #[test]
    fn test_validation_result_operations() {
        let mut result = ValidationResult::success();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());

        let error = ValidationError::new("test.field".to_string(), "测试错误".to_string());

        result.add_error(error);
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_language_validation() {
        // 测试有效语言
        let valid_languages = ["en", "zh-CN", "zh-TW", "ja", "ko"];
        for lang in valid_languages {
            let value = Value::String(lang.to_string());
            let result = ConfigValidator::validate_language(&value);
            assert!(result.is_ok(), "语言 '{}' 应该通过验证", lang);
        }

        // 测试无效语言
        let invalid_lang = Value::String("invalid-lang".to_string());
        let result = ConfigValidator::validate_language(&invalid_lang);
        assert!(result.is_err(), "无效语言应该验证失败");
    }

    #[test]
    fn test_font_size_validation() {
        // 测试有效字体大小
        let valid_sizes = [8.0, 12.0, 16.0, 24.0, 72.0];
        for size in valid_sizes {
            let value = Value::Number(serde_json::Number::from_f64(size).unwrap());
            let result = ConfigValidator::validate_font_size(&value);
            assert!(result.is_ok(), "字体大小 {} 应该通过验证", size);
        }

        // 测试无效字体大小
        let invalid_sizes = [7.0, 73.0, -1.0];
        for size in invalid_sizes {
            let value = Value::Number(serde_json::Number::from_f64(size).unwrap());
            let result = ConfigValidator::validate_font_size(&value);
            assert!(result.is_err(), "字体大小 {} 应该验证失败", size);
        }
    }



    #[test]
    fn test_complex_config_validation() {
        let validator = ConfigValidator::new();

        // 创建一个包含多种错误的配置
        let mut config = create_default_config();

        // 修改配置使其包含错误
        config.app.language = "invalid-lang".to_string();
        config.appearance.font.size = 5.0; // 太小

        let result = validator.validate_config(&config).unwrap();

        assert!(!result.is_valid, "包含错误的配置应该验证失败");
        assert!(!result.errors.is_empty(), "应该有多个验证错误");

        // 检查是否包含预期的错误
        let error_messages: Vec<String> = result.errors.iter().map(|e| e.message.clone()).collect();

        let has_language_error = error_messages
            .iter()
            .any(|msg| msg.contains("不支持的语言"));
        let has_font_error = error_messages
            .iter()
            .any(|msg| msg.contains("字体大小必须在"));
        let has_opacity_error = error_messages
            .iter()
            .any(|msg| msg.contains("窗口透明度必须在"));

        assert!(has_language_error, "应该有语言验证错误");
        assert!(has_font_error, "应该有字体大小验证错误");
        assert!(has_opacity_error, "应该有透明度验证错误");
    }
}
