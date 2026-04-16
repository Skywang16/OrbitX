pub mod commands;

use crate::utils::language::{Language, LanguageManager};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::LazyLock;

type I18nMessages = HashMap<String, HashMap<String, Value>>;

static I18N_MESSAGES: LazyLock<std::sync::RwLock<I18nMessages>> =
    LazyLock::new(|| std::sync::RwLock::new(HashMap::new()));

/// Internationalization manager
pub struct I18nManager;

impl I18nManager {
    /// Initialize internationalization system
    pub fn initialize() -> Result<(), String> {
        Self::load_language_pack(Language::ZhCN)?;
        Self::load_language_pack(Language::EnUS)?;
        Ok(())
    }

    /// Load specified language pack
    fn load_language_pack(language: Language) -> Result<(), String> {
        let lang_code = language.to_string();
        let json_content = Self::load_language_file(&lang_code)?;
        let messages: HashMap<String, Value> = serde_json::from_str(&json_content)
            .map_err(|e| format!("Failed to parse language file {lang_code}: {e}"))?;

        match I18N_MESSAGES.write() {
            Ok(mut i18n_messages) => {
                i18n_messages.insert(lang_code, messages);
                Ok(())
            }
            Err(err) => Err(format!("Failed to write to i18n message store: {err}")),
        }
    }

    /// Load language file from filesystem
    ///
    /// # Arguments
    /// * `lang_code` - Language code, e.g., "zh-CN", "en-US"
    fn load_language_file(lang_code: &str) -> Result<String, String> {
        // In actual implementation, should use std::fs::read_to_string or include_str! macro
        // For demonstration, return empty JSON structure here
        match lang_code {
            "zh-CN" => Ok(include_str!("i18n/zh-CN.json").to_string()),
            "en-US" => Ok(include_str!("i18n/en-US.json").to_string()),
            _ => Err(format!("Unsupported language: {lang_code}")),
        }
    }

    /// Get internationalized text
    ///
    /// # Arguments
    /// * `key` - Message key, supports nested format like "module.section.message"
    /// * `params` - Optional parameter map for text interpolation
    ///
    /// # Returns
    /// Internationalized text, returns key itself if not found
    pub fn get_text(key: &str, params: Option<&HashMap<String, String>>) -> String {
        let current_lang = LanguageManager::get_language().to_string();

        // First try current language
        if let Some(text) = Self::get_text_for_language(&current_lang, key) {
            return Self::interpolate_params(&text, params);
        }

        // Fallback to Chinese
        if current_lang != "zh-CN" {
            if let Some(text) = Self::get_text_for_language("zh-CN", key) {
                return Self::interpolate_params(&text, params);
            }
        }

        key.to_string()
    }

    /// Get text for specified language
    ///
    /// # Arguments
    /// * `lang_code` - Language code
    /// * `key` - Message key
    fn get_text_for_language(lang_code: &str, key: &str) -> Option<String> {
        let i18n_messages = match I18N_MESSAGES.read() {
            Ok(messages) => messages,
            Err(err) => {
                tracing::warn!("Failed to read i18n message store: {}", err);
                return None;
            }
        };
        let messages = i18n_messages.get(lang_code)?;

        Self::get_nested_value(messages, key)
    }

    pub fn get_string_array(key: &str) -> Option<Vec<String>> {
        let current_lang = LanguageManager::get_language().to_string();

        if let Some(values) = Self::get_string_array_for_language(&current_lang, key) {
            return Some(values);
        }

        if current_lang != "zh-CN" {
            return Self::get_string_array_for_language("zh-CN", key);
        }

        None
    }

    fn get_string_array_for_language(lang_code: &str, key: &str) -> Option<Vec<String>> {
        let i18n_messages = match I18N_MESSAGES.read() {
            Ok(messages) => messages,
            Err(err) => {
                tracing::warn!("Failed to read i18n message store: {}", err);
                return None;
            }
        };
        let messages = i18n_messages.get(lang_code)?;

        let value = Self::get_nested_json_value(messages, key)?;
        let array = value.as_array()?;
        let items = array
            .iter()
            .filter_map(|item| item.as_str().map(ToOwned::to_owned))
            .collect::<Vec<_>>();
        (!items.is_empty()).then_some(items)
    }

    /// Get value from nested structure
    ///
    /// Supports keys in "module.section.message" format
    fn get_nested_value(messages: &HashMap<String, Value>, key: &str) -> Option<String> {
        let current = Self::get_nested_json_value(messages, key)?;

        match current {
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    fn get_nested_json_value<'a>(
        messages: &'a HashMap<String, Value>,
        key: &str,
    ) -> Option<&'a Value> {
        let parts: Vec<&str> = key.split('.').collect();
        if parts.is_empty() {
            return None;
        }
        let mut current = messages.get(parts[0])?;

        // Safe slicing
        let nested_parts = match parts.get(1..) {
            Some(parts) => parts,
            None => &[],
        };
        for &part in nested_parts {
            current = current.as_object()?.get(part)?;
        }
        Some(current)
    }

    /// Parameter interpolation
    ///
    /// Replace placeholders in {param_name} format with actual parameter values
    fn interpolate_params(text: &str, params: Option<&HashMap<String, String>>) -> String {
        if let Some(params) = params {
            let mut result = text.to_string();
            for (key, value) in params {
                let placeholder = format!("{{{key}}}");
                result = result.replace(&placeholder, value);
            }
            result
        } else {
            text.to_string()
        }
    }

    /// Reload language pack
    ///
    /// Used for dynamically updating translation content
    pub fn reload() -> Result<(), String> {
        match I18N_MESSAGES.write() {
            Ok(mut i18n_messages) => i18n_messages.clear(),
            Err(err) => {
                return Err(format!("Failed to clear i18n message store: {err}"));
            }
        }
        Self::initialize()
    }
}

/// Convenient internationalization macro
///
/// Usage:
/// - `t!("common.success")` - Simple text
/// - `t!("error.with_param", "name" => "filename")` - Text with parameters
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
        // These tests need to run after initializing I18n
    }
}
