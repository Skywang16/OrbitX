use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::Emitter;
use tauri::{AppHandle, Runtime};
use tokio::sync::Notify;
use uuid::Uuid;

use crate::completion::output_analyzer::OutputAnalyzer;
use crate::events::ShellEvent;
use crate::mux::singleton::get_mux;
use crate::mux::{PaneId, PtySize, TerminalConfig};
use crate::mux::{ShellConfig, TerminalMux};
use crate::terminal::TerminalScrollback;

use super::types::{AgentTerminal, TerminalExecutionMode, TerminalId, TerminalStatus};

struct AgentTerminalEntry {
    terminal: AgentTerminal,
    notify: Arc<Notify>,
}

type EventEmitter = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;

pub struct AgentTerminalManager {
    terminals: RwLock<HashMap<TerminalId, AgentTerminalEntry>>,
    pane_index: RwLock<HashMap<u32, TerminalId>>,
    pending_completed: RwLock<HashMap<i64, Vec<TerminalId>>>,
    session_index: RwLock<HashMap<i64, TerminalId>>,
    mux: Arc<TerminalMux>,
    emitter: EventEmitter,
}

static AGENT_TERMINAL_MANAGER: OnceLock<Arc<AgentTerminalManager>> = OnceLock::new();

impl AgentTerminalManager {
    pub fn init<R: Runtime>(app_handle: AppHandle<R>) -> Arc<Self> {
        if let Some(manager) = AGENT_TERMINAL_MANAGER.get() {
            return Arc::clone(manager);
        }

        let mux = get_mux();
        let emitter: EventEmitter = Arc::new(move |event, payload| {
            let _ = app_handle.emit(event, payload);
        });

        let manager = Arc::new(Self {
            terminals: RwLock::new(HashMap::new()),
            pane_index: RwLock::new(HashMap::new()),
            pending_completed: RwLock::new(HashMap::new()),
            session_index: RwLock::new(HashMap::new()),
            mux: Arc::clone(&mux),
            emitter,
        });

        if AGENT_TERMINAL_MANAGER.set(Arc::clone(&manager)).is_err() {
            return manager;
        }

        manager.start_shell_event_loop();
        manager
    }

    pub fn global() -> Option<Arc<Self>> {
        AGENT_TERMINAL_MANAGER.get().cloned()
    }

    pub async fn create_terminal(
        &self,
        command: String,
        mode: TerminalExecutionMode,
        session_id: i64,
        cwd: Option<String>,
        label: Option<String>,
    ) -> Result<AgentTerminal, String> {
        let terminal_id = {
            let session_index = self
                .session_index
                .read()
                .map_err(|_| "session index poisoned".to_string())?;
            session_index
                .get(&session_id)
                .cloned()
                .unwrap_or_else(|| Uuid::new_v4().to_string())
        };

        let (existing_pane_id, notify) = {
            let terminals = self
                .terminals
                .read()
                .map_err(|_| "terminal map poisoned".to_string())?;
            if let Some(entry) = terminals.get(&terminal_id) {
                (Some(entry.terminal.pane_id), Some(Arc::clone(&entry.notify)))
            } else {
                (None, None)
            }
        };

        let notify = notify.unwrap_or_else(|| Arc::new(Notify::new()));

        let pane_id = if let Some(pane_id) = existing_pane_id.map(PaneId::new) {
            if self.mux.pane_exists(pane_id) {
                pane_id
            } else {
                self.create_agent_pane(cwd.as_deref()).await?
            }
        } else {
            self.create_agent_pane(cwd.as_deref()).await?
        };

        let now_ms_value = now_ms();

        let command_line =
            if let Some(working_dir) = cwd.as_deref().filter(|v| !v.trim().is_empty()) {
                format!(
                    "cd {} && {}",
                    shell_escape_single_quotes(working_dir),
                    command
                )
            } else {
                command.clone()
            };

        let (is_new_terminal, previous_pane_id, terminal) = {
            let mut terminals = self
                .terminals
                .write()
                .map_err(|_| "terminal map poisoned".to_string())?;

            let is_new_terminal = !terminals.contains_key(&terminal_id);
            let entry = terminals.entry(terminal_id.clone()).or_insert_with(|| AgentTerminalEntry {
                terminal: AgentTerminal {
                    id: terminal_id.clone(),
                    command: command.clone(),
                    pane_id: pane_id.as_u32(),
                    mode: mode.clone(),
                    status: TerminalStatus::Initializing,
                    session_id,
                    created_at_ms: now_ms_value,
                    completed_at_ms: None,
                    label: label.clone(),
                },
                notify: Arc::clone(&notify),
            });

            if matches!(entry.terminal.status, TerminalStatus::Running) {
                return Err("Agent terminal is busy (a command is still running).".to_string());
            }

            let previous_pane_id = entry.terminal.pane_id;

            entry.terminal.command = command.clone();
            entry.terminal.pane_id = pane_id.as_u32();
            entry.terminal.mode = mode.clone();
            entry.terminal.status = TerminalStatus::Running;
            entry.terminal.session_id = session_id;
            entry.terminal.created_at_ms = now_ms_value;
            entry.terminal.completed_at_ms = None;
            entry.terminal.label = label.clone();

            (is_new_terminal, previous_pane_id, entry.terminal.clone())
        };

        {
            let mut session_index = self
                .session_index
                .write()
                .map_err(|_| "session index poisoned".to_string())?;
            session_index.insert(session_id, terminal_id.clone());
        }

        {
            let mut pane_index = self
                .pane_index
                .write()
                .map_err(|_| "pane index poisoned".to_string())?;
            if previous_pane_id != pane_id.as_u32() {
                pane_index.remove(&previous_pane_id);
            }
            pane_index.insert(pane_id.as_u32(), terminal_id.clone());
        }

        let wire = format!("{command_line}\n");
        if let Err(err) = self.mux.write_to_pane(pane_id, wire.as_bytes()) {
            let mut terminals = self
                .terminals
                .write()
                .map_err(|_| "terminal map poisoned".to_string())?;
            if let Some(entry) = terminals.get_mut(&terminal_id) {
                entry.terminal.status = TerminalStatus::Failed {
                    error: err.to_string(),
                };
                entry.terminal.completed_at_ms = Some(now_ms());
                entry.notify.notify_waiters();

                (self.emitter)(
                    "agent_terminal_updated",
                    serde_json::to_value(&entry.terminal)
                        .unwrap_or_else(|_| serde_json::json!({})),
                );
                (self.emitter)(
                    "agent_terminal_completed",
                    serde_json::to_value(&entry.terminal)
                        .unwrap_or_else(|_| serde_json::json!({})),
                );
            }
            return Err(format!("write command failed: {err}"));
        }

        (self.emitter)(
            if is_new_terminal {
                "agent_terminal_created"
            } else {
                "agent_terminal_updated"
            },
            serde_json::to_value(&terminal).unwrap_or_else(|_| serde_json::json!({})),
        );

        Ok(terminal)
    }

    pub fn list_terminals(&self, session_id: Option<i64>) -> Vec<AgentTerminal> {
        let terminals = match self.terminals.read() {
            Ok(guard) => guard,
            Err(_) => return Vec::new(),
        };

        let mut list: Vec<AgentTerminal> = terminals
            .values()
            .map(|entry| entry.terminal.clone())
            .filter(|terminal| session_id.map(|id| terminal.session_id == id).unwrap_or(true))
            .collect();

        list.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
        list
    }

    pub fn get_terminal(&self, terminal_id: &str) -> Option<AgentTerminal> {
        let terminals = self.terminals.read().ok()?;
        terminals.get(terminal_id).map(|entry| entry.terminal.clone())
    }

    pub fn get_terminal_status(&self, terminal_id: &str) -> Option<TerminalStatus> {
        self.get_terminal(terminal_id).map(|t| t.status)
    }

    pub async fn wait_for_completion(
        &self,
        terminal_id: &str,
        timeout: Duration,
    ) -> Result<TerminalStatus, String> {
        let notify = {
            let terminals = self
                .terminals
                .read()
                .map_err(|_| "terminal map poisoned".to_string())?;
            let entry = terminals
                .get(terminal_id)
                .ok_or_else(|| "terminal not found".to_string())?;
            Arc::clone(&entry.notify)
        };

        if let Some(status) = self.get_terminal_status(terminal_id) {
            if status.is_terminal() {
                return Ok(status);
            }
        }

        tokio::time::timeout(timeout, notify.notified())
            .await
            .map_err(|_| "timeout waiting for terminal completion".to_string())?;

        self.get_terminal_status(terminal_id)
            .ok_or_else(|| "terminal not found".to_string())
    }

    pub fn get_terminal_output(&self, terminal_id: &str) -> Result<String, String> {
        let terminal = self
            .get_terminal(terminal_id)
            .ok_or_else(|| "terminal not found".to_string())?;
        Ok(TerminalScrollback::global().get_text_lossy(terminal.pane_id))
    }

    pub fn get_terminal_last_command_output(&self, terminal_id: &str) -> Result<String, String> {
        let terminal = self
            .get_terminal(terminal_id)
            .ok_or_else(|| "terminal not found".to_string())?;

        Ok(OutputAnalyzer::global()
            .get_last_command_output(terminal.pane_id)
            .map_err(|e| format!("read last command output failed: {e}"))?
            .unwrap_or_default())
    }

    pub fn abort_terminal(&self, terminal_id: &str) -> Result<(), String> {
        let mut terminals = self
            .terminals
            .write()
            .map_err(|_| "terminal map poisoned".to_string())?;
        let entry = terminals
            .get_mut(terminal_id)
            .ok_or_else(|| "terminal not found".to_string())?;

        if entry.terminal.status.is_terminal() {
            return Ok(());
        }

        let pane_id = PaneId::new(entry.terminal.pane_id);
        let _ = self.mux.write_to_pane(pane_id, b"\x03");

        entry.terminal.status = TerminalStatus::Aborted;
        entry.terminal.completed_at_ms = Some(now_ms());

        let mut pending = self.pending_completed.write().unwrap_or_else(|e| e.into_inner());
        pending
            .entry(entry.terminal.session_id)
            .or_default()
            .push(entry.terminal.id.clone());

        (self.emitter)(
            "agent_terminal_updated",
            serde_json::to_value(&entry.terminal).unwrap_or_else(|_| serde_json::json!({})),
        );
        (self.emitter)(
            "agent_terminal_completed",
            serde_json::to_value(&entry.terminal).unwrap_or_else(|_| serde_json::json!({})),
        );

        entry.notify.notify_waiters();
        Ok(())
    }

    pub fn remove_terminal(&self, terminal_id: &str) -> Result<(), String> {
        let terminal = {
            let terminals = self
                .terminals
                .read()
                .map_err(|_| "terminal map poisoned".to_string())?;
            terminals
                .get(terminal_id)
                .map(|entry| entry.terminal.clone())
                .ok_or_else(|| "terminal not found".to_string())?
        };

        let pane_id = PaneId::new(terminal.pane_id);
        let _ = self.mux.remove_pane(pane_id);

        {
            let mut terminals = self
                .terminals
                .write()
                .map_err(|_| "terminal map poisoned".to_string())?;
            terminals.remove(terminal_id);
        }
        {
            let mut session_index = self
                .session_index
                .write()
                .map_err(|_| "session index poisoned".to_string())?;
            session_index.retain(|_, value| value != terminal_id);
        }
        {
            let mut pane_index = self
                .pane_index
                .write()
                .map_err(|_| "pane index poisoned".to_string())?;
            pane_index.remove(&terminal.pane_id);
        }
        {
            let mut pending = self.pending_completed.write().unwrap_or_else(|e| e.into_inner());
            if let Some(list) = pending.get_mut(&terminal.session_id) {
                list.retain(|id| id != terminal_id);
                if list.is_empty() {
                    pending.remove(&terminal.session_id);
                }
            }
        }

        (self.emitter)(
            "agent_terminal_removed",
            serde_json::json!({ "terminalId": terminal_id }),
        );

        Ok(())
    }

    pub fn drain_completed_notifications(&self, session_id: i64) -> Vec<AgentTerminal> {
        let ids = {
            let mut pending = match self.pending_completed.write() {
                Ok(guard) => guard,
                Err(_) => return Vec::new(),
            };
            pending.remove(&session_id).unwrap_or_default()
        };

        ids.into_iter()
            .filter_map(|id| self.get_terminal(&id))
            .collect()
    }

    pub fn build_prompt_overlay(&self, session_id: i64) -> Option<String> {
        let running_background: Vec<AgentTerminal> = self
            .list_terminals(Some(session_id))
            .into_iter()
            .filter(|t| {
                t.mode == TerminalExecutionMode::Background
                    && matches!(t.status, TerminalStatus::Running)
            })
            .collect();

        let completed = self.drain_completed_notifications(session_id);

        if running_background.is_empty() && completed.is_empty() {
            return None;
        }

        let mut overlay = String::new();
        overlay.push_str("## Agent Terminals\n\n");

        if !running_background.is_empty() {
            overlay.push_str("### Running (background)\n");
            for term in &running_background {
                overlay.push_str(&format!(
                    "- `{}`: `{}` (use `read_agent_terminal` with terminalId)\n",
                    term.id, term.command
                ));
            }
            overlay.push('\n');
        }

        if !completed.is_empty() {
            overlay.push_str("### Completed (background)\n");
            for term in &completed {
                let exit_code = match term.status {
                    TerminalStatus::Completed { exit_code } => exit_code,
                    _ => None,
                };
                let exit_label = exit_code
                    .map(|code| format!("exit {code}"))
                    .unwrap_or_else(|| "exit unknown".to_string());
                overlay.push_str(&format!(
                    "- `{}`: `{}` finished ({}) - use `read_agent_terminal`\n",
                    term.id, term.command, exit_label
                ));
            }
            overlay.push('\n');
        }

        Some(overlay)
    }

    fn start_shell_event_loop(self: &Arc<Self>) {
        let mut receiver = self.mux.shell_integration().subscribe_events();
        let manager = Arc::downgrade(self);
        tauri::async_runtime::spawn(async move {
            loop {
                let event = match receiver.recv().await {
                    Ok(item) => item,
                    Err(_) => break,
                };

                let Some(manager) = manager.upgrade() else {
                    break;
                };

                let (pane_id, shell_event) = event;
                manager.handle_shell_event(pane_id, shell_event);
            }
        });
    }

    fn handle_shell_event(&self, pane_id: PaneId, event: ShellEvent) {
        let ShellEvent::CommandEvent { command } = event else {
            return;
        };

        if !command.is_finished() {
            return;
        }

        // Ensure last-command output is recorded before we notify waiters.
        // TerminalEventHandler also processes these events, but it may run after this manager.
        let _ = OutputAnalyzer::global().on_shell_command_event(pane_id.as_u32(), &command);

        let terminal_id = {
            let pane_index = match self.pane_index.read() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            pane_index.get(&pane_id.as_u32()).cloned()
        };

        let Some(terminal_id) = terminal_id else {
            return;
        };

        let mut terminals = match self.terminals.write() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        let entry = match terminals.get_mut(&terminal_id) {
            Some(entry) => entry,
            None => return,
        };

        if entry.terminal.status.is_terminal() {
            return;
        }

        entry.terminal.status = TerminalStatus::Completed {
            exit_code: command.exit_code,
        };
        entry.terminal.completed_at_ms = Some(now_ms());

        if entry.terminal.mode == TerminalExecutionMode::Background {
            let mut pending = self.pending_completed.write().unwrap_or_else(|e| e.into_inner());
            pending
                .entry(entry.terminal.session_id)
                .or_default()
                .push(terminal_id.clone());
        }

        (self.emitter)(
            "agent_terminal_updated",
            serde_json::to_value(&entry.terminal).unwrap_or_else(|_| serde_json::json!({})),
        );
        (self.emitter)(
            "agent_terminal_completed",
            serde_json::to_value(&entry.terminal).unwrap_or_else(|_| serde_json::json!({})),
        );

        entry.notify.notify_waiters();
    }

    async fn create_agent_pane(&self, cwd: Option<&str>) -> Result<PaneId, String> {
        let size = PtySize::new(24, 80);
        let mut shell_config = ShellConfig::with_default_shell();
        shell_config.shell_info.display_name = "agent".to_string();
        shell_config.shell_info.name = "agent".to_string();
        if let Some(working_dir) = cwd.filter(|v| !v.trim().is_empty()) {
            shell_config.working_directory = Some(working_dir.into());
        }
        let config = TerminalConfig::with_shell(shell_config);
        self.mux
            .create_pane_with_config(size, &config)
            .await
            .map_err(|e| format!("create pane failed: {e}"))
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn shell_escape_single_quotes(value: &str) -> String {
    // POSIX-ish single-quote escaping: close, escape quote, reopen.
    // Example: abc'd -> 'abc'\''d'
    let mut out = String::with_capacity(value.len() + 2);
    out.push('\'');
    for ch in value.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}
