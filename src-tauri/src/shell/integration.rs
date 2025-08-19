//! Shell Integration - 完整的Shell集成管理系统
//!
//! 支持命令生命周期跟踪、CWD同步、窗口标题更新等功能

use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use tracing::{debug, info};

use super::osc_parser::{OscParser, OscSequence, VsCodeMarker, CommandStatus, ShellIntegrationState};
use super::script_generator::{ShellType, ShellScriptGenerator, ShellIntegrationConfig};
use crate::mux::PaneId;

/// 命令执行信息
#[derive(Debug, Clone)]
pub struct CommandInfo {
    /// 命令ID（递增）
    pub id: u64,
    /// 命令开始时间
    pub start_time: Instant,
    /// 命令结束时间
    pub end_time: Option<Instant>,
    /// 退出码
    pub exit_code: Option<i32>,
    /// 命令状态
    pub status: CommandStatus,
    /// 命令行文本（如果可用）
    pub command_line: Option<String>,
    /// 执行目录
    pub working_directory: Option<String>,
}

impl CommandInfo {
    fn new(id: u64) -> Self {
        Self {
            id,
            start_time: Instant::now(),
            end_time: None,
            exit_code: None,
            status: CommandStatus::Ready,
            command_line: None,
            working_directory: None,
        }
    }
    
    /// 获取命令执行时长
    pub fn duration(&self) -> Duration {
        match self.end_time {
            Some(end) => end.duration_since(self.start_time),
            None => Instant::now().duration_since(self.start_time),
        }
    }
    
    /// 检查命令是否完成
    pub fn is_finished(&self) -> bool {
        matches!(self.status, CommandStatus::Finished { .. })
    }
}

/// 面板Shell状态
#[derive(Debug, Clone)]
pub struct PaneShellState {
    /// Shell Integration状态
    pub integration_state: ShellIntegrationState,
    /// Shell类型
    pub shell_type: Option<ShellType>,
    /// 当前工作目录
    pub current_working_directory: Option<String>,
    /// 当前命令信息
    pub current_command: Option<CommandInfo>,
    /// 历史命令（最近20个）
    pub command_history: Vec<CommandInfo>,
    /// 下一个命令ID
    pub next_command_id: u64,
    /// 窗口标题
    pub window_title: Option<String>,
    /// 最后活动时间
    pub last_activity: SystemTime,
}

impl Default for PaneShellState {
    fn default() -> Self {
        Self {
            integration_state: ShellIntegrationState::Disabled,
            shell_type: None,
            current_working_directory: None,
            current_command: None,
            command_history: Vec::new(),
            next_command_id: 1,
            window_title: None,
            last_activity: SystemTime::now(),
        }
    }
}

impl PaneShellState {
    /// 开始新命令
    fn start_command(&mut self) -> u64 {
        let command_id = self.next_command_id;
        self.next_command_id += 1;
        
        let mut command = CommandInfo::new(command_id);
        command.status = CommandStatus::Running;
        command.working_directory = self.current_working_directory.clone();
        
        self.current_command = Some(command);
        self.last_activity = SystemTime::now();
        
        command_id
    }
    
    /// 结束当前命令
    fn finish_command(&mut self, exit_code: Option<i32>) {
        if let Some(mut command) = self.current_command.take() {
            command.end_time = Some(Instant::now());
            command.exit_code = exit_code;
            command.status = CommandStatus::Finished { exit_code };
            
            // 添加到历史记录，保持最近20个
            self.command_history.push(command);
            if self.command_history.len() > 20 {
                self.command_history.remove(0);
            }
        }
        
        self.last_activity = SystemTime::now();
    }
    
    /// 更新CWD
    fn update_cwd(&mut self, new_cwd: String) {
        self.current_working_directory = Some(new_cwd);
        self.last_activity = SystemTime::now();
    }
    
    /// 获取当前命令执行时长
    pub fn current_command_duration(&self) -> Option<Duration> {
        self.current_command.as_ref().map(|cmd| cmd.duration())
    }
}

/// Shell Integration管理器 - 支持完整的Shell集成功能
pub struct ShellIntegrationManager {
    /// 面板状态映射
    pane_states: Arc<Mutex<HashMap<PaneId, PaneShellState>>>,
    /// OSC序列解析器
    parser: OscParser,
    /// 脚本生成器
    script_generator: ShellScriptGenerator,
    /// CWD变化回调
    cwd_callbacks: Arc<Mutex<Vec<Box<dyn Fn(PaneId, &str) + Send + Sync>>>>,
    /// 命令状态变化回调
    command_callbacks: Arc<Mutex<Vec<Box<dyn Fn(PaneId, &CommandInfo) + Send + Sync>>>>,
    /// 窗口标题变化回调
    title_callbacks: Arc<Mutex<Vec<Box<dyn Fn(PaneId, &str) + Send + Sync>>>>,
}

impl ShellIntegrationManager {
    /// 创建新的Shell Integration管理器
    pub fn new() -> Result<Self> {
        let config = ShellIntegrationConfig::default();
        Self::new_with_config(config)
    }
    
    /// 使用指定配置创建Shell Integration管理器
    pub fn new_with_config(config: ShellIntegrationConfig) -> Result<Self> {
        Ok(Self {
            pane_states: Arc::new(Mutex::new(HashMap::new())),
            parser: OscParser::new()?,
            script_generator: ShellScriptGenerator::new(config),
            cwd_callbacks: Arc::new(Mutex::new(Vec::new())),
            command_callbacks: Arc::new(Mutex::new(Vec::new())),
            title_callbacks: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// 注册CWD变化回调
    pub fn register_cwd_callback<F>(&self, callback: F)
    where
        F: Fn(PaneId, &str) + Send + Sync + 'static,
    {
        if let Ok(mut callbacks) = self.cwd_callbacks.lock() {
            callbacks.push(Box::new(callback));
            debug!("注册CWD变化回调");
        }
    }
    
    /// 注册命令状态变化回调
    pub fn register_command_callback<F>(&self, callback: F)
    where
        F: Fn(PaneId, &CommandInfo) + Send + Sync + 'static,
    {
        if let Ok(mut callbacks) = self.command_callbacks.lock() {
            callbacks.push(Box::new(callback));
            debug!("注册命令状态变化回调");
        }
    }
    
    /// 注册窗口标题变化回调
    pub fn register_title_callback<F>(&self, callback: F)
    where
        F: Fn(PaneId, &str) + Send + Sync + 'static,
    {
        if let Ok(mut callbacks) = self.title_callbacks.lock() {
            callbacks.push(Box::new(callback));
            debug!("注册窗口标题变化回调");
        }
    }

    /// 处理终端输出，解析OSC序列并更新状态
    pub fn process_output(&self, pane_id: PaneId, data: &str) {
        debug!(
            "ShellIntegrationManager processing for pane {}: {:?}",
            pane_id,
            data
        );
        let sequences = self.parser.parse(data);

        if !sequences.is_empty() {
            debug!(
                "Pane {} parsed {} OSC sequences: {:?}",
                pane_id,
                sequences.len(),
                &sequences
            );
        }

        for sequence in sequences {
            match sequence {
                OscSequence::CurrentWorkingDirectory { path } => {
                    self.update_cwd(pane_id, path);
                }
                OscSequence::WindowsTerminalCwd { path } => {
                    self.update_cwd(pane_id, path);
                }
                OscSequence::VsCodeShellIntegration { marker, data } => {
                    self.handle_vscode_integration(pane_id, marker, data);
                }
                OscSequence::ITerm2ShellIntegration { key, value } => {
                    self.handle_iterm2_integration(pane_id, key, value);
                }
                OscSequence::WindowTitle { title, .. } => {
                    self.update_window_title(pane_id, title);
                }
                OscSequence::Unknown { number, data } => {
                    debug!("未知OSC序列: OSC {};{}", number, data);
                }
            }
        }
    }

    /// 处理VS Code Shell Integration序列
    fn handle_vscode_integration(&self, pane_id: PaneId, marker: VsCodeMarker, _data: Option<String>) {
        if let Ok(mut states) = self.pane_states.lock() {
            let state = states.entry(pane_id).or_default();
            state.integration_state = ShellIntegrationState::Enabled;
            
            match marker {
                VsCodeMarker::PromptStart => {
                    // 准备接收新命令
                    state.current_command = Some(CommandInfo::new(state.next_command_id));
                    debug!("面板 {} 提示符开始", pane_id);
                }
                VsCodeMarker::CommandStart => {
                    // 用户开始输入命令
                    debug!("面板 {} 命令开始", pane_id);
                }
                VsCodeMarker::CommandExecuted => {
                    // 命令开始执行
                    let command_id = state.start_command();
                    debug!("面板 {} 命令执行开始 (ID: {})", pane_id, command_id);
                    
                    if let Some(command) = &state.current_command {
                        self.trigger_command_callbacks(pane_id, command);
                    }
                }
                VsCodeMarker::CommandFinished { exit_code } => {
                    // 命令执行完成
                    debug!("面板 {} 命令完成，退出码: {:?}", pane_id, exit_code);
                    state.finish_command(exit_code);
                    
                    if let Some(last_command) = state.command_history.last() {
                        self.trigger_command_callbacks(pane_id, last_command);
                    }
                }
                VsCodeMarker::CommandCancelled => {
                    // 命令被取消
                    debug!("面板 {} 命令被取消", pane_id);
                    state.finish_command(Some(130)); // SIGINT退出码
                }
                _ => {
                    debug!("面板 {} 接收到VS Code标记: {:?}", pane_id, marker);
                }
            }
        }
    }

    /// 处理iTerm2 Shell Integration序列
    fn handle_iterm2_integration(&self, pane_id: PaneId, key: String, value: Option<String>) {
        if let Ok(mut states) = self.pane_states.lock() {
            let state = states.entry(pane_id).or_default();
            state.integration_state = ShellIntegrationState::Enabled;
            
            match key.as_str() {
                "CurrentDir" => {
                    if let Some(path) = value {
                        state.update_cwd(path.clone());
                        self.trigger_cwd_callbacks(pane_id, &path);
                        debug!("面板 {} iTerm2 CWD: {}", pane_id, path);
                    }
                }
                "ShellIntegrationVersion" => {
                    debug!("面板 {} iTerm2集成版本: {:?}", pane_id, value);
                }
                "RemoteHost" => {
                    debug!("面板 {} 远程主机: {:?}", pane_id, value);
                }
                _ => {
                    debug!("面板 {} iTerm2序列: {}={:?}", pane_id, key, value);
                }
            }
        }
    }

    /// 更新面板的CWD并触发回调
    fn update_cwd(&self, pane_id: PaneId, new_cwd: String) {
        debug!(
            "Attempting to update CWD for pane {} to: {}",
            pane_id,
            &new_cwd
        );
        let old_cwd = if let Ok(mut states) = self.pane_states.lock() {
            let state = states.entry(pane_id).or_default();
            let old = state.current_working_directory.clone();
            state.update_cwd(new_cwd.clone());
            old
        } else {
            return;
        };

        // 只有CWD真的变化了才打日志和触发回调
        if old_cwd.as_ref() != Some(&new_cwd) {
            info!("📁 面板 {} CWD: {}", pane_id, new_cwd);
            self.trigger_cwd_callbacks(pane_id, &new_cwd);
        }
    }

    /// 更新窗口标题
    fn update_window_title(&self, pane_id: PaneId, title: String) {
        debug!(
            "Attempting to update window title for pane {} to: {}",
            pane_id,
            &title
        );
        if let Ok(mut states) = self.pane_states.lock() {
            let state = states.entry(pane_id).or_default();
            let old_title = state.window_title.clone();
            state.window_title = Some(title.clone());
            state.last_activity = SystemTime::now();
            
            if old_title.as_ref() != Some(&title) {
                info!("📋 面板 {} 标题: {}", pane_id, title);
                self.trigger_title_callbacks(pane_id, &title);
            }
        }
    }

    /// 触发CWD变化回调
    fn trigger_cwd_callbacks(&self, pane_id: PaneId, new_cwd: &str) {
        if let Ok(callbacks) = self.cwd_callbacks.lock() {
            for callback in callbacks.iter() {
                callback(pane_id, new_cwd);
            }
        }
    }

    /// 触发命令状态变化回调
    fn trigger_command_callbacks(&self, pane_id: PaneId, command: &CommandInfo) {
        if let Ok(callbacks) = self.command_callbacks.lock() {
            for callback in callbacks.iter() {
                callback(pane_id, command);
            }
        }
    }

    /// 触发窗口标题变化回调
    fn trigger_title_callbacks(&self, pane_id: PaneId, title: &str) {
        if let Ok(callbacks) = self.title_callbacks.lock() {
            for callback in callbacks.iter() {
                callback(pane_id, title);
            }
        }
    }

    /// 获取面板的当前工作目录
    pub fn get_current_working_directory(&self, pane_id: PaneId) -> Option<String> {
        self.pane_states.lock().ok()?
            .get(&pane_id)?
            .current_working_directory.clone()
    }

    /// 手动更新面板的当前工作目录
    pub fn update_current_working_directory(&self, pane_id: PaneId, cwd: String) {
        self.update_cwd(pane_id, cwd);
    }

    /// 检查面板是否有Shell Integration状态
    pub fn get_pane_state(&self, pane_id: PaneId) -> Option<()> {
        self.pane_states.lock().ok()?.get(&pane_id).map(|_| ())
    }
    
    /// 获取面板的完整状态
    pub fn get_pane_shell_state(&self, pane_id: PaneId) -> Option<PaneShellState> {
        self.pane_states.lock().ok()?.get(&pane_id).cloned()
    }

    /// 设置面板的Shell类型
    pub fn set_pane_shell_type(&self, pane_id: PaneId, shell_type: ShellType) {
        if let Ok(mut states) = self.pane_states.lock() {
            let state = states.entry(pane_id).or_default();
            state.shell_type = Some(shell_type.clone());
            debug!("面板 {} Shell类型: {}", pane_id, shell_type.display_name());
        }
    }

    /// 生成Shell集成脚本
    pub fn generate_shell_script(&self, shell_type: &ShellType) -> Result<String> {
        self.script_generator.generate_integration_script(shell_type)
    }

    /// 生成Shell环境变量
    pub fn generate_shell_env_vars(&self, shell_type: &ShellType) -> HashMap<String, String> {
        self.script_generator.generate_env_vars(shell_type)
    }

    /// 启用Shell Integration
    pub fn enable_integration(&self, pane_id: PaneId) {
        if let Ok(mut states) = self.pane_states.lock() {
            let state = states.entry(pane_id).or_default();
            state.integration_state = ShellIntegrationState::Enabled;
            info!("面板 {} Shell Integration已启用", pane_id);
        }
    }

    /// 禁用Shell Integration
    pub fn disable_integration(&self, pane_id: PaneId) {
        if let Ok(mut states) = self.pane_states.lock() {
            let state = states.entry(pane_id).or_default();
            state.integration_state = ShellIntegrationState::Disabled;
            info!("面板 {} Shell Integration已禁用", pane_id);
        }
    }

    /// 检查面板是否启用了Shell Integration
    pub fn is_integration_enabled(&self, pane_id: PaneId) -> bool {
        if let Ok(states) = self.pane_states.lock() {
            if let Some(state) = states.get(&pane_id) {
                return state.integration_state == ShellIntegrationState::Enabled;
            }
        }
        false
    }

    /// 获取面板的当前命令信息
    pub fn get_current_command(&self, pane_id: PaneId) -> Option<CommandInfo> {
        self.pane_states.lock().ok()?
            .get(&pane_id)?
            .current_command.clone()
    }

    /// 获取面板的命令历史
    pub fn get_command_history(&self, pane_id: PaneId) -> Vec<CommandInfo> {
        if let Ok(states) = self.pane_states.lock() {
            if let Some(state) = states.get(&pane_id) {
                return state.command_history.clone();
            }
        }
        Vec::new()
    }

    /// 清理面板状态
    pub fn cleanup_pane(&self, pane_id: PaneId) {
        if let Ok(mut states) = self.pane_states.lock() {
            states.remove(&pane_id);
            debug!("清理面板 {} 的Shell Integration状态", pane_id);
        }
    }
}

impl Default for ShellIntegrationManager {
    fn default() -> Self {
        Self::new().expect("Failed to create Shell Integration manager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_integration_manager() {
        let manager = ShellIntegrationManager::new().unwrap();
        let pane_id = PaneId::new(1);

        // 测试CWD更新
        manager.update_current_working_directory(pane_id, "/home/user".to_string());
        assert_eq!(
            manager.get_current_working_directory(pane_id),
            Some("/home/user".to_string())
        );

        // 测试Shell类型设置
        manager.set_pane_shell_type(pane_id, ShellType::Bash);
        let state = manager.get_pane_shell_state(pane_id).unwrap();
        assert_eq!(state.shell_type, Some(ShellType::Bash));
    }

    #[test]
    fn test_command_lifecycle() {
        let manager = ShellIntegrationManager::new().unwrap();
        let pane_id = PaneId::new(1);

        // 模拟VS Code命令序列
        manager.process_output(pane_id, "\x1b]633;A\x07"); // 提示符开始
        manager.process_output(pane_id, "\x1b]633;B\x07"); // 命令开始
        manager.process_output(pane_id, "\x1b]633;C\x07"); // 命令执行
        manager.process_output(pane_id, "\x1b]633;D;0\x07"); // 命令完成

        let state = manager.get_pane_shell_state(pane_id).unwrap();
        assert!(state.integration_state == ShellIntegrationState::Enabled);
        assert_eq!(state.command_history.len(), 1);
        assert_eq!(state.command_history[0].exit_code, Some(0));
    }

    #[test]
    fn test_script_generation() {
        let manager = ShellIntegrationManager::new().unwrap();
        
        let bash_script = manager.generate_shell_script(&ShellType::Bash).unwrap();
        assert!(bash_script.contains("ORBITX_SHELL_INTEGRATION"));
        
        let env_vars = manager.generate_shell_env_vars(&ShellType::Bash);
        assert!(env_vars.contains_key("ORBITX_SHELL_INTEGRATION"));
    }
}