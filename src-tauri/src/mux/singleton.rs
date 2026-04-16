//! TerminalMux global singleton management
//!
//! Ensures the entire application has only one Mux instance

use std::sync::{Arc, OnceLock};

use crate::mux::{MuxError, MuxResult, TerminalMux};

/// Global TerminalMux singleton instance
static GLOBAL_MUX: OnceLock<Arc<TerminalMux>> = OnceLock::new();

/// Get global TerminalMux instance
///
/// This function is thread-safe, creates the instance on first call,
/// and returns a reference to the same instance on subsequent calls
///
/// Note: It's recommended to use init_mux_with_shell_integration during
/// application initialization to specify ShellIntegrationManager and ensure
/// callbacks are properly registered
pub fn get_mux() -> Arc<TerminalMux> {
    GLOBAL_MUX.get_or_init(|| init_mux_internal(None)).clone()
}

/// Initialize global TerminalMux with specified ShellIntegrationManager
///
/// This function should be called once at application startup to ensure Mux uses
/// the correct ShellIntegrationManager. Returns an error if already initialized.
pub fn init_mux_with_shell_integration(
    shell_integration: std::sync::Arc<crate::shell::ShellIntegrationManager>,
) -> Result<Arc<TerminalMux>, &'static str> {
    GLOBAL_MUX
        .set(init_mux_internal(Some(shell_integration)))
        .map_err(|_| "TerminalMux already initialized")?;
    GLOBAL_MUX
        .get()
        .cloned()
        .ok_or("TerminalMux initialization failed")
}

fn init_mux_internal(
    shell_integration: Option<std::sync::Arc<crate::shell::ShellIntegrationManager>>,
) -> Arc<TerminalMux> {
    // Ensure the mux config manager is initialized before any mux component tries to
    // read it.  ConfigManager::init() is idempotent-safe: it only fails if called a
    // second time, which cannot happen here because init_mux_internal is only ever
    // executed once (inside OnceLock::set / get_or_init).
    if let Err(err) = crate::mux::ConfigManager::init() {
        tracing::warn!("Terminal mux config manager init failed: {}", err);
    }

    if let Some(integration) = shell_integration {
        TerminalMux::new_shared_with_shell_integration(integration)
    } else {
        TerminalMux::new_shared()
    }
}

/// Shutdown global TerminalMux instance
///
/// Note: This function should only be called once when the application is closing.
/// After calling, get_mux() will still return the closed instance.
pub fn shutdown_mux() -> MuxResult<()> {
    if let Some(mux) = GLOBAL_MUX.get() {
        mux.shutdown().map_err(MuxError::from)
    } else {
        Ok(())
    }
}
