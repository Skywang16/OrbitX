use anyhow::Result;
use dashmap::DashMap;
use std::collections::{HashMap, VecDeque};
use std::sync::{RwLock, Weak};
use std::time::{Duration, Instant, SystemTime};

use super::osc_parser::{
    CommandStatus, IntegrationMarker, OscParser, OscSequence, ShellIntegrationState,
};
use super::script_generator::{ShellIntegrationConfig, ShellScriptGenerator, ShellType};
use crate::mux::PaneId;

#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub id: u64,
    pub start_time: Instant,
    pub start_time_wallclock: SystemTime,
    pub end_time: Option<Instant>,
    pub end_time_wallclock: Option<SystemTime>,
    pub exit_code: Option<i32>,
    pub status: CommandStatus,
    pub command_line: Option<String>,
    pub working_directory: Option<String>,
}

impl CommandInfo {
    fn new(id: u64, cwd: Option<String>) -> Self {
        Self {
            id,
            start_time: Instant::now(),
            start_time_wallclock: SystemTime::now(),
            end_time: None,
            end_time_wallclock: None,
            exit_code: None,
            status: CommandStatus::Running,
            command_line: None,
            working_directory: cwd,
        }
    }

    pub fn duration(&self) -> Duration {
        match self.end_time {
            Some(end) => end.duration_since(self.start_time),
            None => Instant::now().duration_since(self.start_time),
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.status, CommandStatus::Finished { .. })
    }
}

#[derive(Debug, Clone)]
pub struct PaneShellState {
    pub integration_state: ShellIntegrationState,
    pub shell_type: Option<ShellType>,
    pub current_working_directory: Option<String>,
    pub current_command: Option<CommandInfo>,
    pub command_history: VecDeque<CommandInfo>,
    pub next_command_id: u64,
    pub window_title: Option<String>,
    pub last_activity: SystemTime,
}

impl PaneShellState {
    fn new() -> Self {
        Self {
            integration_state: ShellIntegrationState::Disabled,
            shell_type: None,
            current_working_directory: None,
            current_command: None,
            command_history: VecDeque::new(),
            next_command_id: 1,
            window_title: None,
            last_activity: SystemTime::now(),
        }
    }
}

pub trait ContextServiceIntegration: Send + Sync {
    fn invalidate_cache(&self, pane_id: PaneId);
    fn send_cwd_changed_event(&self, pane_id: PaneId, old_cwd: Option<String>, new_cwd: String);
    fn send_shell_integration_changed_event(&self, pane_id: PaneId, enabled: bool);
}

struct CallbackRegistry {
    cwd: RwLock<Vec<Box<dyn Fn(PaneId, &str) + Send + Sync>>>,
    command: RwLock<Vec<Box<dyn Fn(PaneId, &CommandInfo) + Send + Sync>>>,
    title: RwLock<Vec<Box<dyn Fn(PaneId, &str) + Send + Sync>>>,
}

impl CallbackRegistry {
    fn new() -> Self {
        Self {
            cwd: RwLock::new(Vec::new()),
            command: RwLock::new(Vec::new()),
            title: RwLock::new(Vec::new()),
        }
    }

    fn on_cwd<F>(&self, cb: F)
    where
        F: Fn(PaneId, &str) + Send + Sync + 'static,
    {
        self.cwd.write().unwrap().push(Box::new(cb));
    }

    fn on_command<F>(&self, cb: F)
    where
        F: Fn(PaneId, &CommandInfo) + Send + Sync + 'static,
    {
        self.command.write().unwrap().push(Box::new(cb));
    }

    fn on_title<F>(&self, cb: F)
    where
        F: Fn(PaneId, &str) + Send + Sync + 'static,
    {
        self.title.write().unwrap().push(Box::new(cb));
    }

    fn emit_cwd(&self, pane_id: PaneId, cwd: &str) {
        for cb in self.cwd.read().unwrap().iter() {
            cb(pane_id, cwd);
        }
    }

    fn emit_command(&self, pane_id: PaneId, command: &CommandInfo) {
        for cb in self.command.read().unwrap().iter() {
            cb(pane_id, command);
        }
    }

    fn emit_title(&self, pane_id: PaneId, title: &str) {
        for cb in self.title.read().unwrap().iter() {
            cb(pane_id, title);
        }
    }
}

pub struct ShellIntegrationManager {
    states: DashMap<PaneId, PaneShellState>,
    parser: OscParser,
    script_generator: ShellScriptGenerator,
    callbacks: CallbackRegistry,
    history_limit: usize,
    context_service: RwLock<Option<Weak<dyn ContextServiceIntegration>>>,
}

impl ShellIntegrationManager {
    pub fn new() -> Result<Self> {
        Self::new_with_config(ShellIntegrationConfig::default())
    }

    pub fn new_with_config(config: ShellIntegrationConfig) -> Result<Self> {
        Ok(Self {
            states: DashMap::new(),
            parser: OscParser::new()?,
            script_generator: ShellScriptGenerator::new(config),
            callbacks: CallbackRegistry::new(),
            history_limit: 128,
            context_service: RwLock::new(None),
        })
    }

    pub fn set_context_service_integration(
        &self,
        context_service: Weak<dyn ContextServiceIntegration>,
    ) {
        *self.context_service.write().unwrap() = Some(context_service);
    }

    pub fn remove_context_service_integration(&self) {
        *self.context_service.write().unwrap() = None;
    }

    pub fn register_cwd_callback<F>(&self, callback: F)
    where
        F: Fn(PaneId, &str) + Send + Sync + 'static,
    {
        self.callbacks.on_cwd(callback);
    }

    pub fn register_command_callback<F>(&self, callback: F)
    where
        F: Fn(PaneId, &CommandInfo) + Send + Sync + 'static,
    {
        self.callbacks.on_command(callback);
    }

    pub fn register_title_callback<F>(&self, callback: F)
    where
        F: Fn(PaneId, &str) + Send + Sync + 'static,
    {
        self.callbacks.on_title(callback);
    }

    pub fn process_output(&self, pane_id: PaneId, data: &str) {
        for sequence in self.parser.parse(data) {
            match sequence {
                OscSequence::CurrentWorkingDirectory { path } => self.apply_cwd(pane_id, path),
                OscSequence::WindowsTerminalCwd { path } => self.apply_cwd(pane_id, path),
                OscSequence::ShellIntegration { marker, data } => {
                    self.apply_shell_integration(pane_id, marker, data)
                }
                OscSequence::WindowTitle { title, .. } => self.apply_title(pane_id, title),
                OscSequence::Unknown { .. } => {}
            }
        }
    }

    pub fn strip_osc_sequences(&self, data: &str) -> String {
        self.parser.strip_osc_sequences(data)
    }

    pub fn get_current_working_directory(&self, pane_id: PaneId) -> Option<String> {
        self.states
            .get(&pane_id)
            .and_then(|state| state.current_working_directory.clone())
    }

    pub fn update_current_working_directory(&self, pane_id: PaneId, cwd: String) {
        self.apply_cwd(pane_id, cwd);
    }

    pub fn get_pane_state(&self, pane_id: PaneId) -> Option<()> {
        self.states.get(&pane_id).map(|_| ())
    }

    pub fn get_pane_shell_state(&self, pane_id: PaneId) -> Option<PaneShellState> {
        self.states.get(&pane_id).map(|state| state.clone())
    }

    pub fn set_pane_shell_type(&self, pane_id: PaneId, shell_type: ShellType) {
        let changed = {
            let mut state = self
                .states
                .entry(pane_id)
                .or_insert_with(PaneShellState::new);
            let previous = state.value().shell_type.clone();
            if previous.as_ref() != Some(&shell_type) {
                state.value_mut().shell_type = Some(shell_type.clone());
                true
            } else {
                false
            }
        };
        if changed {
            self.notify_context_service_cache_invalidation(pane_id);
        }
    }

    pub fn generate_shell_script(&self, shell_type: &ShellType) -> Result<String> {
        self.script_generator
            .generate_integration_script(shell_type)
    }

    pub fn generate_shell_env_vars(&self, shell_type: &ShellType) -> HashMap<String, String> {
        self.script_generator.generate_env_vars(shell_type)
    }

    pub fn enable_integration(&self, pane_id: PaneId) {
        let changed = {
            let mut state = self
                .states
                .entry(pane_id)
                .or_insert_with(PaneShellState::new);
            if !matches!(
                state.value().integration_state,
                ShellIntegrationState::Enabled
            ) {
                state.value_mut().integration_state = ShellIntegrationState::Enabled;
                true
            } else {
                false
            }
        };
        if changed {
            self.notify_context_service_integration_changed(pane_id, true);
        }
    }

    pub fn disable_integration(&self, pane_id: PaneId) {
        let changed = {
            let mut state = self
                .states
                .entry(pane_id)
                .or_insert_with(PaneShellState::new);
            if !matches!(
                state.value().integration_state,
                ShellIntegrationState::Disabled
            ) {
                state.value_mut().integration_state = ShellIntegrationState::Disabled;
                state.value_mut().current_command = None;
                true
            } else {
                false
            }
        };
        if changed {
            self.notify_context_service_integration_changed(pane_id, false);
        }
    }

    pub fn is_integration_enabled(&self, pane_id: PaneId) -> bool {
        self.states
            .get(&pane_id)
            .map(|state| matches!(state.integration_state, ShellIntegrationState::Enabled))
            .unwrap_or(false)
    }

    pub fn get_current_command(&self, pane_id: PaneId) -> Option<CommandInfo> {
        self.states
            .get(&pane_id)
            .and_then(|state| state.current_command.clone())
    }

    pub fn get_command_history(&self, pane_id: PaneId) -> Vec<CommandInfo> {
        self.states
            .get(&pane_id)
            .map(|state| state.command_history.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_integration_state(&self, pane_id: PaneId) -> ShellIntegrationState {
        self.states
            .get(&pane_id)
            .map(|state| state.integration_state.clone())
            .unwrap_or(ShellIntegrationState::Disabled)
    }

    pub fn get_command_stats(&self, pane_id: PaneId) -> Option<(usize, usize, usize)> {
        self.states.get(&pane_id).map(|state| {
            let history_total = state.command_history.len();
            let running = state
                .current_command
                .as_ref()
                .filter(|cmd| !cmd.is_finished())
                .map(|_| 1)
                .unwrap_or(0);
            let finished = history_total;
            (history_total, running, finished)
        })
    }

    pub fn cleanup_pane(&self, pane_id: PaneId) {
        self.states.remove(&pane_id);
    }

    pub fn get_pane_state_snapshot(&self, pane_id: PaneId) -> Option<PaneShellState> {
        self.get_pane_shell_state(pane_id)
    }

    pub fn get_multiple_pane_states(&self, pane_ids: &[PaneId]) -> HashMap<PaneId, PaneShellState> {
        pane_ids
            .iter()
            .filter_map(|id| self.states.get(id).map(|state| (*id, state.clone())))
            .collect()
    }

    pub fn get_active_pane_ids(&self) -> Vec<PaneId> {
        self.states.iter().map(|entry| *entry.key()).collect()
    }

    fn apply_cwd(&self, pane_id: PaneId, new_path: String) {
        let change = {
            let mut entry = self
                .states
                .entry(pane_id)
                .or_insert_with(PaneShellState::new);
            let state = entry.value_mut();
            if state.current_working_directory.as_ref() == Some(&new_path) {
                None
            } else {
                let old = state.current_working_directory.clone();
                state.current_working_directory = Some(new_path.clone());
                state.last_activity = SystemTime::now();
                if let Some(cmd) = &mut state.current_command {
                    cmd.working_directory = Some(new_path.clone());
                }
                Some((old, new_path))
            }
        };

        if let Some((old, new_value)) = change {
            self.callbacks.emit_cwd(pane_id, &new_value);
            self.notify_context_service_cwd_changed(pane_id, old, new_value);
        }
    }

    fn apply_title(&self, pane_id: PaneId, title: String) {
        let changed = {
            let mut entry = self
                .states
                .entry(pane_id)
                .or_insert_with(PaneShellState::new);
            let state = entry.value_mut();
            if state.window_title.as_ref() == Some(&title) {
                None
            } else {
                state.window_title = Some(title.clone());
                state.last_activity = SystemTime::now();
                Some(title)
            }
        };

        if let Some(title) = changed {
            self.callbacks.emit_title(pane_id, &title);
            self.notify_context_service_cache_invalidation(pane_id);
        }
    }

    fn apply_shell_integration(
        &self,
        pane_id: PaneId,
        marker: IntegrationMarker,
        data: Option<String>,
    ) {
        let mut command_events = Vec::new();
        {
            let mut entry = self
                .states
                .entry(pane_id)
                .or_insert_with(PaneShellState::new);
            let state = entry.value_mut();
            state.integration_state = ShellIntegrationState::Enabled;
            state.last_activity = SystemTime::now();

            match marker {
                IntegrationMarker::PromptStart => {
                    if let Some(cmd) = state.current_command.take() {
                        let mut finished = cmd;
                        finished.end_time = Some(Instant::now());
                        finished.end_time_wallclock = Some(SystemTime::now());
                        finished.status = CommandStatus::Finished { exit_code: None };
                        state.command_history.push_back(finished.clone());
                        if state.command_history.len() > self.history_limit {
                            state.command_history.pop_front();
                        }
                        command_events.push(finished);
                    }
                }
                IntegrationMarker::CommandStart => {
                    let mut command = CommandInfo::new(
                        state.next_command_id,
                        state.current_working_directory.clone(),
                    );
                    if let Some(ref line) = data {
                        if !line.is_empty() {
                            command.command_line = Some(line.clone());
                        }
                    }
                    state.next_command_id += 1;
                    state.current_command = Some(command.clone());
                    command_events.push(command);
                    self.notify_context_service_cache_invalidation(pane_id);
                }
                IntegrationMarker::CommandExecuted => {
                    if let Some(cmd) = &mut state.current_command {
                        cmd.status = CommandStatus::Running;
                        if cmd.command_line.is_none() {
                            if let Some(ref line) = data {
                                if !line.is_empty() {
                                    cmd.command_line = Some(line.clone());
                                }
                            }
                        }
                        command_events.push(cmd.clone());
                    }
                }
                IntegrationMarker::CommandFinished { exit_code } => {
                    if let Some(mut cmd) = state.current_command.take() {
                        cmd.end_time = Some(Instant::now());
                        cmd.end_time_wallclock = Some(SystemTime::now());
                        cmd.exit_code = exit_code;
                        cmd.status = CommandStatus::Finished { exit_code };
                        state.command_history.push_back(cmd.clone());
                        if state.command_history.len() > self.history_limit {
                            state.command_history.pop_front();
                        }
                        command_events.push(cmd);
                        self.notify_context_service_cache_invalidation(pane_id);
                    }
                }
                IntegrationMarker::CommandContinuation => {
                    if let (Some(cmd), Some(ref fragment)) = (&mut state.current_command, &data) {
                        let entry = cmd.command_line.get_or_insert_with(String::new);
                        if !entry.is_empty() {
                            entry.push(' ');
                        }
                        entry.push_str(fragment);
                        command_events.push(cmd.clone());
                    }
                }
                IntegrationMarker::RightPrompt => {}
                IntegrationMarker::CommandInvalid => {
                    if let Some(mut cmd) = state.current_command.take() {
                        cmd.end_time = Some(Instant::now());
                        cmd.end_time_wallclock = Some(SystemTime::now());
                        cmd.status = CommandStatus::Finished { exit_code: None };
                        state.command_history.push_back(cmd.clone());
                        if state.command_history.len() > self.history_limit {
                            state.command_history.pop_front();
                        }
                        command_events.push(cmd);
                    }
                }
                IntegrationMarker::CommandCancelled => {
                    if let Some(mut cmd) = state.current_command.take() {
                        cmd.end_time = Some(Instant::now());
                        cmd.end_time_wallclock = Some(SystemTime::now());
                        cmd.exit_code = Some(130);
                        cmd.status = CommandStatus::Finished {
                            exit_code: Some(130),
                        };
                        state.command_history.push_back(cmd.clone());
                        if state.command_history.len() > self.history_limit {
                            state.command_history.pop_front();
                        }
                        command_events.push(cmd);
                        self.notify_context_service_cache_invalidation(pane_id);
                    }
                }
                IntegrationMarker::Property { key, value } => {
                    if key.eq_ignore_ascii_case("cwd") {
                        self.apply_cwd(pane_id, value);
                    }
                }
            }
        }

        for event in command_events {
            self.callbacks.emit_command(pane_id, &event);
        }
    }

    fn notify_context_service_cache_invalidation(&self, pane_id: PaneId) {
        if let Some(service) = self
            .context_service
            .read()
            .unwrap()
            .as_ref()
            .and_then(|w| w.upgrade())
        {
            service.invalidate_cache(pane_id);
        }
    }

    fn notify_context_service_cwd_changed(
        &self,
        pane_id: PaneId,
        old_cwd: Option<String>,
        new_cwd: String,
    ) {
        if let Some(service) = self
            .context_service
            .read()
            .unwrap()
            .as_ref()
            .and_then(|w| w.upgrade())
        {
            service.invalidate_cache(pane_id);
            service.send_cwd_changed_event(pane_id, old_cwd, new_cwd);
        }
    }

    fn notify_context_service_integration_changed(&self, pane_id: PaneId, enabled: bool) {
        if let Some(service) = self
            .context_service
            .read()
            .unwrap()
            .as_ref()
            .and_then(|w| w.upgrade())
        {
            service.invalidate_cache(pane_id);
            service.send_shell_integration_changed_event(pane_id, enabled);
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
    fn tracks_command_lifecycle() {
        let manager = ShellIntegrationManager::new().unwrap();
        let pane_id = PaneId::new(1);
        manager.process_output(pane_id, "\u{1b}]133;A\u{7}");
        manager.process_output(pane_id, "\u{1b}]133;B\u{7}");
        manager.process_output(pane_id, "\u{1b}]133;C\u{7}");
        manager.process_output(pane_id, "\u{1b}]133;D;0\u{7}");

        let history = manager.get_command_history(pane_id);
        assert_eq!(history.len(), 1);
        assert!(history[0].is_finished());
    }

    #[test]
    fn updates_cwd() {
        let manager = ShellIntegrationManager::new().unwrap();
        let pane_id = PaneId::new(2);
        manager.process_output(pane_id, "\u{1b}]7;file://localhost/tmp\u{7}");
        assert_eq!(
            manager.get_current_working_directory(pane_id),
            Some("/tmp".to_string())
        );
    }
}
