//! OSC序列解析器
//!
//! 解析终端输出中的OSC (Operating System Command) 控制序列
//! 支持完整的Shell Integration协议，包括命令生命周期、CWD同步等

use anyhow::Result;
use regex::Regex;
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

/// OSC序列类型 - 支持完整的Shell Integration
#[derive(Debug, Clone)]
pub enum OscSequence {
    /// OSC 7 - 当前工作目录
    CurrentWorkingDirectory { path: String },

    /// OSC 133 序列 - VS Code Shell Integration
    VsCodeShellIntegration {
        marker: VsCodeMarker,
        data: Option<String>,
    },

    /// OSC 1337 序列 - iTerm2 Shell Integration
    ITerm2ShellIntegration { key: String, value: Option<String> },

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

/// VS Code Shell Integration标记
#[derive(Debug, Clone, PartialEq)]
pub enum VsCodeMarker {
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
    /// VS Code Shell Integration 正则表达式
    vscode_regex: Regex,
    /// iTerm2 Shell Integration 正则表达式
    iterm2_regex: Regex,
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

        // VS Code Shell Integration (OSC 633)
        let vscode_regex = Regex::new(r"\x1b]633;([A-Z]);?([^\x07\x1b]*?)(?:\x07|\x1b\\)")?;

        // iTerm2 Shell Integration (OSC 1337)
        let iterm2_regex =
            Regex::new(r"\x1b]1337;([^=\x07\x1b]+)(?:=([^\x07\x1b]*))?(?:\x07|\x1b\\)")?;

        // 窗口标题 (OSC 0/1/2)
        let title_regex = Regex::new(r"\x1b]([012]);([^\x07\x1b]*?)(?:\x07|\x1b\\)")?;

        // CWD (OSC 7)
        let cwd_regex = Regex::new(r"\x1b]7;([^\x07\x1b]*?)(?:\x07|\x1b\\)")?;

        Ok(Self {
            osc_regex,
            vscode_regex,
            iterm2_regex,
            title_regex,
            cwd_regex,
        })
    }

    /// 解析文本中的OSC序列
    pub fn parse(&self, data: &str) -> Vec<OscSequence> {
        debug!("OSC Parser received data: {:?}", data);
        let mut sequences = Vec::new();

        // 解析VS Code Shell Integration序列 (OSC 633)
        for captures in self.vscode_regex.captures_iter(data) {
            if let Some(marker_match) = captures.get(1) {
                let marker_str = marker_match.as_str();
                let data_str = captures.get(2).map(|m| m.as_str()).unwrap_or("");

                if let Some(sequence) = self.parse_vscode_sequence(marker_str, data_str) {
                    debug!("解析到VS Code Shell Integration序列: {:?}", sequence);
                    sequences.push(sequence);
                }
            }
        }

        // 解析iTerm2 Shell Integration序列 (OSC 1337)
        for captures in self.iterm2_regex.captures_iter(data) {
            if let Some(key_match) = captures.get(1) {
                let key = key_match.as_str().to_string();
                let value = captures.get(2).map(|m| m.as_str().to_string());

                let sequence = OscSequence::ITerm2ShellIntegration { key, value };
                debug!("解析到iTerm2 Shell Integration序列: {:?}", sequence);
                sequences.push(sequence);
            }
        }

        // 解析CWD序列 (OSC 7)
        for captures in self.cwd_regex.captures_iter(data) {
            if let Some(data_match) = captures.get(1) {
                if let Some(sequence) = self.parse_cwd_sequence(data_match.as_str()) {
                    debug!("解析到CWD序列: {:?}", sequence);
                    sequences.push(sequence);
                }
            }
        }

        // 解析窗口标题序列 (OSC 0/1/2)
        for captures in self.title_regex.captures_iter(data) {
            if let (Some(type_match), Some(title_match)) = (captures.get(1), captures.get(2)) {
                let title_type = match type_match.as_str() {
                    "0" => WindowTitleType::Both,
                    "1" => WindowTitleType::Icon,
                    "2" => WindowTitleType::Window,
                    _ => continue,
                };
                let title = title_match.as_str().to_string();

                let sequence = OscSequence::WindowTitle { title_type, title };
                debug!("解析到窗口标题序列: {:?}", sequence);
                sequences.push(sequence);
            }
        }

        // 解析其他OSC序列
        for captures in self.osc_regex.captures_iter(data) {
            if let (Some(number_match), Some(data_match)) = (captures.get(1), captures.get(2)) {
                let number = number_match.as_str();
                let sequence_data = data_match.as_str();

                // 跳过已经处理过的序列
                if matches!(number, "0" | "1" | "2" | "7" | "633" | "1337") {
                    continue;
                }

                match self.parse_osc_sequence(number, sequence_data) {
                    Some(sequence) => {
                        debug!("解析到OSC序列: {:?}", sequence);
                        sequences.push(sequence);
                    }
                    None => {
                        // 记录未知序列用于调试
                        let sequence = OscSequence::Unknown {
                            number: number.to_string(),
                            data: sequence_data.to_string(),
                        };
                        debug!("未识别的OSC序列: {:?}", sequence);
                        sequences.push(sequence);
                    }
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

    /// 解析VS Code Shell Integration序列
    fn parse_vscode_sequence(&self, marker: &str, data: &str) -> Option<OscSequence> {
        let vscode_marker = match marker {
            "A" => VsCodeMarker::PromptStart,
            "B" => VsCodeMarker::CommandStart,
            "C" => VsCodeMarker::CommandExecuted,
            "D" => {
                // D可能包含退出码
                let exit_code = if data.is_empty() {
                    None
                } else {
                    data.parse::<i32>().ok()
                };
                VsCodeMarker::CommandFinished { exit_code }
            }
            "E" => VsCodeMarker::CommandContinuation,
            "F" => VsCodeMarker::RightPrompt,
            "G" => VsCodeMarker::CommandInvalid,
            "H" => VsCodeMarker::CommandCancelled,
            "P" => {
                // P格式: key=value
                if let Some((key, value)) = data.split_once('=') {
                    VsCodeMarker::Property {
                        key: key.to_string(),
                        value: value.to_string(),
                    }
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        Some(OscSequence::VsCodeShellIntegration {
            marker: vscode_marker,
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
                let path = url.path().to_string();
                return Some(OscSequence::CurrentWorkingDirectory { path });
            }
        }

        // 如果不是URL格式，直接作为路径处理
        if !data.is_empty() {
            Some(OscSequence::CurrentWorkingDirectory {
                path: data.to_string(),
            })
        } else {
            None
        }
    }

    /// 移除文本中的OSC序列，返回清理后的文本
    pub fn strip_osc_sequences(&self, data: &str) -> String {
        let mut result = data.to_string();

        // 移除所有类型的OSC序列
        result = self.vscode_regex.replace_all(&result, "").to_string();
        result = self.iterm2_regex.replace_all(&result, "").to_string();
        result = self.title_regex.replace_all(&result, "").to_string();
        result = self.osc_regex.replace_all(&result, "").to_string();

        result
    }

    /// 检测是否包含Shell Integration序列
    pub fn contains_shell_integration(&self, data: &str) -> bool {
        self.vscode_regex.is_match(data) || self.iterm2_regex.is_match(data)
    }

    /// 提取命令执行相关的序列
    pub fn extract_command_sequences(&self, data: &str) -> Vec<OscSequence> {
        self.parse(data)
            .into_iter()
            .filter(|seq| {
                matches!(
                    seq,
                    OscSequence::VsCodeShellIntegration { .. }
                        | OscSequence::ITerm2ShellIntegration { .. }
                )
            })
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
    /// 生成VS Code Shell Integration序列
    pub fn vscode_sequence(marker: &VsCodeMarker, data: Option<&str>) -> String {
        let marker_char = match marker {
            VsCodeMarker::PromptStart => "A",
            VsCodeMarker::CommandStart => "B",
            VsCodeMarker::CommandExecuted => "C",
            VsCodeMarker::CommandFinished { exit_code } => {
                return if let Some(code) = exit_code {
                    format!("\x1b]633;D;{}\x07", code)
                } else {
                    "\x1b]633;D\x07".to_string()
                };
            }
            VsCodeMarker::CommandContinuation => "E",
            VsCodeMarker::RightPrompt => "F",
            VsCodeMarker::CommandInvalid => "G",
            VsCodeMarker::CommandCancelled => "H",
            VsCodeMarker::Property { key, value } => {
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

    /// 生成iTerm2 Shell Integration序列
    pub fn iterm2_sequence(key: &str, value: Option<&str>) -> String {
        if let Some(value) = value {
            format!("\x1b]1337;{}={}\x07", key, value)
        } else {
            format!("\x1b]1337;{}\x07", key)
        }
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
    fn test_parse_vscode_sequences() {
        let parser = OscParser::new().unwrap();

        let sequences = parser.parse("\x1b]633;A\x07\x1b]633;B\x07\x1b]633;C\x07\x1b]633;D;0\x07");

        assert_eq!(sequences.len(), 4);

        // 测试命令开始序列
        match &sequences[0] {
            OscSequence::VsCodeShellIntegration {
                marker: VsCodeMarker::PromptStart,
                ..
            } => {}
            _ => panic!("期望PromptStart序列"),
        }

        // 测试命令完成序列
        match &sequences[3] {
            OscSequence::VsCodeShellIntegration {
                marker: VsCodeMarker::CommandFinished { exit_code: Some(0) },
                ..
            } => {}
            _ => panic!("期望CommandFinished序列"),
        }
    }

    #[test]
    fn test_parse_iterm2_sequences() {
        let parser = OscParser::new().unwrap();

        let sequences = parser
            .parse("\x1b]1337;RemoteHost=user@hostname\x07\x1b]1337;CurrentDir=/home/user\x07");

        assert_eq!(sequences.len(), 2);

        match &sequences[0] {
            OscSequence::ITerm2ShellIntegration { key, value } => {
                assert_eq!(key, "RemoteHost");
                assert_eq!(value.as_ref().unwrap(), "user@hostname");
            }
            _ => panic!("期望iTerm2序列"),
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
        assert!(parser.contains_shell_integration("\x1b]1337;CurrentDir=/home\x07"));
        assert!(!parser.contains_shell_integration("\x1b]7;/home/user\x07"));
        assert!(!parser.contains_shell_integration("regular text"));
    }

    #[test]
    fn test_osc_generator() {
        // 测试VS Code序列生成
        let prompt_start = OscGenerator::vscode_sequence(&VsCodeMarker::PromptStart, None);
        assert_eq!(prompt_start, "\x1b]633;A\x07");

        let command_finished = OscGenerator::vscode_sequence(
            &VsCodeMarker::CommandFinished { exit_code: Some(0) },
            None,
        );
        assert_eq!(command_finished, "\x1b]633;D;0\x07");

        // 测试CWD序列生成
        let cwd = OscGenerator::cwd_sequence("/home/user");
        assert_eq!(cwd, "\x1b]7;file:///home/user\x07");

        // 测试iTerm2序列生成
        let iterm2 = OscGenerator::iterm2_sequence("CurrentDir", Some("/home/user"));
        assert_eq!(iterm2, "\x1b]1337;CurrentDir=/home/user\x07");
    }
}
