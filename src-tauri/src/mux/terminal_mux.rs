//! TerminalMux - Core terminal multiplexer
//!
//! Provides unified terminal session management, event notifications, and PTY I/O handling

use crossbeam_channel::{unbounded, Sender};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use tracing::{error, instrument, warn};

use crate::mux::{
    error::{TerminalMuxError, TerminalMuxResult},
    IoHandler, LocalPane, MuxNotification, MuxSessionConfig, Pane, PaneId, PaneRuntimeMetadata,
    PtySize,
};
use crate::shell::ShellIntegrationManager;

pub type SubscriberCallback = Box<dyn Fn(&MuxNotification) -> bool + Send + Sync>;

#[derive(Debug, Clone)]
pub struct TerminalMuxStatus {
    pub pane_count: usize,
    pub subscriber_count: usize,
    pub next_pane_id: u32,
    pub next_subscriber_id: u32,
}

pub struct TerminalMux {
    panes: RwLock<HashMap<PaneId, Arc<dyn Pane>>>,

    pane_metadata: RwLock<HashMap<PaneId, PaneRuntimeMetadata>>,

    /// Event subscribers - subscriber ID -> callback function
    subscribers: RwLock<HashMap<usize, SubscriberCallback>>,

    /// Next pane ID generator
    next_pane_id: AtomicU32,

    /// Next subscriber ID generator
    next_subscriber_id: AtomicU32,

    /// Cross-thread notification sender
    notification_sender: Sender<MuxNotification>,

    /// Notification processing thread handle (if auto-processing mode is enabled)
    notification_thread: Mutex<Option<thread::JoinHandle<()>>>,

    /// I/O handler
    io_handler: IoHandler,

    /// Shell Integration manager
    shell_integration: Arc<ShellIntegrationManager>,

    /// Whether shutdown is in progress (for graceful exit of notification processing thread)
    shutting_down: std::sync::atomic::AtomicBool,
}

impl TerminalMux {
    /// Create and start a shared instance with notification thread (recommended)
    pub fn new_shared() -> Arc<Self> {
        Self::new_shared_with_shell_integration(Arc::new(ShellIntegrationManager::new()))
    }

    /// Create and start a shared instance with notification thread (allows injecting ShellIntegrationManager)
    pub fn new_shared_with_shell_integration(
        shell_integration: Arc<ShellIntegrationManager>,
    ) -> Arc<Self> {
        let (notification_sender, notification_receiver) = unbounded();
        let io_handler = IoHandler::new(notification_sender.clone(), shell_integration.clone());

        let mux = Arc::new(Self {
            panes: RwLock::new(HashMap::new()),
            pane_metadata: RwLock::new(HashMap::new()),
            subscribers: RwLock::new(HashMap::new()),
            next_pane_id: AtomicU32::new(1),
            next_subscriber_id: AtomicU32::new(1),
            notification_sender,
            notification_thread: Mutex::new(None),
            io_handler,
            shell_integration,
            shutting_down: std::sync::atomic::AtomicBool::new(false),
        });

        // Notification thread owns receiver; TerminalMux only keeps sender to avoid "Option receiver" patched state machine.
        let mux_for_thread = Arc::clone(&mux);
        let handle = thread::spawn(move || loop {
            if mux_for_thread
                .shutting_down
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                break;
            }

            match notification_receiver.recv_timeout(Duration::from_millis(20)) {
                Ok(notification) => mux_for_thread.notify_internal(&notification),
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }
        });

        {
            let mut guard = match mux.notification_thread.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    warn!("Notification thread mutex poisoned, recovering");
                    poisoned.into_inner()
                }
            };
            *guard = Some(handle);
        }

        mux
    }

    pub fn shell_integration(&self) -> &Arc<ShellIntegrationManager> {
        &self.shell_integration
    }

    pub fn get_status(&self) -> TerminalMuxResult<TerminalMuxStatus> {
        let panes = self
            .panes
            .read()
            .map_err(|err| TerminalMuxError::from_read_poison("panes", err))?;
        let subscribers = self
            .subscribers
            .read()
            .map_err(|err| TerminalMuxError::from_read_poison("subscribers", err))?;

        let status = TerminalMuxStatus {
            pane_count: panes.len(),
            subscriber_count: subscribers.len(),
            next_pane_id: self.next_pane_id.load(Ordering::Relaxed),
            next_subscriber_id: self.next_subscriber_id.load(Ordering::Relaxed),
        };

        Ok(status)
    }

    /// Generate next unique pane ID
    fn next_pane_id(&self) -> PaneId {
        let id = self.next_pane_id.fetch_add(1, Ordering::Relaxed);
        PaneId::new(id)
    }

    /// Generate next unique subscriber ID
    fn next_subscriber_id(&self) -> usize {
        self.next_subscriber_id.fetch_add(1, Ordering::Relaxed) as usize
    }

    /// Create new pane
    pub async fn create_pane(&self, size: PtySize) -> TerminalMuxResult<PaneId> {
        let config = MuxSessionConfig::with_default_shell().map_err(TerminalMuxError::Internal)?;
        self.create_pane_with_config(size, &config).await
    }

    /// Create new pane with specified configuration
    ///
    /// - Uses structured logging format
    /// - Includes performance metrics
    #[instrument(skip(self, config), fields(pane_id, shell = %config.shell_config.shell_info.display_name))]
    pub async fn create_pane_with_config(
        &self,
        size: PtySize,
        config: &MuxSessionConfig,
    ) -> TerminalMuxResult<PaneId> {
        let pane_id = self.next_pane_id();
        let pane = Arc::new(LocalPane::new_with_config(pane_id, size, config)?);

        // Add to pane mapping
        {
            let mut panes = self
                .panes
                .write()
                .map_err(|err| TerminalMuxError::from_write_poison("panes", err))?;

            if panes.contains_key(&pane_id) {
                return Err(TerminalMuxError::PaneExists { pane_id });
            }
            panes.insert(pane_id, pane.clone());
        }

        {
            let mut pane_metadata = self
                .pane_metadata
                .write()
                .map_err(|err| TerminalMuxError::from_write_poison("pane_metadata", err))?;
            pane_metadata.insert(pane_id, config.runtime_metadata.clone());
        }

        // Set pane's Shell type to shell_integration
        let shell_type =
            crate::shell::ShellType::from_program(&config.shell_config.shell_info.path);
        self.shell_integration
            .set_pane_shell_type(pane_id, shell_type.clone());
        if let Some(cwd) = config.shell_config.working_directory.as_ref() {
            self.shell_integration
                .update_current_working_directory(pane_id, cwd.to_string_lossy().into_owned());
        }

        // Start I/O processing threads
        self.io_handler.spawn_io_threads(pane.clone())?;

        // Send pane added notification
        self.notify(MuxNotification::PaneAdded(pane_id));
        Ok(pane_id)
    }

    /// Get pane reference
    pub fn get_pane(&self, pane_id: PaneId) -> Option<Arc<dyn Pane>> {
        let panes = match self.panes.read() {
            Ok(panes) => panes,
            Err(err) => {
                error!(
                    "failed to acquire pane read lock for {:?}: {}",
                    pane_id, err
                );
                return None;
            }
        };
        panes.get(&pane_id).cloned()
    }

    /// Check if pane exists
    pub fn pane_exists(&self, pane_id: PaneId) -> bool {
        match self.panes.read() {
            Ok(panes) => panes.contains_key(&pane_id),
            Err(err) => {
                error!(
                    "failed to acquire pane read lock for {:?}: {}",
                    pane_id, err
                );
                false
            }
        }
    }

    /// Remove pane
    ///
    /// - Uses structured logging format
    /// - Includes performance metrics
    #[instrument(skip(self), fields(pane_id = ?pane_id))]
    pub fn remove_pane(&self, pane_id: PaneId) -> TerminalMuxResult<()> {
        let pane = {
            let mut panes = self
                .panes
                .write()
                .map_err(|err| TerminalMuxError::from_write_poison("panes", err))?;

            panes
                .remove(&pane_id)
                .ok_or(TerminalMuxError::PaneNotFound { pane_id })?
        };

        if let Ok(mut pane_metadata) = self.pane_metadata.write() {
            pane_metadata.remove(&pane_id);
        } else {
            warn!(
                "Failed to acquire pane metadata write lock while removing pane {:?}",
                pane_id
            );
        }

        // Mark pane as dead, stop I/O threads
        pane.mark_dead();

        // Stop I/O processing
        if let Err(e) = self.io_handler.stop_pane_io(pane_id) {
            warn!("Failed to stop pane {:?} I/O processing: {}", pane_id, e);
        }

        // Send pane removed notification
        self.notify(MuxNotification::PaneRemoved(pane_id));
        Ok(())
    }

    /// Get list of all pane IDs
    pub fn list_panes(&self) -> Vec<PaneId> {
        match self.panes.read() {
            Ok(panes) => panes.keys().copied().collect(),
            Err(err) => {
                error!(
                    "failed to acquire pane read lock while listing panes: {}",
                    err
                );
                Vec::new()
            }
        }
    }

    pub fn get_pane_runtime_metadata(&self, pane_id: PaneId) -> Option<PaneRuntimeMetadata> {
        match self.pane_metadata.read() {
            Ok(pane_metadata) => pane_metadata.get(&pane_id).cloned(),
            Err(err) => {
                error!(
                    "failed to acquire pane metadata read lock for {:?}: {}",
                    pane_id, err
                );
                None
            }
        }
    }

    /// Get pane count
    pub fn pane_count(&self) -> usize {
        match self.panes.read() {
            Ok(panes) => panes.len(),
            Err(err) => {
                error!(
                    "failed to acquire pane read lock while counting panes: {}",
                    err
                );
                0
            }
        }
    }

    /// Write data to specified pane
    ///
    /// - Uses structured logging format
    /// - Includes performance metrics
    #[instrument(skip(self, data), fields(pane_id = ?pane_id, data_len = data.len()), level = "trace")]
    pub fn write_to_pane(&self, pane_id: PaneId, data: &[u8]) -> TerminalMuxResult<()> {
        let pane = self
            .get_pane(pane_id)
            .ok_or(TerminalMuxError::PaneNotFound { pane_id })?;

        pane.write(data)?;
        Ok(())
    }

    /// Resize pane
    ///
    /// - Uses structured logging format
    /// - Includes performance metrics
    #[instrument(skip(self), fields(pane_id = ?pane_id, size = ?size))]
    pub fn resize_pane(&self, pane_id: PaneId, size: PtySize) -> TerminalMuxResult<()> {
        let pane = self
            .get_pane(pane_id)
            .ok_or(TerminalMuxError::PaneNotFound { pane_id })?;

        pane.resize(size)?;

        // Send resize notification
        self.notify(MuxNotification::PaneResized { pane_id, size });
        Ok(())
    }

    /// Subscribe to event notifications
    pub fn subscribe<F>(&self, subscriber: F) -> usize
    where
        F: Fn(&MuxNotification) -> bool + Send + Sync + 'static,
    {
        let subscriber_id = self.next_subscriber_id();

        match self.subscribers.write() {
            Ok(mut subscribers) => {
                subscribers.insert(subscriber_id, Box::new(subscriber));
            }
            Err(err) => {
                error!("Failed to acquire subscriber write lock: {}", err);
            }
        }

        subscriber_id
    }

    /// Unsubscribe
    pub fn unsubscribe(&self, subscriber_id: usize) -> bool {
        match self.subscribers.write() {
            Ok(mut subscribers) => subscribers.remove(&subscriber_id).is_some(),
            Err(err) => {
                error!("Failed to acquire subscriber write lock: {}", err);
                false
            }
        }
    }

    /// Send notification to all subscribers
    pub fn notify(&self, notification: MuxNotification) {
        if let Err(e) = self.notification_sender.send(notification) {
            error!("Failed to send cross-thread notification: {}", e);
        }
    }

    /// Internal notification implementation (executed serially within notification thread)
    fn notify_internal(&self, notification: &MuxNotification) {
        let mut dead_subscribers = Vec::new();

        match self.subscribers.read() {
            Ok(subscribers) => {
                for (&subscriber_id, callback) in subscribers.iter() {
                    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        callback(notification)
                    })) {
                        Ok(true) => {}
                        Ok(false) => {
                            dead_subscribers.push(subscriber_id);
                        }
                        Err(_) => {
                            error!("Subscriber {} callback panicked", subscriber_id);
                            dead_subscribers.push(subscriber_id);
                        }
                    }
                }
            }
            Err(err) => {
                error!("Failed to acquire subscriber read lock: {}", err);
                return;
            }
        }

        // Clean up invalid subscribers
        if !dead_subscribers.is_empty() {
            match self.subscribers.write() {
                Ok(mut subscribers) => {
                    for subscriber_id in dead_subscribers {
                        subscribers.remove(&subscriber_id);
                    }
                }
                Err(err) => {
                    error!(
                        "Failed to acquire subscriber write lock for cleanup: {}",
                        err
                    );
                }
            }
        }
    }

    /// Set up pane's Shell Integration and inject script
    pub fn setup_pane_integration_with_script(
        &self,
        pane_id: PaneId,
        silent: bool,
    ) -> TerminalMuxResult<()> {
        // Enable Shell Integration
        self.shell_integration.enable_integration(pane_id);

        // Get Shell type already set from shell_integration
        let panes = self
            .panes
            .read()
            .map_err(|err| TerminalMuxError::from_read_poison("panes", err))?;

        if panes.get(&pane_id).is_none() {
            return Err(TerminalMuxError::PaneNotFound { pane_id });
        }

        let shell_type = self
            .shell_integration
            .with_pane_state(pane_id, |state| state.shell_type.clone())
            .flatten();
        let shell_type = match shell_type {
            Some(shell_type) => shell_type,
            None => {
                return Err(TerminalMuxError::Internal(format!(
                    "Pane {pane_id:?} has no shell type registered"
                )));
            }
        };

        if !silent {
            let script = self
                .shell_integration
                .generate_shell_script(&shell_type)
                .map_err(|err| {
                    TerminalMuxError::Internal(format!("Shell integration error: {err}"))
                })?;
            self.write_to_pane(pane_id, script.as_bytes())?;
        }

        Ok(())
    }

    /// Get pane's current working directory
    pub fn shell_get_pane_cwd(&self, pane_id: PaneId) -> Option<String> {
        self.shell_integration
            .get_current_working_directory(pane_id)
    }

    /// Update pane's current working directory
    pub fn shell_update_pane_cwd(&self, pane_id: PaneId, cwd: String) {
        self.shell_integration
            .update_current_working_directory(pane_id, cwd);
    }

    pub fn get_pane_shell_state(&self, pane_id: PaneId) -> Option<crate::shell::PaneShellState> {
        self.shell_integration.get_pane_shell_state(pane_id)
    }

    pub fn set_pane_shell_type(&self, pane_id: PaneId, shell_type: crate::shell::ShellType) {
        self.shell_integration
            .set_pane_shell_type(pane_id, shell_type);
    }

    /// Generate Shell environment variables
    pub fn generate_shell_env_vars(
        &self,
        shell_type: &crate::shell::ShellType,
    ) -> std::collections::HashMap<String, String> {
        self.shell_integration.generate_shell_env_vars(shell_type)
    }

    /// Clean up all resources
    pub fn shutdown(&self) -> TerminalMuxResult<()> {
        let shutdown_start = std::time::Instant::now();

        // Mark as shutting down so notification processing thread can exit quickly
        self.shutting_down
            .store(true, std::sync::atomic::Ordering::Relaxed);

        match self.notification_thread.lock() {
            Ok(mut guard) => {
                if let Some(handle) = guard.take() {
                    if handle.join().is_err() {
                        tracing::warn!("Notification thread panicked during shutdown");
                    }
                }
            }
            Err(err) => {
                tracing::warn!(
                    "Failed to lock notification thread during shutdown: {}",
                    err
                );
            }
        }

        let pane_ids: Vec<PaneId> = self.list_panes();

        // Immediately mark all panes as dead to speed up shutdown process
        {
            let panes = self
                .panes
                .read()
                .map_err(|err| TerminalMuxError::from_read_poison("panes", err))?;
            for (_pane_id, pane) in panes.iter() {
                pane.mark_dead();
            }
        }

        // Close panes one by one
        let mut failed_panes = Vec::new();
        for pane_id in pane_ids {
            match self.remove_pane(pane_id) {
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("Failed to close pane {:?}: {}", pane_id, e);
                    failed_panes.push(pane_id);
                }
            }

            if shutdown_start.elapsed() > Duration::from_secs(3) {
                tracing::warn!("Shutdown timeout, forcing exit of remaining panes");
                break;
            }
        }

        if !failed_panes.is_empty() {
            tracing::warn!(
                "{} panes failed to close: {:?}",
                failed_panes.len(),
                failed_panes
            );
        }

        // Clean up all subscribers
        match self.subscribers.write() {
            Ok(mut subscribers) => subscribers.clear(),
            Err(err) => {
                tracing::warn!("Failed to clear subscribers during shutdown: {}", err);
            }
        }

        // Shutdown I/O handler
        if let Err(err) = self.io_handler.shutdown() {
            tracing::warn!("Failed to shut down I/O handler cleanly: {}", err);
        }

        Ok(())
    }
}

// Implement Debug trait for debugging
impl std::fmt::Debug for TerminalMux {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalMux")
            .field("pane_count", &self.pane_count())
            .field("next_pane_id", &self.next_pane_id.load(Ordering::Relaxed))
            .field(
                "next_subscriber_id",
                &self.next_subscriber_id.load(Ordering::Relaxed),
            )
            .finish()
    }
}

// Thread safety marker
// Relies on automatic derivation of Send/Sync from member types, no explicit unsafe marker needed
