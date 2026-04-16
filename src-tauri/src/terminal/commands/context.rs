/*!
 * Terminal context management commands
 *
 * Provides terminal context information retrieval functionality, including:
 * - Get context for specified terminal
 * - Get context for active terminal
 * - Support fallback logic handling
 */

use super::TerminalContextState;
use crate::mux::PaneId;
use crate::terminal::{ContextServiceError, TerminalContext};
use crate::utils::TauriApiResult;
use crate::{api_error, api_success};
use tauri::State;
use tracing::{error, warn};

/// Get context information for specified terminal
///
/// Get complete terminal context information based on provided pane ID, including current working directory,
/// shell type, command history, etc. If pane ID is not provided, get context for current active terminal.
///
/// # Arguments
/// * `pane_id` - Optional pane ID, if None then use active terminal
/// * `state` - Terminal context state
///
/// # Returns
/// * `Ok(TerminalContext)` - Terminal context information
/// * `Err(String)` - Error message if retrieval failed
#[tauri::command]
pub async fn terminal_context_get(
    pane_id: Option<u32>,
    state: State<'_, TerminalContextState>,
) -> TauriApiResult<TerminalContext> {
    if let Some(id) = pane_id {
        if id == 0 {
            warn!("Pane ID cannot be 0");
            return Ok(api_error!("common.invalid_id"));
        }
    }

    let pane_id = pane_id.map(PaneId::new);

    // Use context service to get terminal context, supports fallback logic
    match state
        .context_service
        .get_context_with_fallback(pane_id)
        .await
    {
        Ok(context) => Ok(api_success!(context)),
        Err(e) => {
            error!("Failed to get terminal context: {}", e);
            Ok(api_error!("terminal.get_context_failed"))
        }
    }
}

/// Get context information for current active terminal
///
///
/// # Arguments
/// * `state` - Terminal context state
///
/// # Returns
/// * `Ok(TerminalContext)` - Active terminal context information
/// * `Err(String)` - Error message if retrieval failed
#[tauri::command]
pub async fn terminal_context_get_active(
    state: State<'_, TerminalContextState>,
) -> TauriApiResult<TerminalContext> {
    match state.context_service.get_active_context().await {
        Ok(context) => Ok(api_success!(context)),
        Err(ContextServiceError::NoActivePane) => {
            match state.context_service.get_context_with_fallback(None).await {
                Ok(context) => Ok(api_success!(context)),
                Err(e) => {
                    error!(
                        "Failed to get active terminal context (fallback also failed): {}",
                        e
                    );
                    Ok(api_error!("terminal.get_active_context_failed"))
                }
            }
        }
        Err(e) => {
            error!("Failed to get active terminal context: {}", e);
            Ok(api_error!("terminal.get_active_context_failed"))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mux::PaneId;
    use crate::terminal::commands::tests::create_test_state;

    fn expected_fallback_cwd() -> String {
        dirs::home_dir()
            .or_else(|| std::env::current_dir().ok())
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "~".to_string())
    }

    #[tokio::test]
    async fn test_get_terminal_context_fallback() {
        let state = create_test_state();

        // When there's no active terminal, should return default context
        let result = state.context_service.get_context_with_fallback(None).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(
            context.current_working_directory,
            Some(expected_fallback_cwd())
        );
        assert!(matches!(
            context.shell_type,
            Some(crate::terminal::ShellType::Bash)
        ));
    }

    #[tokio::test]
    async fn test_get_active_terminal_context_fallback() {
        let state = create_test_state();

        // When there's no active terminal, get_active_context should return error
        let result = state.context_service.get_active_context().await;
        assert!(
            result.is_err()
                && result
                    .unwrap_err()
                    .to_string()
                    .contains("No active terminal pane")
        );

        // But get_context_with_fallback should return default context
        let result = state.context_service.get_context_with_fallback(None).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(
            context.current_working_directory,
            Some(expected_fallback_cwd())
        );
        assert!(!context.shell_integration_enabled);
    }

    #[tokio::test]
    async fn test_context_service_integration() {
        let state = create_test_state();
        let pane_id = PaneId::new(123);

        state
            .registry
            .terminal_context_set_active_pane(pane_id)
            .unwrap();

        // Test getting active terminal context (should fail because pane doesn't exist in mux)
        let result = state.context_service.get_active_context().await;
        if result.is_err() {
            let error_msg = result.unwrap_err().to_string();
            let error_msg_lower = error_msg.to_lowercase();
            assert!(
                error_msg_lower.contains("pane does not exist")
                    || error_msg_lower.contains("pane")
                    || error_msg_lower.contains("active")
                    || error_msg_lower.contains("failed to query terminal context")
            );
        } else {
            panic!("Expected error for non-existent pane");
        }

        // Test using fallback logic
        let result = state
            .context_service
            .get_context_with_fallback(Some(pane_id))
            .await;
        assert!(result.is_ok());

        let context = result.unwrap();
        // Since pane doesn't exist, should fallback to default context
        assert_eq!(
            context.current_working_directory,
            Some(expected_fallback_cwd())
        );
    }
}
