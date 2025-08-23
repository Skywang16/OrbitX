/*!
 * 主题系统类型定义
 *
 * 包含主题相关的所有数据结构和类型定义。
 */

use serde::{Deserialize, Serialize};

/// 主题配置 (对应 TOML 中的 [appearance.theme_config] 节)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeConfig {
    /// 自动切换时间
    pub auto_switch_time: String,

    /// 终端主题名称，引用themes/目录下的文件
    pub terminal_theme: String,

    /// 浅色主题
    pub light_theme: String,

    /// 深色主题
    pub dark_theme: String,

    /// 跟随系统主题
    pub follow_system: bool,
}

/// 主题类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeType {
    Light,
    Dark,
    Auto,
}

impl std::fmt::Display for ThemeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeType::Light => write!(f, "light"),
            ThemeType::Dark => write!(f, "dark"),
            ThemeType::Auto => write!(f, "auto"),
        }
    }
}

/// 主题定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Theme {
    /// 主题名称
    pub name: String,

    /// 主题类型
    pub theme_type: ThemeType,

    /// ANSI 颜色
    pub ansi: AnsiColors,

    /// 明亮 ANSI 颜色
    pub bright: AnsiColors,

    /// 语法高亮
    pub syntax: SyntaxHighlight,

    /// UI 颜色
    pub ui: UIColors,
}

/// ANSI 颜色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct AnsiColors {
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
}

/// 语法高亮
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct SyntaxHighlight {
    /// 关键字
    pub keyword: String,

    /// 字符串
    pub string: String,

    /// 注释
    pub comment: String,

    /// 数字
    pub number: String,

    /// 函数
    pub function: String,

    /// 变量
    pub variable: String,

    /// 类型
    pub type_name: String,

    /// 操作符
    pub operator: String,
}

/// UI 颜色 - 全新的数字层次系统
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct UIColors {
    // 背景色层次
    pub bg_100: String,
    pub bg_200: String,
    pub bg_300: String,
    pub bg_400: String,
    pub bg_500: String,
    pub bg_600: String,
    pub bg_700: String,

    // 边框层次
    pub border_200: String,
    pub border_300: String,
    pub border_400: String,

    // 文本层次
    pub text_100: String,
    pub text_200: String,
    pub text_300: String,
    pub text_400: String,
    pub text_500: String,

    // 状态颜色
    pub primary: String,
    pub primary_hover: String,
    pub primary_alpha: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub info: String,

    // 交互状态
    pub hover: String,
    pub active: String,
    pub focus: String,
    pub selection: String,
}
