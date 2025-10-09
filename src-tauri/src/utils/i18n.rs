pub mod commands;

use crate::utils::language::{Language, LanguageManager};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::LazyLock;

type I18nMessages = HashMap<String, HashMap<String, Value>>;

static I18N_MESSAGES: LazyLock<std::sync::RwLock<I18nMessages>> =
    LazyLock::new(|| std::sync::RwLock::new(HashMap::new()));

/// 国际化管理器
pub struct I18nManager;

impl I18nManager {
    /// 初始化国际化系统
    pub fn initialize() -> Result<(), String> {
        Self::load_language_pack(Language::ZhCN)?;
        Self::load_language_pack(Language::EnUS)?;
        Ok(())
    }

    /// 加载指定语言包
    fn load_language_pack(language: Language) -> Result<(), String> {
        let lang_code = language.to_string();
        let json_content = Self::load_language_file(&lang_code)?;
        let messages: HashMap<String, Value> = serde_json::from_str(&json_content)
            .map_err(|e| format!("Failed to parse language file {}: {}", lang_code, e))?;

        if let Ok(mut i18n_messages) = I18N_MESSAGES.write() {
            i18n_messages.insert(lang_code, messages);
            Ok(())
        } else {
            Err("Failed to write to i18n message store".to_string())
        }
    }

    /// 从文件系统加载语言文件
    ///
    /// # Arguments
    /// * `lang_code` - 语言代码，如 "zh-CN", "en-US"
    fn load_language_file(lang_code: &str) -> Result<String, String> {
        // 在实际实现中，这里应该使用 std::fs::read_to_string 或 include_str! 宏
        // 为了演示，这里返回空的JSON结构
        match lang_code {
            "zh-CN" => Ok(include_str!("i18n/zh-CN.json").to_string()),
            "en-US" => Ok(include_str!("i18n/en-US.json").to_string()),
            _ => Err(format!("Unsupported language: {}", lang_code)),
        }
    }

    /// 获取国际化文本
    ///
    /// # Arguments
    /// * `key` - 消息键，支持嵌套格式如 "module.section.message"
    /// * `params` - 可选的参数映射，用于文本插值
    ///
    /// # Returns
    /// 国际化后的文本，找不到时返回键本身
    pub fn get_text(key: &str, params: Option<&HashMap<String, String>>) -> String {
        let current_lang = LanguageManager::get_language().to_string();

        // 首先尝试当前语言
        if let Some(text) = Self::get_text_for_language(&current_lang, key) {
            return Self::interpolate_params(&text, params);
        }

        // 回退到中文
        if current_lang != "zh-CN" {
            if let Some(text) = Self::get_text_for_language("zh-CN", key) {
                return Self::interpolate_params(&text, params);
            }
        }

        key.to_string()
    }

    /// 获取指定语言的文本
    ///
    /// # Arguments
    /// * `lang_code` - 语言代码
    /// * `key` - 消息键
    fn get_text_for_language(lang_code: &str, key: &str) -> Option<String> {
        let i18n_messages = I18N_MESSAGES.read().ok()?;
        let messages = i18n_messages.get(lang_code)?;

        Self::get_nested_value(messages, key)
    }

    /// 从嵌套结构中获取值
    ///
    /// 支持 "module.section.message" 格式的键
    fn get_nested_value(messages: &HashMap<String, Value>, key: &str) -> Option<String> {
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = messages.get(parts[0])?;

        for &part in &parts[1..] {
            current = current.as_object()?.get(part)?;
        }

        match current {
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// 参数插值
    ///
    /// 将 {param_name} 格式的占位符替换为实际参数值
    fn interpolate_params(text: &str, params: Option<&HashMap<String, String>>) -> String {
        if let Some(params) = params {
            let mut result = text.to_string();
            for (key, value) in params {
                let placeholder = format!("{{{}}}", key);
                result = result.replace(&placeholder, value);
            }
            result
        } else {
            text.to_string()
        }
    }

    /// 重新加载语言包
    ///
    /// 用于动态更新翻译内容
    pub fn reload() -> Result<(), String> {
        if let Ok(mut i18n_messages) = I18N_MESSAGES.write() {
            i18n_messages.clear();
        }
        Self::initialize()
    }

    /// 添加或更新消息
    ///
    /// 用于运行时动态添加翻译内容
    pub fn add_message(lang_code: &str, key: &str, value: &str) -> Result<(), String> {
        let mut i18n_messages = I18N_MESSAGES
            .write()
            .map_err(|_| "Failed to acquire i18n message store write lock")?;

        let messages = i18n_messages
            .entry(lang_code.to_string())
            .or_insert_with(HashMap::new);

        messages.insert(key.to_string(), Value::String(value.to_string()));
        Ok(())
    }

    /// 检查键是否存在
    pub fn has_key(key: &str) -> bool {
        let current_lang = LanguageManager::get_language().to_string();
        Self::get_text_for_language(&current_lang, key).is_some()
            || Self::get_text_for_language("zh-CN", key).is_some()
    }

    /// 获取所有已加载的语言
    pub fn get_loaded_languages() -> Vec<String> {
        I18N_MESSAGES
            .read()
            .map(|messages| messages.keys().cloned().collect())
            .unwrap_or_default()
    }
}

/// 便捷的国际化宏
///
/// 用法：
/// - `t!("common.success")` - 简单文本
/// - `t!("error.with_param", "name" => "文件名")` - 带参数的文本
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::utils::i18n::I18nManager::get_text($key, None)
    };

    ($key:expr, $($param_key:expr => $param_value:expr),+ $(,)?) => {{
        let mut params = std::collections::HashMap::new();
        $(
            params.insert($param_key.to_string(), $param_value.to_string());
        )+
        $crate::utils::i18n::I18nManager::get_text($key, Some(&params))
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_params() {
        let text = "Hello {name}, you have {count} messages";
        let mut params = HashMap::new();
        params.insert("name".to_string(), "Alice".to_string());
        params.insert("count".to_string(), "5".to_string());

        let result = I18nManager::interpolate_params(text, Some(&params));
        assert_eq!(result, "Hello Alice, you have 5 messages");
    }

    #[test]
    fn test_nested_value() {
        let mut messages = HashMap::new();
        let mut common = HashMap::new();
        common.insert("success".to_string(), Value::String("成功".to_string()));
        let mut common_obj = serde_json::Map::new();
        for (k, v) in common {
            common_obj.insert(k, v);
        }
        messages.insert("common".to_string(), Value::Object(common_obj));

        let result = I18nManager::get_nested_value(&messages, "common.success");
        assert_eq!(result, Some("成功".to_string()));
    }

    #[test]
    fn test_macro() {
        // 这些测试需要在初始化I18n后运行
    }
}
