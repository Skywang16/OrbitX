/*!
 * 主题系统类型定义
 *
 * 包含主题相关的所有数据结构和类型定义。
 */

use serde::{Deserialize, Serialize};

/// 主题配置 (对应 TOML 中的 [appearance.theme_config] 节)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

    /// 颜色配置
    pub colors: ColorScheme,

    /// 语法高亮
    pub syntax: SyntaxHighlight,

    /// UI 颜色
    pub ui: UIColors,
}

/// 颜色方案
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ColorScheme {
    /// 前景色
    pub foreground: String,

    /// 背景色
    pub background: String,

    /// 光标颜色
    pub cursor: String,

    /// 选择颜色
    pub selection: String,

    /// ANSI 颜色
    pub ansi: AnsiColors,

    /// 明亮 ANSI 颜色
    pub bright: AnsiColors,
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

/// UI 颜色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct UIColors {
    /// 主色调
    pub primary: String,

    /// 次要色调
    pub secondary: String,

    /// 成功色
    pub success: String,

    /// 警告色
    pub warning: String,

    /// 错误色
    pub error: String,

    /// 信息色
    pub info: String,

    /// 边框色
    pub border: String,

    /// 分割线色
    pub divider: String,
}
