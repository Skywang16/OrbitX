//! OSC序列解析器
//!
//! 解析终端输出中的OSC (Operating System Command) 控制序列
//! 支持完整的Shell Integration协议，包括命令生命周期、CWD同步等

use anyhow::Result;
use percent_encoding;
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use tracing::debug;
use url::Url;

/// Shell Integration状态
#[derive(Debug, Clone, PartialEq)]
pub enum ShellIntegrationState {
    /// 未启用
    Disabled,
    /// 检测中
    Detecting,
    /// 已启用
    Enabled,
}

/// 命令执行状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommandStatus {
    /// 准备中（显示提示符）
    Ready,
    /// 执行中
    Running,
    /// 完成
    Finished { exit_code: Option<i32> },
}

/// OSC序列类型 - 支持Shell Integration（OSC 633）、CWD（OSC 7）等
#[derive(Debug, Clone)]
pub enum OscSequence {
    /// OSC 7 - 当前工作目录
    CurrentWorkingDirectory { path: String },

    /// OSC 633 序列 - Shell Integration（命令生命周期）
    ShellIntegration {
        marker: IntegrationMarker,
        data: Option<String>,
    },

    /// OSC 9;9 序列 - Windows Terminal CWD
    WindowsTerminalCwd { path: String },

    /// OSC 0/1/2 - 窗口标题
    WindowTitle {
        title_type: WindowTitleType,
        title: String,
    },

    /// 未知OSC序列（用于调试）
    Unknown { number: String, data: String },
}

/// Shell Integration（OSC 633）标记
#[derive(Debug, Clone, PartialEq)]
pub enum IntegrationMarker {
    /// A - 命令开始前（提示符开始）
    PromptStart,
    /// B - 命令开始（用户输入完成）
    CommandStart,
    /// C - 命令执行开始
    CommandExecuted,
    /// D - 命令完成（带退出码）
    CommandFinished { exit_code: Option<i32> },
    /// E - 命令行继续（多行命令）
    CommandContinuation,
    /// F - 右侧提示符
    RightPrompt,
    /// G - 命令无效（如语法错误）
    CommandInvalid,
    /// H - 命令取消（如Ctrl+C）
    CommandCancelled,
    /// P - 属性设置
    Property { key: String, value: String },
}

/// 窗口标题类型
#[derive(Debug, Clone, PartialEq)]
pub enum WindowTitleType {
    /// OSC 0 - 同时设置图标和窗口标题
    Both,
    /// OSC 1 - 设置图标标题
    Icon,
    /// OSC 2 - 设置窗口标题
    Window,
}

/// OSC序列解析器
pub struct OscParser {
    /// OSC序列正则表达式
    osc_regex: Regex,
    /// Shell Integration (OSC 633) 正则表达式
    si_regex: Regex,
    /// 窗口标题正则表达式
    title_regex: Regex,
    /// CWD (OSC 7) 正则表达式
    cwd_regex: Regex,
}

impl OscParser {
    /// 创建新的OSC解析器
    pub fn new() -> Result<Self> {
        // 基本OSC序列正则表达式
        let osc_regex = Regex::new(r"\x1b]([0-9]+);([^\x07\x1b]*?)(?:\x07|\x1b\\)")?;

        // Shell Integration (OSC 633) - 大小写不敏感
        let si_regex = Regex::new(r"(?i)\x1b]633;([A-Z]);?([^\x07\x1b]*?)(?:\x07|\x1b\\)")?;

        // 窗口标题 (OSC 0/1/2)
        let title_regex = Regex::new(r"\x1b]([012]);([^\x07\x1b]*?)(?:\x07|\x1b\\)")?;

        // CWD (OSC 7)
        let cwd_regex = Regex::new(r"\x1b]7;([^\x07\x1b]*?)(?:\x07|\x1b\\)")?;

        Ok(Self {
            osc_regex,
            si_regex,
            title_regex,
            cwd_regex,
        })
    }

    /// 解析文本中的OSC序列
    pub fn parse(&self, data: &str) -> Vec<OscSequence> {
        let mut sequences = Vec::new();

        // 解析 Shell Integration 序列 (OSC 633)
        for captures in self.si_regex.captures_iter(data) {
            if let Some(marker_match) = captures.get(1) {
                let marker_str = marker_match.as_str();
                let data_str = captures.get(2).map(|m| m.as_str()).unwrap_or("");

                if let Some(sequence) = self.parse_shell_integration_sequence(marker_str, data_str)
                {
                    sequences.push(sequence);
                }
            }
        }

        // 解析CWD序列 (OSC 7)
        for captures in self.cwd_regex.captures_iter(data) {
            if let Some(data_match) = captures.get(1) {
                if let Some(sequence) = self.parse_cwd_sequence(data_match.as_str()) {
                    sequences.push(sequence);
                }
            }
        }

        sequences
    }

    /// 解析具体的OSC序列
    fn parse_osc_sequence(&self, number: &str, data: &str) -> Option<OscSequence> {
        match number {
            "9" => self.parse_windows_terminal_sequence(data),
            _ => None,
        }
    }

    /// 解析Shell Integration（OSC 633）序列
    fn parse_shell_integration_sequence(&self, marker: &str, data: &str) -> Option<OscSequence> {
        // 统一大写，兼容大小写标记
        let marker_up = marker.to_ascii_uppercase();
        let si_marker = match marker_up.as_str() {
            "A" => IntegrationMarker::PromptStart,
            "B" => IntegrationMarker::CommandStart,
            "C" => IntegrationMarker::CommandExecuted,
            "D" => {
                // D 可能包含退出码，兼容 "0"、"exit=0"、"status=0" 等形式
                let exit_code = if data.is_empty() {
                    None
                } else if let Ok(n) = data.parse::<i32>() {
                    Some(n)
                } else {
                    // 在以 ;、=、空白 分隔的 token 中查找第一个可解析为整数的片段
                    data.split(|c: char| c == ';' || c == '=' || c.is_whitespace())
                        .find_map(|tok| tok.parse::<i32>().ok())
                };
                IntegrationMarker::CommandFinished { exit_code }
            }
            "E" => IntegrationMarker::CommandContinuation,
            "F" => IntegrationMarker::RightPrompt,
            "G" => IntegrationMarker::CommandInvalid,
            "H" => IntegrationMarker::CommandCancelled,
            "P" => {
                // P格式: key=value
                if let Some((key, value)) = data.split_once('=') {
                    IntegrationMarker::Property {
                        key: key.to_string(),
                        value: value.to_string(),
                    }
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        Some(OscSequence::ShellIntegration {
            marker: si_marker,
            data: if data.is_empty() {
                None
            } else {
                Some(data.to_string())
            },
        })
    }

    /// 解析Windows Terminal序列
    fn parse_windows_terminal_sequence(&self, data: &str) -> Option<OscSequence> {
        // OSC 9;9;path 格式
        if data.starts_with("9;") {
            let path = data.strip_prefix("9;")?.to_string();
            Some(OscSequence::WindowsTerminalCwd { path })
        } else {
            None
        }
    }

    /// 解析CWD序列 (OSC 7)
    fn parse_cwd_sequence(&self, data: &str) -> Option<OscSequence> {
        // 尝试解析file://格式的URL
        if let Ok(url) = Url::parse(data) {
            if url.scheme() == "file" {
                let mut path = url.path().to_string();

                // 处理Windows路径格式
                #[cfg(windows)]
                {
                    if path.starts_with('/') && path.len() > 3 && path.chars().nth(2) == Some(':') {
                        path = path[1..].to_string();
                    }
                }

                return Some(OscSequence::CurrentWorkingDirectory { path });
            }
        }

        // 处理直接路径格式
        if !data.is_empty() {
            let path = if data.starts_with("file://") {
                let without_scheme = data.strip_prefix("file://").unwrap_or(data);
                if let Some(slash_pos) = without_scheme.find('/') {
                    without_scheme[slash_pos..].to_string()
                } else {
                    without_scheme.to_string()
                }
            } else {
                data.to_string()
            };

            // URL解码
            let decoded_path = percent_encoding::percent_decode_str(&path)
                .decode_utf8()
                .unwrap_or_else(|_| std::borrow::Cow::Borrowed(&path))
                .to_string();

            Some(OscSequence::CurrentWorkingDirectory { path: decoded_path })
        } else {
            None
        }
    }

    /// 移除文本中的OSC序列，返回清理后的文本
    pub fn strip_osc_sequences(&self, data: &str) -> String {
        let mut result = data.to_string();

        // 先移除明确匹配到的 Shell Integration / 窗口标题 序列
        result = self.si_regex.replace_all(&result, "").to_string();
        result = self.title_regex.replace_all(&result, "").to_string();
        // 再移除其他 OSC，但保留任何未被上面正则匹配到的 633（避免误删未识别变体）
        result = self
            .osc_regex
            .replace_all(&result, |caps: &Captures| {
                let number = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                if number == "633" {
                    // 保留 633（若之前未匹配到特定形式）
                    caps.get(0)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_else(String::new)
                } else {
                    String::new()
                }
            })
            .to_string();

        result
    }

    /// 移除非命令生命周期相关的 OSC 序列（仅保留 633），用于 sidecar 前置检测
    pub fn strip_preserve_shell_integration(&self, data: &str) -> String {
        let mut result = data.to_string();
        // 去掉常见非命令生命周期相关内容（标题等）
        result = self.title_regex.replace_all(&result, "").to_string();
        // 使用通用 OSC 清洗，但保留 633
        result = self
            .osc_regex
            .replace_all(&result, |caps: &Captures| {
                let number = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                if number == "633" {
                    caps.get(0)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_else(String::new)
                } else {
                    String::new()
                }
            })
            .to_string();
        result
    }

    /// 检测是否包含 Shell Integration（OSC 633）序列
    pub fn contains_shell_integration(&self, data: &str) -> bool {
        self.si_regex.is_match(data)
    }

    /// 提取命令执行相关的序列
    pub fn extract_command_sequences(&self, data: &str) -> Vec<OscSequence> {
        self.parse(data)
            .into_iter()
            .filter(|seq| matches!(seq, OscSequence::ShellIntegration { .. }))
            .collect()
    }

    /// 提取CWD相关的序列
    pub fn extract_cwd_sequences(&self, data: &str) -> Vec<OscSequence> {
        self.parse(data)
            .into_iter()
            .filter(|seq| {
                matches!(
                    seq,
                    OscSequence::CurrentWorkingDirectory { .. }
                        | OscSequence::WindowsTerminalCwd { .. }
                )
            })
            .collect()
    }
}

impl Default for OscParser {
    fn default() -> Self {
        Self::new().expect("Failed to create OSC parser")
    }
}

/// Shell Integration序列生成器
pub struct OscGenerator;

impl OscGenerator {
    /// 生成 Shell Integration（OSC 633） 序列
    pub fn shell_integration_sequence(marker: &IntegrationMarker, data: Option<&str>) -> String {
        let marker_char = match marker {
            IntegrationMarker::PromptStart => "A",
            IntegrationMarker::CommandStart => "B",
            IntegrationMarker::CommandExecuted => "C",
            IntegrationMarker::CommandFinished { exit_code } => {
                return if let Some(code) = exit_code {
                    format!("\x1b]633;D;{}\x07", code)
                } else {
                    "\x1b]633;D\x07".to_string()
                };
            }
            IntegrationMarker::CommandContinuation => "E",
            IntegrationMarker::RightPrompt => "F",
            IntegrationMarker::CommandInvalid => "G",
            IntegrationMarker::CommandCancelled => "H",
            IntegrationMarker::Property { key, value } => {
                return format!("\x1b]633;P;{}={}\x07", key, value);
            }
        };

        if let Some(data) = data {
            format!("\x1b]633;{};{}\x07", marker_char, data)
        } else {
            format!("\x1b]633;{}\x07", marker_char)
        }
    }

    /// 生成CWD序列 (OSC 7)
    pub fn cwd_sequence(path: &str) -> String {
        format!("\x1b]7;file://{}\x07", path)
    }

    /// 生成Windows Terminal CWD序列
    pub fn windows_terminal_cwd_sequence(path: &str) -> String {
        format!("\x1b]9;9;{}\x07", path)
    }

    /// 生成窗口标题序列
    pub fn window_title_sequence(title_type: WindowTitleType, title: &str) -> String {
        let type_num = match title_type {
            WindowTitleType::Both => 0,
            WindowTitleType::Icon => 1,
            WindowTitleType::Window => 2,
        };
        format!("\x1b]{};{}\x07", type_num, title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cwd_sequence() {
        let parser = OscParser::new().unwrap();

        let sequences = parser.parse("\x1b]7;file://hostname/home/user\x07");

        assert_eq!(sequences.len(), 1);
        match &sequences[0] {
            OscSequence::CurrentWorkingDirectory { path } => {
                assert_eq!(path, "/home/user");
            }
            _ => panic!("期望CWD序列"),
        }
    }

    #[test]
    fn test_parse_shell_integration_sequences() {
        let parser = OscParser::new().unwrap();

        let sequences = parser.parse("\x1b]633;A\x07\x1b]633;B\x07\x1b]633;C\x07\x1b]633;D;0\x07");

        assert_eq!(sequences.len(), 4);

        // 测试命令开始序列
        match &sequences[0] {
            OscSequence::ShellIntegration {
                marker: IntegrationMarker::PromptStart,
                ..
            } => {}
            _ => panic!("期望PromptStart序列"),
        }

        // 测试命令完成序列
        match &sequences[3] {
            OscSequence::ShellIntegration {
                marker: IntegrationMarker::CommandFinished { exit_code: Some(0) },
                ..
            } => {}
            _ => panic!("期望CommandFinished序列"),
        }
    }

    #[test]
    fn test_strip_osc_sequences() {
        let parser = OscParser::new().unwrap();

        let input = "Hello\x1b]7;/home/user\x07World\x1b]633;A\x07!";
        let output = parser.strip_osc_sequences(input);

        assert_eq!(output, "HelloWorld!");
    }

    #[test]
    fn test_contains_shell_integration() {
        let parser = OscParser::new().unwrap();

        assert!(parser.contains_shell_integration("\x1b]633;A\x07"));
        assert!(!parser.contains_shell_integration("\x1b]7;/home/user\x07"));
        assert!(!parser.contains_shell_integration("regular text"));
    }

    #[test]
    fn test_osc_generator() {
        // 测试 Shell Integration 633 序列生成
        let prompt_start =
            OscGenerator::shell_integration_sequence(&IntegrationMarker::PromptStart, None);
        assert_eq!(prompt_start, "\x1b]633;A\x07");

        let command_finished = OscGenerator::shell_integration_sequence(
            &IntegrationMarker::CommandFinished { exit_code: Some(0) },
            None,
        );
        assert_eq!(command_finished, "\x1b]633;D;0\x07");

        // 测试CWD序列生成
        let cwd = OscGenerator::cwd_sequence("/home/user");
        assert_eq!(cwd, "\x1b]7;file:///home/user\x07");
    }
}
