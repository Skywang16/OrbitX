// 终端上下文相关类型定义

use crate::mux::PaneId;
use crate::terminal::error::{TerminalValidationError, TerminalValidationResult};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// 终端 Channel 消息类型
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum TerminalChannelMessage {
    Data { pane_id: u32, data: Vec<u8> },
    Error { pane_id: u32, error: String },
    Close { pane_id: u32 },
}

/// 终端上下文事件类型
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TerminalContextEvent {
    /// 活跃面板变化
    ActivePaneChanged {
        old_pane_id: Option<PaneId>,
        new_pane_id: Option<PaneId>,
    },
    /// 面板上下文更新
    PaneContextUpdated {
        pane_id: PaneId,
        context: TerminalContext,
    },
    /// 面板CWD变化
    PaneCwdChanged {
        pane_id: PaneId,
        old_cwd: Option<String>,
        new_cwd: String,
    },
    /// 面板Shell集成状态变化
    PaneShellIntegrationChanged { pane_id: PaneId, enabled: bool },
}

/// 终端上下文数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalContext {
    pub pane_id: PaneId,
    pub current_working_directory: Option<String>,
    pub shell_type: Option<ShellType>,
    pub shell_integration_enabled: bool,
    pub current_command: Option<CommandInfo>,
    pub command_history: Vec<CommandInfo>,
    pub window_title: Option<String>,
    pub last_activity: SystemTime,
    pub is_active: bool,
}

/// Shell类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    Other(String),
}

impl ShellType {
    /// 从字符串解析Shell类型
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "bash" => ShellType::Bash,
            "zsh" => ShellType::Zsh,
            "fish" => ShellType::Fish,
            _ => ShellType::Other(s.to_string()),
        }
    }

    /// 获取Shell的显示名称
    pub fn display_name(&self) -> &str {
        match self {
            ShellType::Bash => "Bash",
            ShellType::Zsh => "Zsh",
            ShellType::Fish => "Fish",
            ShellType::Other(name) => name,
        }
    }

    /// 检查是否支持Shell集成
    pub fn supports_integration(&self) -> bool {
        matches!(self, ShellType::Bash | ShellType::Zsh | ShellType::Fish)
    }

    /// 获取Shell的默认提示符
    pub fn default_prompt(&self) -> &str {
        match self {
            ShellType::Bash | ShellType::Zsh => "$ ",
            ShellType::Fish => "❯ ",
            ShellType::Other(_) => "$ ",
        }
    }
}

impl Default for ShellType {
    fn default() -> Self {
        ShellType::Bash
    }
}

impl std::fmt::Display for ShellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// 命令信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandInfo {
    pub command: String,
    pub args: Vec<String>,
    pub start_time: SystemTime,
    pub end_time: Option<SystemTime>,
    pub exit_code: Option<i32>,
    pub working_directory: Option<String>,
}

impl CommandInfo {
    /// 创建新的命令信息
    pub fn new(command: String, args: Vec<String>, working_directory: Option<String>) -> Self {
        Self {
            command,
            args,
            start_time: SystemTime::now(),
            end_time: None,
            exit_code: None,
            working_directory,
        }
    }

    /// 标记命令完成
    pub fn complete(&mut self, exit_code: i32) {
        self.end_time = Some(SystemTime::now());
        self.exit_code = Some(exit_code);
    }

    /// 验证命令信息的有效性
    pub fn validate(&self) -> Result<(), String> {
        if self.command.trim().is_empty() {
            return Err("Command cannot be empty".to_string());
        }

        // 验证时间逻辑
        if let Some(end_time) = self.end_time {
            if end_time < self.start_time {
                return Err("End time cannot be earlier than start time".to_string());
            }
        }

        // 验证退出码逻辑
        if self.end_time.is_none() && self.exit_code.is_some() {
            return Err("Unfinished command should not have exit code".to_string());
        }

        Ok(())
    }

    /// 检查命令是否已完成
    pub fn is_completed(&self) -> bool {
        self.end_time.is_some()
    }

    /// 检查命令是否成功执行
    pub fn is_successful(&self) -> bool {
        self.exit_code.map_or(false, |code| code == 0)
    }

    /// 获取命令执行时长
    pub fn duration(&self) -> Option<std::time::Duration> {
        if let Some(end_time) = self.end_time {
            end_time.duration_since(self.start_time).ok()
        } else {
            None
        }
    }
}

impl TerminalContext {
    /// 创建新的终端上下文
    pub fn new(pane_id: PaneId) -> Self {
        Self {
            pane_id,
            current_working_directory: None,
            shell_type: None,
            shell_integration_enabled: false,
            current_command: None,
            command_history: Vec::new(),
            window_title: None,
            last_activity: SystemTime::now(),
            is_active: false,
        }
    }

    /// 创建带有默认值的终端上下文（用于错误回退）
    pub fn with_defaults(pane_id: PaneId) -> Self {
        Self {
            pane_id,
            current_working_directory: Some("~".to_string()),
            shell_type: Some(ShellType::Bash),
            shell_integration_enabled: false,
            current_command: None,
            command_history: Vec::new(),
            window_title: None,
            last_activity: SystemTime::now(),
            is_active: false,
        }
    }

    /// 验证终端上下文的完整性
    pub fn validate(&self) -> TerminalValidationResult<()> {
        // 验证面板ID是否有效
        if self.pane_id.as_u32() == 0 {
            return Err(TerminalValidationError::InvalidPaneId);
        }

        // 验证命令历史记录的完整性
        for (index, command) in self.command_history.iter().enumerate() {
            if let Err(e) = command.validate() {
                return Err(TerminalValidationError::InvalidHistoryEntry { index, reason: e });
            }
        }

        // 验证当前命令的完整性
        if let Some(ref command) = self.current_command {
            if let Err(e) = command.validate() {
                return Err(TerminalValidationError::InvalidCurrentCommand { reason: e });
            }
        }

        Ok(())
    }

    /// 检查上下文是否包含完整的终端状态信息
    pub fn is_complete(&self) -> bool {
        self.current_working_directory.is_some() && self.shell_type.is_some()
    }

    /// 获取有效的工作目录，如果没有则返回默认值
    pub fn get_cwd_or_default(&self) -> String {
        self.current_working_directory
            .clone()
            .unwrap_or_else(|| "~".to_string())
    }

    /// 获取Shell类型，如果没有则返回默认值
    pub fn get_shell_type_or_default(&self) -> ShellType {
        self.shell_type.clone().unwrap_or(ShellType::Bash)
    }

    /// 更新活跃状态
    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
        if active {
            self.last_activity = SystemTime::now();
        }
    }

    /// 更新工作目录
    pub fn update_cwd(&mut self, cwd: String) {
        self.current_working_directory = Some(cwd);
        self.last_activity = SystemTime::now();
    }

    /// 更新Shell类型
    pub fn update_shell_type(&mut self, shell_type: ShellType) {
        self.shell_type = Some(shell_type);
        self.last_activity = SystemTime::now();
    }

    /// 设置Shell集成状态
    pub fn set_shell_integration(&mut self, enabled: bool) {
        self.shell_integration_enabled = enabled;
        self.last_activity = SystemTime::now();
    }

    /// 添加命令到历史
    pub fn add_command(&mut self, command: CommandInfo) {
        self.command_history.push(command);
        self.last_activity = SystemTime::now();

        // 限制历史记录数量
        if self.command_history.len() > 100 {
            self.command_history.remove(0);
        }
    }

    /// 设置当前命令
    pub fn set_current_command(&mut self, command: Option<CommandInfo>) {
        self.current_command = command;
        self.last_activity = SystemTime::now();
    }

    /// 更新窗口标题
    pub fn update_window_title(&mut self, title: String) {
        self.window_title = Some(title);
        self.last_activity = SystemTime::now();
    }
}

/// 缓存的上下文信息
/// 上下文查询选项
#[derive(Debug, Clone, Default)]
pub struct ContextQueryOptions {
    /// 是否使用缓存
    pub use_cache: bool,
    /// 查询超时时间
    pub timeout: Option<std::time::Duration>,
    /// 是否允许回退到默认值
    pub allow_fallback: bool,
    /// 是否包含命令历史
    pub include_history: bool,
    /// 最大历史记录数量
    pub max_history_count: Option<usize>,
}

impl ContextQueryOptions {
    /// 创建快速查询选项（使用缓存，允许回退）
    pub fn fast() -> Self {
        Self {
            use_cache: true,
            timeout: Some(std::time::Duration::from_millis(10)),
            allow_fallback: true,
            include_history: false,
            max_history_count: None,
        }
    }

    /// 创建完整查询选项（包含所有信息）
    pub fn complete() -> Self {
        Self {
            use_cache: false,
            timeout: Some(std::time::Duration::from_millis(100)),
            allow_fallback: true,
            include_history: true,
            max_history_count: Some(50),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mux::PaneId;

    #[test]
    fn test_terminal_context_creation() {
        let pane_id = PaneId::new(1);
        let context = TerminalContext::new(pane_id);

        assert_eq!(context.pane_id, pane_id);
        assert_eq!(context.current_working_directory, None);
        assert_eq!(context.shell_type, None);
        assert!(!context.shell_integration_enabled);
        assert!(!context.is_active);
    }

    #[test]
    fn test_terminal_context_with_defaults() {
        let pane_id = PaneId::new(1);
        let context = TerminalContext::with_defaults(pane_id);

        assert_eq!(context.pane_id, pane_id);
        assert_eq!(context.current_working_directory, Some("~".to_string()));
        assert_eq!(context.shell_type, Some(ShellType::Bash));
        assert!(!context.shell_integration_enabled);
        assert!(!context.is_active);
    }

    #[test]
    fn test_terminal_context_validation() {
        let pane_id = PaneId::new(1);
        let context = TerminalContext::new(pane_id);

        // Valid context should pass validation
        assert!(context.validate().is_ok());

        // Invalid pane ID should fail validation
        let invalid_context = TerminalContext::new(PaneId::new(0));
        assert!(invalid_context.validate().is_err());
    }

    #[test]
    fn test_command_info_validation() {
        let mut command = CommandInfo::new(
            "ls".to_string(),
            vec!["-la".to_string()],
            Some("/home/user".to_string()),
        );

        // Valid command should pass validation
        assert!(command.validate().is_ok());

        // Empty command should fail validation
        let empty_command = CommandInfo::new("".to_string(), vec![], None);
        assert!(empty_command.validate().is_err());

        // Complete the command
        command.complete(0);
        assert!(command.is_completed());
        assert!(command.is_successful());
    }

    #[test]
    fn test_shell_type_parsing() {
        assert_eq!(ShellType::from_str("bash"), ShellType::Bash);
        assert_eq!(ShellType::from_str("zsh"), ShellType::Zsh);
        assert_eq!(ShellType::from_str("fish"), ShellType::Fish);
        assert_eq!(
            ShellType::from_str("powershell"),
            ShellType::Other("powershell".to_string())
        );
        assert_eq!(
            ShellType::from_str("cmd"),
            ShellType::Other("cmd".to_string())
        );
        assert_eq!(
            ShellType::from_str("unknown"),
            ShellType::Other("unknown".to_string())
        );
    }

    #[test]
    fn test_context_query_options() {
        let fast_options = ContextQueryOptions::fast();
        assert!(fast_options.use_cache);
        assert!(fast_options.allow_fallback);
        assert!(!fast_options.include_history);

        let complete_options = ContextQueryOptions::complete();
        assert!(!complete_options.use_cache);
        assert!(complete_options.allow_fallback);
        assert!(complete_options.include_history);
    }
}
