/*!
 * CSS 主题解析器
 *
 * 用于解析 CSS 主题文件并提取颜色变量，转换为 TOML 主题配置
 */

use crate::config::types::{AnsiColors, ColorScheme, SyntaxHighlight, Theme, ThemeType, UIColors};
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;


/// CSS 变量解析器
pub struct CssThemeParser {
    /// CSS 变量映射
    variables: HashMap<String, String>,
}

impl CssThemeParser {
    /// 创建新的 CSS 解析器
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// 从 CSS 文件解析主题
    pub fn parse_css_theme<P: AsRef<Path>>(css_file_path: P) -> AppResult<Theme> {
        let css_file_path = css_file_path.as_ref();
        let css_content = fs::read_to_string(css_file_path)
            .with_context(|| format!("无法读取 CSS 文件: {}", css_file_path.display()))?;

        let mut parser = Self::new();
        parser.parse_css_content(&css_content)?;

        // 从文件名推断主题名称
        let theme_name = css_file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        parser.create_theme(&theme_name)
    }

    /// 解析 CSS 内容
    fn parse_css_content(&mut self, css_content: &str) -> AppResult<()> {
        // 匹配 CSS 变量的正则表达式
        let var_regex = Regex::new(r"--([a-zA-Z0-9-_]+):\s*([^;]+);")
            .map_err(|e| anyhow!("正则表达式编译失败: {}", e))?;

        for cap in var_regex.captures_iter(css_content) {
            let var_name = cap[1].to_string();
            let var_value = cap[2].trim().to_string();

            // 清理颜色值（移除注释等）
            let cleaned_value = self.clean_color_value(&var_value);
            self.variables.insert(var_name, cleaned_value);
        }

        Ok(())
    }

    /// 清理颜色值
    fn clean_color_value(&self, value: &str) -> String {
        // 移除行内注释
        let value = if let Some(comment_pos) = value.find("/*") {
            &value[..comment_pos]
        } else {
            value
        };

        value.trim().to_string()
    }

    /// 创建主题
    fn create_theme(&self, theme_name: &str) -> AppResult<Theme> {
        let theme_type = self.detect_theme_type();

        Ok(Theme {
            name: theme_name.to_string(),
            theme_type,
            colors: self.create_color_scheme()?,
            syntax: self.create_syntax_highlight()?,
            ui: self.create_ui_colors()?,
        })
    }

    /// 检测主题类型
    fn detect_theme_type(&self) -> ThemeType {
        // 通过背景色判断主题类型
        if let Some(bg_color) = self
            .get_variable("color-background")
            .or_else(|| self.get_variable("terminal-bg"))
        {
            // 简单的亮度检测
            if self.is_dark_color(&bg_color) {
                ThemeType::Dark
            } else {
                ThemeType::Light
            }
        } else {
            // 默认为深色主题
            ThemeType::Dark
        }
    }

    /// 判断是否为深色
    fn is_dark_color(&self, color: &str) -> bool {
        // 移除 # 前缀
        let color = color.trim_start_matches('#');

        if color.len() == 6 {
            // 解析 RGB 值
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&color[0..2], 16),
                u8::from_str_radix(&color[2..4], 16),
                u8::from_str_radix(&color[4..6], 16),
            ) {
                // 计算亮度 (使用 ITU-R BT.709 标准)
                let luminance = 0.2126 * (r as f32) + 0.7152 * (g as f32) + 0.0722 * (b as f32);
                return luminance < 128.0;
            }
        }

        // 默认判断为深色
        true
    }

    /// 获取变量值
    fn get_variable(&self, name: &str) -> Option<String> {
        self.variables.get(name).cloned()
    }

    /// 获取变量值或默认值
    fn get_variable_or_default(&self, name: &str, default: &str) -> String {
        self.get_variable(name)
            .unwrap_or_else(|| default.to_string())
    }

    /// 创建颜色方案
    fn create_color_scheme(&self) -> AppResult<ColorScheme> {
        Ok(ColorScheme {
            foreground: self.get_variable_or_default("terminal-fg", "#ffffff"),
            background: self.get_variable_or_default("terminal-bg", "#000000"),
            cursor: self.get_variable_or_default("terminal-cursor", "#ffffff"),
            selection: self.get_variable_or_default("terminal-selection", "rgba(255,255,255,0.3)"),
            ansi: AnsiColors {
                black: self.get_variable_or_default("terminal-black", "#000000"),
                red: self.get_variable_or_default("terminal-red", "#ff0000"),
                green: self.get_variable_or_default("terminal-green", "#00ff00"),
                yellow: self.get_variable_or_default("terminal-yellow", "#ffff00"),
                blue: self.get_variable_or_default("terminal-blue", "#0000ff"),
                magenta: self.get_variable_or_default("terminal-magenta", "#ff00ff"),
                cyan: self.get_variable_or_default("terminal-cyan", "#00ffff"),
                white: self.get_variable_or_default("terminal-white", "#ffffff"),
            },
            bright: AnsiColors {
                black: self.get_variable_or_default("terminal-bright-black", "#808080"),
                red: self.get_variable_or_default("terminal-bright-red", "#ff8080"),
                green: self.get_variable_or_default("terminal-bright-green", "#80ff80"),
                yellow: self.get_variable_or_default("terminal-bright-yellow", "#ffff80"),
                blue: self.get_variable_or_default("terminal-bright-blue", "#8080ff"),
                magenta: self.get_variable_or_default("terminal-bright-magenta", "#ff80ff"),
                cyan: self.get_variable_or_default("terminal-bright-cyan", "#80ffff"),
                white: self.get_variable_or_default("terminal-bright-white", "#ffffff"),
            },
        })
    }

    /// 创建语法高亮配置
    fn create_syntax_highlight(&self) -> AppResult<SyntaxHighlight> {
        Ok(SyntaxHighlight {
            keyword: self.get_variable_or_default("terminal-blue", "#0000ff"),
            string: self.get_variable_or_default("terminal-green", "#00ff00"),
            comment: self.get_variable_or_default("text-muted", "#808080"),
            number: self.get_variable_or_default("terminal-magenta", "#ff00ff"),
            function: self.get_variable_or_default("terminal-cyan", "#00ffff"),
            variable: self.get_variable_or_default("terminal-yellow", "#ffff00"),
            type_name: self.get_variable_or_default("terminal-blue", "#0000ff"),
            operator: self.get_variable_or_default("terminal-red", "#ff0000"),
        })
    }

    /// 创建 UI 颜色配置
    fn create_ui_colors(&self) -> AppResult<UIColors> {
        Ok(UIColors {
            primary: self.get_variable_or_default("color-primary", "#007acc"),
            secondary: self.get_variable_or_default("color-secondary", "#6f42c1"),
            success: self.get_variable_or_default("terminal-green", "#00ff00"),
            warning: self.get_variable_or_default("terminal-yellow", "#ffff00"),
            error: self.get_variable_or_default("terminal-red", "#ff0000"),
            info: self.get_variable_or_default("terminal-cyan", "#00ffff"),
            border: self.get_variable_or_default("border-color", "#cccccc"),
            divider: self.get_variable_or_default("color-border", "#cccccc"),
        })
    }
}

impl Default for CssThemeParser {
    fn default() -> Self {
        Self::new()
    }
}
