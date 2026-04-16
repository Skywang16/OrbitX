use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::Notify;
use uuid::Uuid;

use crate::events::ShellEvent;
use crate::mux::singleton::get_mux;
use crate::mux::{MuxSessionConfig, MuxShellConfig, PaneId, PtySize, TerminalMux};
use crate::terminal::TerminalScrollback;

use super::types::{AgentTerminal, TerminalExecutionMode, TerminalId, TerminalStatus};

struct AgentTerminalEntry {
    terminal: AgentTerminal,
    notify: Arc<Notify>,
}

pub struct AgentTerminalManager {
    terminals: RwLock<HashMap<TerminalId, AgentTerminalEntry>>,
    pane_index: RwLock<HashMap<u32, TerminalId>>,
    pending_completed: RwLock<HashMap<i64, Vec<TerminalId>>>,
    /// One persistent pane per thread for blocking shell commands.
    thread_panes: RwLock<HashMap<i64, PaneId>>,
    mux: Arc<TerminalMux>,
}

static AGENT_TERMINAL_MANAGER: OnceLock<Arc<AgentTerminalManager>> = OnceLock::new();

impl AgentTerminalManager {
    pub fn init() -> Arc<Self> {
        let manager = AGENT_TERMINAL_MANAGER.get_or_init(|| {
            let mux = get_mux();
            let manager = Arc::new(Self {
                terminals: RwLock::new(HashMap::new()),
                pane_index: RwLock::new(HashMap::new()),
                pending_completed: RwLock::new(HashMap::new()),
                thread_panes: RwLock::new(HashMap::new()),
                mux: Arc::clone(&mux),
            });
            manager.start_shell_event_loop();
            manager
        });
        Arc::clone(manager)
    }

    pub fn global() -> Option<Arc<Self>> {
        AGENT_TERMINAL_MANAGER.get().cloned()
    }

    pub async fn create_terminal(
        &self,
        command: String,
        mode: TerminalExecutionMode,
        thread_id: i64,
        cwd: Option<String>,
        label: Option<String>,
    ) -> Result<AgentTerminal, String> {
        let terminal_id = Uuid::new_v4().to_string();
        let notify = Arc::new(Notify::new());

        // Blocking commands reuse a single persistent pane per thread.
        // Background commands always get their own fresh pane.
        let pane_id = if mode == TerminalExecutionMode::Blocking {
            self.get_or_create_thread_pane(thread_id, cwd.as_deref())
                .await?
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

        let terminal = {
            let mut terminals = self
                .terminals
                .write()
                .map_err(|_| "terminal map poisoned".to_string())?;

            let terminal = AgentTerminal {
                id: terminal_id.clone(),
                command: command.clone(),
                pane_id: pane_id.as_u32(),
                mode: mode.clone(),
                status: TerminalStatus::Running,
                thread_id,
                created_at_ms: now_ms_value,
                completed_at_ms: None,
                label: label.clone(),
            };

            terminals.insert(
                terminal_id.clone(),
                AgentTerminalEntry {
                    terminal: terminal.clone(),
                    notify: Arc::clone(&notify),
                },
            );

            terminal
        };

        {
            let mut pane_index = self
                .pane_index
                .write()
                .map_err(|_| "pane index poisoned".to_string())?;
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
            }
            return Err(format!("write command failed: {err}"));
        }

        Ok(terminal)
    }

    /// Get the persistent pane for a thread, creating it if it doesn't exist yet.
    async fn get_or_create_thread_pane(
        &self,
        thread_id: i64,
        cwd: Option<&str>,
    ) -> Result<PaneId, String> {
        // Fast path: pane already exists.
        {
            let thread_panes = self
                .thread_panes
                .read()
                .map_err(|_| "thread_panes poisoned".to_string())?;
            if let Some(&pane_id) = thread_panes.get(&thread_id) {
                // Verify the pane is still alive in the mux.
                if self.mux.pane_exists(pane_id) {
                    return Ok(pane_id);
                }
            }
        }

        // Slow path: create a new pane and register it.
        let pane_id = self.create_agent_pane(cwd).await?;
        {
            let mut thread_panes = self
                .thread_panes
                .write()
                .map_err(|_| "thread_panes poisoned".to_string())?;
            thread_panes.insert(thread_id, pane_id);
        }
        Ok(pane_id)
    }

    pub fn list_terminals(&self, thread_id: Option<i64>) -> Vec<AgentTerminal> {
        let terminals = match self.terminals.read() {
            Ok(guard) => guard,
            Err(err) => {
                tracing::error!("terminal map poisoned while listing terminals: {err}");
                return Vec::new();
            }
        };

        let mut list: Vec<AgentTerminal> = terminals
            .values()
            .map(|entry| entry.terminal.clone())
            .filter(|terminal| match thread_id {
                Some(id) => terminal.thread_id == id,
                None => true,
            })
            .collect();

        list.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));
        list
    }

    pub fn get_terminal(&self, terminal_id: &str) -> Option<AgentTerminal> {
        let terminals = match self.terminals.read() {
            Ok(guard) => guard,
            Err(err) => {
                tracing::error!(
                    "terminal map poisoned while reading terminal {terminal_id}: {err}"
                );
                return None;
            }
        };
        terminals
            .get(terminal_id)
            .map(|entry| entry.terminal.clone())
    }

    pub fn get_terminal_by_pane_id(&self, pane_id: u32) -> Option<AgentTerminal> {
        let terminal_id = {
            let pane_index = match self.pane_index.read() {
                Ok(guard) => guard,
                Err(err) => {
                    tracing::error!("pane index poisoned while reading pane {pane_id}: {err}");
                    return None;
                }
            };
            pane_index.get(&pane_id).cloned()?
        };

        self.get_terminal(&terminal_id)
    }

    pub fn get_terminal_status(&self, terminal_id: &str) -> Option<TerminalStatus> {
        self.get_terminal(terminal_id).map(|t| t.status)
    }

    pub async fn wait_for_completion(
        &self,
        terminal_id: &str,
        timeout: Duration,
    ) -> Result<TerminalStatus, String> {
        let notify_arc = {
            let terminals = self
                .terminals
                .read()
                .map_err(|_| "terminal map poisoned".to_string())?;
            let entry = terminals
                .get(terminal_id)
                .ok_or_else(|| "terminal not found".to_string())?;

            if entry.terminal.status.is_terminal() {
                return Ok(entry.terminal.status.clone());
            }

            Arc::clone(&entry.notify)
        };

        let notified = notify_arc.notified();
        if let Some(status) = self.get_terminal_status(terminal_id) {
            if status.is_terminal() {
                return Ok(status);
            }
        }

        tokio::time::timeout(timeout, notified)
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

        TerminalScrollback::global()
            .get_last_command_output(terminal.pane_id)
            .ok_or_else(|| "last command output is unavailable".to_string())
    }

    pub fn drain_completed_notifications(&self, thread_id: i64) -> Vec<AgentTerminal> {
        let ids = {
            let mut pending = match self.pending_completed.write() {
                Ok(guard) => guard,
                Err(err) => {
                    tracing::error!(
                        "pending completion queue poisoned while draining session {}: {}",
                        thread_id,
                        err
                    );
                    return Vec::new();
                }
            };
            pending.remove(&thread_id).unwrap_or_default()
        };

        ids.into_iter()
            .filter_map(|id| self.get_terminal(&id))
            .collect()
    }

    pub fn build_prompt_overlay(&self, thread_id: i64) -> Option<String> {
        let running_background: Vec<AgentTerminal> = self
            .list_terminals(Some(thread_id))
            .into_iter()
            .filter(|t| {
                t.mode == TerminalExecutionMode::Background
                    && matches!(t.status, TerminalStatus::Running)
            })
            .collect();

        let completed = self.drain_completed_notifications(thread_id);

        if running_background.is_empty() && completed.is_empty() {
            return None;
        }

        let mut overlay = String::new();
        overlay.push_str("## Agent Terminals\n\n");

        if !running_background.is_empty() {
            overlay.push_str("### Running (background)\n");
            for term in &running_background {
                overlay.push_str(&format!(
                    "- `{}`: `{}` (use `read_terminal` with terminalId)\n",
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
                let exit_label = match exit_code {
                    Some(code) => format!("exit {code}"),
                    None => "no exit code reported".to_string(),
                };
                overlay.push_str(&format!(
                    "- `{}`: `{}` finished ({}) - use `read_terminal` with terminalId\n",
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
        let output = TerminalScrollback::global().get_text_lossy(pane_id.as_u32());
        TerminalScrollback::global().set_last_command_output(pane_id.as_u32(), output);

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
            match self.pending_completed.write() {
                Ok(mut pending) => {
                    pending
                        .entry(entry.terminal.thread_id)
                        .or_default()
                        .push(terminal_id.clone());
                }
                Err(err) => {
                    tracing::error!(
                        "pending completion queue poisoned while handling pane {}: {}",
                        pane_id.as_u32(),
                        err
                    );
                }
            }
        }

        entry.notify.notify_waiters();
    }

    async fn create_agent_pane(&self, cwd: Option<&str>) -> Result<PaneId, String> {
        let size = PtySize::new(24, 80);
        let mut shell_config = MuxShellConfig::with_default_shell()?;
        shell_config.shell_info.display_name = "agent".to_string();
        shell_config.shell_info.name = "agent".to_string();
        if let Some(working_dir) = cwd.filter(|v| !v.trim().is_empty()) {
            shell_config.working_directory = Some(working_dir.into());
        }
        let config = MuxSessionConfig::with_shell(shell_config);
        self.mux
            .create_pane_with_config(size, &config)
            .await
            .map_err(|e| format!("create pane failed: {e}"))
    }
}

fn now_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as i64,
        Err(err) => {
            tracing::error!("system clock is before UNIX_EPOCH: {err}");
            0
        }
    }
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
