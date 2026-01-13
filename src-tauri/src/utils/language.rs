/*!
 * 语言管理模块
 *
 * 提供全局的语言设置管理，支持中文和英文两种语言。
 * 使用线程安全的全局状态管理当前语言设置。
 */

use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// 支持的语言类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Language {
    /// 简体中文
    #[default]
    ZhCN,
    /// 美式英文
    EnUS,
}

impl Language {
    /// 从字符串解析语言类型
    ///
    /// # Arguments
    /// * `s` - 语言字符串，如 "zh-CN", "en-US"
    ///
    /// # Returns
    /// 对应的语言类型，无法识别时默认为中文
    pub fn from_tag_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "zh-cn" | "zh" | "chinese" | "中文" => Language::ZhCN,
            "en-us" | "en" | "english" | "英文" => Language::EnUS,
            _ => Language::ZhCN, // 默认中文
        }
    }

    /// 获取语言标识（BCP-47 tag）
    pub fn tag(&self) -> &'static str {
        match self {
            Language::ZhCN => "zh-CN",
            Language::EnUS => "en-US",
        }
    }

    /// 获取语言的本地化显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::ZhCN => "简体中文",
            Language::EnUS => "English",
        }
    }

    /// 获取所有支持的语言
    pub fn all() -> Vec<Language> {
        vec![Language::ZhCN, Language::EnUS]
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.tag())
    }
}

/// 全局语言状态
static CURRENT_LANGUAGE: LazyLock<std::sync::RwLock<Language>> =
    LazyLock::new(|| std::sync::RwLock::new(Language::ZhCN));

/// 语言管理器
///
/// 提供全局的语言设置和获取功能，确保线程安全。
pub struct LanguageManager;

impl LanguageManager {
    /// 设置当前语言
    ///
    /// # Arguments
    /// * `lang` - 要设置的语言
    ///
    /// # Returns
    /// 设置成功返回true，失败返回false
    pub fn set_language(lang: Language) -> bool {
        match CURRENT_LANGUAGE.write() {
            Ok(mut current_lang) => {
                *current_lang = lang;
                true
            }
            Err(_) => false,
        }
    }

    /// 获取当前语言
    ///
    /// # Returns
    /// 当前设置的语言，获取失败时返回默认语言（中文）
    pub fn get_language() -> Language {
        CURRENT_LANGUAGE
            .read()
            .map(|lang| lang.clone())
            .unwrap_or_default()
    }

    /// 从字符串设置语言
    ///
    /// # Arguments
    /// * `lang_str` - 语言字符串
    ///
    /// # Returns
    /// 设置成功返回true，失败返回false
    pub fn set_language_from_tag_lossy(lang_str: &str) -> bool {
        let lang = Language::from_tag_lossy(lang_str);
        Self::set_language(lang)
    }

    /// 获取当前语言的字符串表示
    pub fn get_language_string() -> String {
        Self::get_language().tag().to_string()
    }

    /// 检查当前语言是否为中文
    pub fn is_chinese() -> bool {
        matches!(Self::get_language(), Language::ZhCN)
    }

    /// 检查当前语言是否为英文
    pub fn is_english() -> bool {
        matches!(Self::get_language(), Language::EnUS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_str() {
        assert_eq!(Language::from_tag_lossy("zh-CN"), Language::ZhCN);
        assert_eq!(Language::from_tag_lossy("zh"), Language::ZhCN);
        assert_eq!(Language::from_tag_lossy("chinese"), Language::ZhCN);
        assert_eq!(Language::from_tag_lossy("中文"), Language::ZhCN);

        assert_eq!(Language::from_tag_lossy("en-US"), Language::EnUS);
        assert_eq!(Language::from_tag_lossy("en"), Language::EnUS);
        assert_eq!(Language::from_tag_lossy("english"), Language::EnUS);

        // 默认情况
        assert_eq!(Language::from_tag_lossy("unknown"), Language::ZhCN);
    }

    #[test]
    fn test_language_to_string() {
        assert_eq!(Language::ZhCN.tag(), "zh-CN");
        assert_eq!(Language::EnUS.tag(), "en-US");
    }

    #[test]
    fn test_language_manager() {
        // 测试设置和获取
        assert!(LanguageManager::set_language(Language::EnUS));
        assert_eq!(LanguageManager::get_language(), Language::EnUS);
        assert!(LanguageManager::is_english());
        assert!(!LanguageManager::is_chinese());

        // 恢复默认
        assert!(LanguageManager::set_language(Language::ZhCN));
        assert_eq!(LanguageManager::get_language(), Language::ZhCN);
        assert!(LanguageManager::is_chinese());
        assert!(!LanguageManager::is_english());
    }

    #[test]
    fn test_language_manager_from_str() {
        assert!(LanguageManager::set_language_from_tag_lossy("en-US"));
        assert_eq!(LanguageManager::get_language(), Language::EnUS);

        assert!(LanguageManager::set_language_from_tag_lossy("zh-CN"));
        assert_eq!(LanguageManager::get_language(), Language::ZhCN);
    }

    #[test]
    fn test_language_all() {
        let langs = Language::all();
        assert_eq!(langs.len(), 2);
        assert!(langs.contains(&Language::ZhCN));
        assert!(langs.contains(&Language::EnUS));
    }
}
