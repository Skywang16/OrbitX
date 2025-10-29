use dashmap::DashMap;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock, Weak};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::broadcast;

use crate::events::ShellEvent;
use super::osc_parser::{
    CommandStatus, IntegrationMarker, OscParser, OscSequence, ShellIntegrationState,
};
use super::script_generator::{ShellIntegrationConfig, ShellScriptGenerator, ShellType};
use crate::mux::PaneId;
use crate::shell::error::ShellScriptResult;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommandInfo {
    pub id: u64,
    #[serde(skip)]
    pub start_time: Instant,
    #[serde(skip)]
    pub start_time_wallclock: SystemTime,
    #[serde(skip)]
    pub end_time: Option<Instant>,
    #[serde(skip)]
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
    pub current_command: Option<Arc<CommandInfo>>,
    pub command_history: VecDeque<Arc<CommandInfo>>,
    pub next_command_id: u64,
    pub window_title: Option<String>,
    pub last_activity: SystemTime,
    pub node_version: Option<String>,
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
            node_version: None,
        }
    }
}

pub trait ContextServiceIntegration: Send + Sync {
    fn invalidate_cache(&self, pane_id: PaneId);
    fn send_cwd_changed_event(&self, pane_id: PaneId, old_cwd: Option<String>, new_cwd: String);
    fn send_shell_integration_changed_event(&self, pane_id: PaneId, enabled: bool);
}

pub struct ShellIntegrationManager {
    states: DashMap<PaneId, PaneShellState>,
    parser: OscParser,
    script_generator: ShellScriptGenerator,
    history_limit: usize,
    context_service: RwLock<Option<Weak<dyn ContextServiceIntegration>>>,
    event_sender: broadcast::Sender<(PaneId, ShellEvent)>,
}

impl ShellIntegrationManager {
    pub fn new() -> Self {
        Self::new_with_config(ShellIntegrationConfig::default())
    }

    pub fn new_with_config(config: ShellIntegrationConfig) -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        
        Self {
            states: DashMap::new(),
            parser: OscParser::new(),
            script_generator: ShellScriptGenerator::new(config),
            history_limit: 128,
            context_service: RwLock::new(None),
            event_sender,
        }
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

    pub fn subscribe_events(&self) -> broadcast::Receiver<(PaneId, ShellEvent)> {
        self.event_sender.subscribe()
    }

    pub fn process_output(&self, pane_id: PaneId, data: &str) {
        for sequence in self.parser.parse(data) {
            match sequence {
                OscSequence::CurrentWorkingDirectory { path } => {
                    if let Some(event) = self.apply_cwd(pane_id, path) {
                        let _ = self.event_sender.send((pane_id, event));
                    }
                }
                OscSequence::WindowsTerminalCwd { path } => {
                    if let Some(event) = self.apply_cwd(pane_id, path) {
                        let _ = self.event_sender.send((pane_id, event));
                    }
                }
                OscSequence::ShellIntegration { marker, data } => {
                    for event in self.apply_shell_integration(pane_id, marker, data) {
                        let _ = self.event_sender.send((pane_id, event.clone()));
                    }
                }
                OscSequence::WindowTitle { title, .. } => {
                    if let Some(event) = self.apply_title(pane_id, title) {
                        let _ = self.event_sender.send((pane_id, event));
                    }
                }
                OscSequence::OrbitXNodeVersion { version } => {
                    if let Some(event) = self.apply_node_version(pane_id, version) {
                        let _ = self.event_sender.send((pane_id, event));
                    }
                }
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

    pub fn with_pane_state<F, R>(&self, pane_id: PaneId, f: F) -> Option<R>
    where
        F: FnOnce(&PaneShellState) -> R,
    {
        self.states.get(&pane_id).map(|state| f(state.value()))
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

    pub fn generate_shell_script(&self, shell_type: &ShellType) -> ShellScriptResult<String> {
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

    pub fn with_current_command<F, R>(&self, pane_id: PaneId, f: F) -> Option<R>
    where
        F: FnOnce(&CommandInfo) -> R,
    {
        self.states
            .get(&pane_id)
            .and_then(|state| state.current_command.as_ref().map(|cmd| f(cmd)))
    }

    pub fn get_current_command(&self, pane_id: PaneId) -> Option<Arc<CommandInfo>> {
        self.states
            .get(&pane_id)
            .and_then(|state| state.current_command.as_ref().map(Arc::clone))
    }

    pub fn get_command_history(&self, pane_id: PaneId) -> Vec<Arc<CommandInfo>> {
        self.states
            .get(&pane_id)
            .map(|state| state.command_history.iter().map(Arc::clone).collect())
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


    pub fn get_multiple_pane_states(&self, pane_ids: &[PaneId]) -> HashMap<PaneId, PaneShellState> {
        pane_ids
            .iter()
            .filter_map(|id| self.states.get(id).map(|state| (*id, state.clone())))
            .collect()
    }

    pub fn get_active_pane_ids(&self) -> Vec<PaneId> {
        self.states.iter().map(|entry| *entry.key()).collect()
    }

    fn apply_cwd(&self, pane_id: PaneId, new_path: String) -> Option<ShellEvent> {
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
                    Arc::make_mut(cmd).working_directory = Some(new_path.clone());
                }
                Some((old, new_path))
            }
        };

        change.map(|(old, new_cwd)| {
            self.notify_context_service_cwd_changed(pane_id, old, new_cwd.clone());
            ShellEvent::CwdChanged { new_cwd }
        })
    }

    fn apply_title(&self, pane_id: PaneId, title: String) -> Option<ShellEvent> {
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

        changed.map(|new_title| {
            self.notify_context_service_cache_invalidation(pane_id);
            ShellEvent::TitleChanged { new_title }
        })
    }

    fn apply_node_version(&self, pane_id: PaneId, new_version: String) -> Option<ShellEvent> {
        let changed = {
            let mut entry = self
                .states
                .entry(pane_id)
                .or_insert_with(PaneShellState::new);
            let state = entry.value_mut();

            let normalized_version = if new_version.is_empty() {
                None
            } else {
                Some(new_version.clone())
            };

            if state.node_version == normalized_version {
                None
            } else {
                state.node_version = normalized_version;
                state.last_activity = SystemTime::now();
                Some(new_version)
            }
        };

        changed.map(|version| ShellEvent::NodeVersionChanged { version })
    }

    fn apply_shell_integration(
        &self,
        pane_id: PaneId,
        marker: IntegrationMarker,
        data: Option<String>,
    ) -> Vec<ShellEvent> {
        let mut events = Vec::new();
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
                        let mut finished = Arc::try_unwrap(cmd).unwrap_or_else(|arc| (*arc).clone());
                        finished.end_time = Some(Instant::now());
                        finished.end_time_wallclock = Some(SystemTime::now());
                        finished.status = CommandStatus::Finished { exit_code: None };
                        let finished_arc = Arc::new(finished);
                        state.command_history.push_back(Arc::clone(&finished_arc));
                        if state.command_history.len() > self.history_limit {
                            state.command_history.pop_front();
                        }
                        command_events.push(finished_arc);
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
                    let command_arc = Arc::new(command);
                    state.current_command = Some(Arc::clone(&command_arc));
                    command_events.push(command_arc);
                    self.notify_context_service_cache_invalidation(pane_id);
                }
                IntegrationMarker::CommandExecuted => {
                    if let Some(cmd) = &mut state.current_command {
                        let cmd_mut = Arc::make_mut(cmd);
                        cmd_mut.status = CommandStatus::Running;
                        if cmd_mut.command_line.is_none() {
                            if let Some(ref line) = data {
                                if !line.is_empty() {
                                    cmd_mut.command_line = Some(line.clone());
                                }
                            }
                        }
                        command_events.push(Arc::clone(cmd));
                    }
                }
                IntegrationMarker::CommandFinished { exit_code } => {
                    if let Some(cmd) = state.current_command.take() {
                        let mut finished = Arc::try_unwrap(cmd).unwrap_or_else(|arc| (*arc).clone());
                        finished.end_time = Some(Instant::now());
                        finished.end_time_wallclock = Some(SystemTime::now());
                        finished.exit_code = exit_code;
                        finished.status = CommandStatus::Finished { exit_code };
                        let finished_arc = Arc::new(finished);
                        state.command_history.push_back(Arc::clone(&finished_arc));
                        if state.command_history.len() > self.history_limit {
                            state.command_history.pop_front();
                        }
                        command_events.push(finished_arc);
                        self.notify_context_service_cache_invalidation(pane_id);
                    }
                }
                IntegrationMarker::CommandContinuation => {
                    if let (Some(cmd), Some(ref fragment)) = (&mut state.current_command, &data) {
                        let cmd_mut = Arc::make_mut(cmd);
                        let entry = cmd_mut.command_line.get_or_insert_with(String::new);
                        if !entry.is_empty() {
                            entry.push(' ');
                        }
                        entry.push_str(fragment);
                        command_events.push(Arc::clone(cmd));
                    }
                }
                IntegrationMarker::RightPrompt => {}
                IntegrationMarker::CommandInvalid => {
                    if let Some(cmd) = state.current_command.take() {
                        let mut finished = Arc::try_unwrap(cmd).unwrap_or_else(|arc| (*arc).clone());
                        finished.end_time = Some(Instant::now());
                        finished.end_time_wallclock = Some(SystemTime::now());
                        finished.status = CommandStatus::Finished { exit_code: None };
                        let finished_arc = Arc::new(finished);
                        state.command_history.push_back(Arc::clone(&finished_arc));
                        if state.command_history.len() > self.history_limit {
                            state.command_history.pop_front();
                        }
                        command_events.push(finished_arc);
                    }
                }
                IntegrationMarker::CommandCancelled => {
                    if let Some(cmd) = state.current_command.take() {
                        let mut cancelled = Arc::try_unwrap(cmd).unwrap_or_else(|arc| (*arc).clone());
                        cancelled.end_time = Some(Instant::now());
                        cancelled.end_time_wallclock = Some(SystemTime::now());
                        cancelled.exit_code = Some(130);
                        cancelled.status = CommandStatus::Finished {
                            exit_code: Some(130),
                        };
                        let cancelled_arc = Arc::new(cancelled);
                        state.command_history.push_back(Arc::clone(&cancelled_arc));
                        if state.command_history.len() > self.history_limit {
                            state.command_history.pop_front();
                        }
                        command_events.push(cancelled_arc);
                        self.notify_context_service_cache_invalidation(pane_id);
                    }
                }
                IntegrationMarker::Property { key, value } => {
                    if key.eq_ignore_ascii_case("cwd") {
                        if let Some(event) = self.apply_cwd(pane_id, value) {
                            events.push(event);
                        }
                    }
                }
            }
        }

        // 转换命令事件为ShellEvent
        events.extend(command_events.into_iter().map(|command| ShellEvent::CommandEvent { command }));
        events
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
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_command_lifecycle() {
        let manager = ShellIntegrationManager::new();
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
        let manager = ShellIntegrationManager::new();
        let pane_id = PaneId::new(2);
        manager.process_output(pane_id, "\u{1b}]7;file://localhost/tmp\u{7}");
        assert_eq!(
            manager.get_current_working_directory(pane_id),
            Some("/tmp".to_string())
        );
    }
}
