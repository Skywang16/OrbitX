//! Shell Integration - 完整的Shell集成管理系统
//!
//! 支持命令生命周期跟踪、CWD同步、窗口标题更新等功能

use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant, SystemTime};

use super::osc_parser::{
    CommandStatus, IntegrationMarker, OscParser, OscSequence, ShellIntegrationState,
};
use super::script_generator::{ShellIntegrationConfig, ShellScriptGenerator, ShellType};
use crate::mux::PaneId;

/// 命令执行信息
#[derive(Debug, Clone)]
pub struct CommandInfo {
    /// 命令ID（递增）
    pub id: u64,
    /// 命令开始时间
    pub start_time: Instant,
    /// 命令开始墙钟时间
    pub start_time_wallclock: SystemTime,
    /// 命令结束时间
    pub end_time: Option<Instant>,
    /// 命令结束墙钟时间
    pub end_time_wallclock: Option<SystemTime>,
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
            start_time_wallclock: SystemTime::now(),
            end_time: None,
            end_time_wallclock: None,
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
            command.end_time_wallclock = Some(SystemTime::now());
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

// 前向声明，避免循环依赖
pub trait ContextServiceIntegration: Send + Sync {
    fn invalidate_cache(&self, pane_id: PaneId);
    fn send_cwd_changed_event(&self, pane_id: PaneId, old_cwd: Option<String>, new_cwd: String);
    fn send_shell_integration_changed_event(&self, pane_id: PaneId, enabled: bool);
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
    /// 上下文服务集成（弱引用避免循环依赖）
    context_service: Arc<Mutex<Option<Weak<dyn ContextServiceIntegration>>>>,
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
            context_service: Arc::new(Mutex::new(None)),
        })
    }

    /// 设置上下文服务集成
    ///
    /// # Arguments
    /// * `context_service` - 上下文服务的弱引用
    pub fn set_context_service_integration(
        &self,
        context_service: Weak<dyn ContextServiceIntegration>,
    ) {
        if let Ok(mut service) = self.context_service.lock() {
            *service = Some(context_service);
            tracing::debug!("上下文服务集成已设置");
        }
    }

    /// 移除上下文服务集成
    pub fn remove_context_service_integration(&self) {
        if let Ok(mut service) = self.context_service.lock() {
            *service = None;
            tracing::debug!("上下文服务集成已移除");
        }
    }

    /// 注册CWD变化回调
    pub fn register_cwd_callback<F>(&self, callback: F)
    where
        F: Fn(PaneId, &str) + Send + Sync + 'static,
    {
        if let Ok(mut callbacks) = self.cwd_callbacks.lock() {
            callbacks.push(Box::new(callback));
        }
    }

    /// 注册命令状态变化回调
    pub fn register_command_callback<F>(&self, callback: F)
    where
        F: Fn(PaneId, &CommandInfo) + Send + Sync + 'static,
    {
        if let Ok(mut callbacks) = self.command_callbacks.lock() {
            callbacks.push(Box::new(callback));
        }
    }

    /// 注册窗口标题变化回调
    pub fn register_title_callback<F>(&self, callback: F)
    where
        F: Fn(PaneId, &str) + Send + Sync + 'static,
    {
        if let Ok(mut callbacks) = self.title_callbacks.lock() {
            callbacks.push(Box::new(callback));
        }
    }

    /// 处理终端输出，解析OSC序列并更新状态
    pub fn process_output(&self, pane_id: PaneId, data: &str) {
        let sequences = self.parser.parse(data);

        for sequence in sequences {
            match sequence {
                OscSequence::CurrentWorkingDirectory { path } => {
                    self.update_cwd(pane_id, path);
                }
                OscSequence::WindowsTerminalCwd { path } => {
                    self.update_cwd(pane_id, path);
                }
                OscSequence::ShellIntegration { marker, data } => {
                    self.handle_shell_integration(pane_id, marker, data);
                }
                OscSequence::WindowTitle { title, .. } => {
                    self.update_window_title(pane_id, title);
                }
                OscSequence::Unknown { .. } => {
                    // 忽略未知OSC序列
                }
            }
        }
    }

    /// 处理 Shell Integration（OSC 133） 序列 - 修复并完善命令状态跟踪
    fn handle_shell_integration(
        &self,
        pane_id: PaneId,
        marker: IntegrationMarker,
        data: Option<String>,
    ) {
        if let Ok(mut states) = self.pane_states.lock() {
            let state = states.entry(pane_id).or_default();
            state.integration_state = ShellIntegrationState::Enabled;

            match marker {
                IntegrationMarker::PromptStart => {
                    // A: 提示符开始 - 准备接收新命令
                    // 如果有未完成的命令，先结束它
                    if state.current_command.is_some() {
                        state.finish_command(None);
                    }
                    // 不在这里创建新命令，等到CommandStart时再创建
                }
                IntegrationMarker::CommandStart => {
                    // B: 命令开始 - 用户开始输入命令
                    let command_id = state.start_command();
                    tracing::debug!("Command started: {} for pane {}", command_id, pane_id);

                    // 通知上下文服务缓存失效（命令状态变化）
                    self.notify_context_service_cache_invalidation(pane_id);
                }
                IntegrationMarker::CommandExecuted => {
                    // C: 命令执行开始 - 命令已提交执行
                    if let Some(command) = &mut state.current_command {
                        command.status = CommandStatus::Running;
                        tracing::debug!("Command executing: {} for pane {}", command.id, pane_id);
                        self.trigger_command_callbacks(pane_id, command);

                        // 通知上下文服务缓存失效（命令状态变化）
                        self.notify_context_service_cache_invalidation(pane_id);
                    }
                }
                IntegrationMarker::CommandFinished { exit_code } => {
                    // D: 命令执行完成
                    tracing::debug!(
                        "Command finished with exit code: {:?} for pane {}",
                        exit_code,
                        pane_id
                    );
                    state.finish_command(exit_code);

                    if let Some(last_command) = state.command_history.last() {
                        self.trigger_command_callbacks(pane_id, last_command);
                    }

                    // 通知上下文服务缓存失效（命令完成）
                    self.notify_context_service_cache_invalidation(pane_id);
                }
                IntegrationMarker::CommandCancelled => {
                    // 命令被取消
                    tracing::debug!("Command cancelled for pane {}", pane_id);
                    state.finish_command(Some(130)); // SIGINT退出码

                    // 通知上下文服务缓存失效（命令取消）
                    self.notify_context_service_cache_invalidation(pane_id);
                }
                IntegrationMarker::Property { key, value } => {
                    // P: 属性更新
                    tracing::debug!("Property update: {}={} for pane {}", key, value, pane_id);
                    // 可以在这里处理特定属性，如CWD等
                    if key == "Cwd" {
                        state.update_cwd(value.clone());
                        self.trigger_cwd_callbacks(pane_id, &value);
                    }
                }
                _ => {
                    // 其他标记
                    tracing::debug!(
                        "Unhandled shell integration marker: {:?} with data: {:?}",
                        marker,
                        data
                    );
                }
            }
        }
    }

    /// 更新面板的CWD并触发回调
    fn update_cwd(&self, pane_id: PaneId, new_cwd: String) {
        let old_cwd = if let Ok(mut states) = self.pane_states.lock() {
            let state = states.entry(pane_id).or_default();
            let old = state.current_working_directory.clone();
            state.update_cwd(new_cwd.clone());
            old
        } else {
            return;
        };

        // 只有CWD真的变化了才触发回调和事件
        if old_cwd.as_ref() != Some(&new_cwd) {
            // 触发传统回调
            self.trigger_cwd_callbacks(pane_id, &new_cwd);

            // 通知上下文服务缓存失效和发送事件
            self.notify_context_service_cwd_changed(pane_id, old_cwd, new_cwd);
        }
    }

    /// 更新窗口标题
    fn update_window_title(&self, pane_id: PaneId, title: String) {
        if let Ok(mut states) = self.pane_states.lock() {
            let state = states.entry(pane_id).or_default();
            let old_title = state.window_title.clone();
            state.window_title = Some(title.clone());
            state.last_activity = SystemTime::now();

            if old_title.as_ref() != Some(&title) {
                self.trigger_title_callbacks(pane_id, &title);

                // 通知上下文服务缓存失效（窗口标题变化）
                self.notify_context_service_cache_invalidation(pane_id);
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
        self.pane_states
            .lock()
            .ok()?
            .get(&pane_id)?
            .current_working_directory
            .clone()
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
            let old_shell_type = state.shell_type.clone();
            state.shell_type = Some(shell_type.clone());

            // 如果Shell类型发生变化，通知上下文服务
            if old_shell_type.as_ref() != Some(&shell_type) {
                drop(states); // 释放锁
                self.notify_context_service_cache_invalidation(pane_id);
            }
        }
    }

    /// 生成Shell集成脚本
    pub fn generate_shell_script(&self, shell_type: &ShellType) -> Result<String> {
        self.script_generator
            .generate_integration_script(shell_type)
    }

    /// 生成Shell环境变量
    pub fn generate_shell_env_vars(&self, shell_type: &ShellType) -> HashMap<String, String> {
        self.script_generator.generate_env_vars(shell_type)
    }

    /// 启用Shell Integration
    pub fn enable_integration(&self, pane_id: PaneId) {
        let was_enabled = self.is_integration_enabled(pane_id);

        if let Ok(mut states) = self.pane_states.lock() {
            let state = states.entry(pane_id).or_default();
            state.integration_state = ShellIntegrationState::Enabled;
        }

        // 如果状态发生变化，通知上下文服务
        if !was_enabled {
            self.notify_context_service_integration_changed(pane_id, true);
        }
    }

    /// 禁用Shell Integration
    pub fn disable_integration(&self, pane_id: PaneId) {
        let was_enabled = self.is_integration_enabled(pane_id);

        if let Ok(mut states) = self.pane_states.lock() {
            let state = states.entry(pane_id).or_default();
            state.integration_state = ShellIntegrationState::Disabled;
        }

        // 如果状态发生变化，通知上下文服务
        if was_enabled {
            self.notify_context_service_integration_changed(pane_id, false);
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
        self.pane_states
            .lock()
            .ok()?
            .get(&pane_id)?
            .current_command
            .clone()
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

    /// 获取面板的Shell Integration状态
    pub fn get_integration_state(&self, pane_id: PaneId) -> ShellIntegrationState {
        if let Ok(states) = self.pane_states.lock() {
            if let Some(state) = states.get(&pane_id) {
                return state.integration_state.clone();
            }
        }
        ShellIntegrationState::Disabled
    }

    /// 获取面板的命令统计信息
    pub fn get_command_stats(&self, pane_id: PaneId) -> Option<(usize, usize, usize)> {
        if let Ok(states) = self.pane_states.lock() {
            if let Some(state) = states.get(&pane_id) {
                let total = state.command_history.len();
                let successful = state
                    .command_history
                    .iter()
                    .filter(|cmd| {
                        matches!(cmd.status, CommandStatus::Finished { exit_code: Some(0) })
                    })
                    .count();
                let failed = state.command_history.iter()
                    .filter(|cmd| matches!(cmd.status, CommandStatus::Finished { exit_code: Some(code) } if code != 0))
                    .count();
                return Some((total, successful, failed));
            }
        }
        None
    }

    /// 清理面板状态
    pub fn cleanup_pane(&self, pane_id: PaneId) {
        if let Ok(mut states) = self.pane_states.lock() {
            states.remove(&pane_id);
        }

        // 通知上下文服务清理缓存
        self.notify_context_service_cache_invalidation(pane_id);
    }

    /// 获取面板状态的快照（优化性能）
    ///
    /// 返回面板状态的克隆，避免长时间持有锁
    pub fn get_pane_state_snapshot(&self, pane_id: PaneId) -> Option<PaneShellState> {
        if let Ok(states) = self.pane_states.lock() {
            states.get(&pane_id).cloned()
        } else {
            None
        }
    }

    /// 批量获取多个面板的状态快照（优化性能）
    pub fn get_multiple_pane_states(&self, pane_ids: &[PaneId]) -> HashMap<PaneId, PaneShellState> {
        let mut result = HashMap::new();

        if let Ok(states) = self.pane_states.lock() {
            for &pane_id in pane_ids {
                if let Some(state) = states.get(&pane_id) {
                    result.insert(pane_id, state.clone());
                }
            }
        }

        result
    }

    /// 获取所有活跃面板的ID列表
    pub fn get_active_pane_ids(&self) -> Vec<PaneId> {
        if let Ok(states) = self.pane_states.lock() {
            states.keys().copied().collect()
        } else {
            Vec::new()
        }
    }

    // 私有方法：上下文服务集成

    /// 通知上下文服务缓存失效
    fn notify_context_service_cache_invalidation(&self, pane_id: PaneId) {
        if let Ok(service_ref) = self.context_service.lock() {
            if let Some(weak_service) = service_ref.as_ref() {
                if let Some(service) = weak_service.upgrade() {
                    service.invalidate_cache(pane_id);
                    tracing::debug!("已通知上下文服务缓存失效: pane_id={:?}", pane_id);
                }
            }
        }
    }

    /// 通知上下文服务CWD变化事件
    fn notify_context_service_cwd_changed(
        &self,
        pane_id: PaneId,
        old_cwd: Option<String>,
        new_cwd: String,
    ) {
        if let Ok(service_ref) = self.context_service.lock() {
            if let Some(weak_service) = service_ref.as_ref() {
                if let Some(service) = weak_service.upgrade() {
                    // 先失效缓存
                    service.invalidate_cache(pane_id);
                    // 再发送事件
                    service.send_cwd_changed_event(pane_id, old_cwd, new_cwd);
                    tracing::debug!("已通知上下文服务CWD变化: pane_id={:?}", pane_id);
                }
            }
        }
    }

    /// 通知上下文服务Shell集成状态变化事件
    fn notify_context_service_integration_changed(&self, pane_id: PaneId, enabled: bool) {
        if let Ok(service_ref) = self.context_service.lock() {
            if let Some(weak_service) = service_ref.as_ref() {
                if let Some(service) = weak_service.upgrade() {
                    // 先失效缓存
                    service.invalidate_cache(pane_id);
                    // 再发送事件
                    service.send_shell_integration_changed_event(pane_id, enabled);
                    tracing::debug!(
                        "已通知上下文服务Shell集成状态变化: pane_id={:?}, enabled={}",
                        pane_id,
                        enabled
                    );
                }
            }
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
    fn test_osc_133_sequences() {
        let manager = ShellIntegrationManager::new().unwrap();
        let pane_id = PaneId::new(1);

        // 测试OSC 133序列解析
        let test_data = "\x1b]133;A\x07\x1b]133;B\x07\x1b]133;C\x07\x1b]133;D;0\x07";
        manager.process_output(pane_id, test_data);

        // 验证命令历史
        let history = manager.get_command_history(pane_id);
        assert!(
            !history.is_empty(),
            "Should have command history after processing OSC 133 sequences"
        );

        // 验证Integration状态
        let state = manager.get_integration_state(pane_id);
        assert_eq!(state, ShellIntegrationState::Enabled);
    }

    #[test]
    fn test_command_lifecycle() {
        let manager = ShellIntegrationManager::new().unwrap();
        let pane_id = PaneId::new(1);

        // 模拟 Shell Integration 命令序列 - 修复：使用OSC 133
        manager.process_output(pane_id, "\x1b]133;A\x07"); // 提示符开始
        manager.process_output(pane_id, "\x1b]133;B\x07"); // 命令开始
        manager.process_output(pane_id, "\x1b]133;C\x07"); // 命令执行
        manager.process_output(pane_id, "\x1b]133;D;0\x07"); // 命令完成

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
        // 验证使用了正确的OSC 133序列
        assert!(
            bash_script.contains("133;C"),
            "Bash script should use OSC 133 sequences"
        );
        assert!(
            bash_script.contains("133;D"),
            "Bash script should use OSC 133 sequences"
        );

        let zsh_script = manager.generate_shell_script(&ShellType::Zsh).unwrap();
        assert!(
            zsh_script.contains("133;C"),
            "Zsh script should use OSC 133 sequences"
        );
        assert!(
            zsh_script.contains("133;D"),
            "Zsh script should use OSC 133 sequences"
        );

        let env_vars = manager.generate_shell_env_vars(&ShellType::Bash);
        assert!(env_vars.contains_key("ORBITX_SHELL_INTEGRATION"));
    }

    #[test]
    fn test_context_service_integration() {
        let manager = ShellIntegrationManager::new().unwrap();
        let pane_id = PaneId::new(1);

        // 测试性能优化方法
        let snapshot = manager.get_pane_state_snapshot(pane_id);
        assert!(snapshot.is_none()); // 初始状态应该为空

        // 设置一些状态
        manager.set_pane_shell_type(pane_id, ShellType::Bash);
        manager.update_current_working_directory(pane_id, "/test/path".to_string());

        // 测试快照获取
        let snapshot = manager.get_pane_state_snapshot(pane_id);
        assert!(snapshot.is_some());
        let state = snapshot.unwrap();
        assert_eq!(state.shell_type, Some(ShellType::Bash));
        assert_eq!(
            state.current_working_directory,
            Some("/test/path".to_string())
        );

        // 测试批量获取
        let pane_ids = vec![pane_id];
        let states = manager.get_multiple_pane_states(&pane_ids);
        assert_eq!(states.len(), 1);
        assert!(states.contains_key(&pane_id));

        // 测试活跃面板ID列表
        let active_panes = manager.get_active_pane_ids();
        assert!(active_panes.contains(&pane_id));
    }

    #[test]
    fn test_integration_state_changes() {
        let manager = ShellIntegrationManager::new().unwrap();
        let pane_id = PaneId::new(1);

        // 初始状态应该是禁用的
        assert!(!manager.is_integration_enabled(pane_id));

        // 启用集成
        manager.enable_integration(pane_id);
        assert!(manager.is_integration_enabled(pane_id));

        // 禁用集成
        manager.disable_integration(pane_id);
        assert!(!manager.is_integration_enabled(pane_id));
    }

    #[test]
    fn test_cleanup_pane() {
        let manager = ShellIntegrationManager::new().unwrap();
        let pane_id = PaneId::new(1);

        // 设置一些状态
        manager.set_pane_shell_type(pane_id, ShellType::Bash);
        manager.update_current_working_directory(pane_id, "/test/path".to_string());

        // 验证状态存在
        assert!(manager.get_pane_state_snapshot(pane_id).is_some());

        // 清理面板
        manager.cleanup_pane(pane_id);

        // 验证状态已被清理
        assert!(manager.get_pane_state_snapshot(pane_id).is_none());
    }
}
