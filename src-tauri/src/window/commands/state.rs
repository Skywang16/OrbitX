use super::*;
use crate::utils::{ApiResponse, TauriApiResult};
use crate::{api_error, t};

fn serialize_to_value<T: serde::Serialize>(
    value: &T,
    context: &str,
) -> Result<serde_json::Value, String> {
    serde_json::to_value(value).map_err(|e| format!("{} serialization failed: {}", context, e))
}

#[tauri::command]
pub async fn window_manage_state<R: Runtime>(
    request: WindowStateBatchRequest,
    app: AppHandle<R>,
    state: State<'_, WindowState>,
) -> TauriApiResult<WindowStateBatchResponse> {
    let start_time = Instant::now();

    let mut results = Vec::new();
    let mut overall_success = true;

    let window_id = match state
        .with_config_manager(|config| Ok(config.get_default_window_id().to_string()))
        .await
    {
        Ok(id) => id,
        Err(_) => return Ok(api_error!("window.get_window_id_failed")),
    };

    let window = match app.get_webview_window(&window_id) {
        Some(window) => window,
        None => {
            error!("Failed to get window instance: {}", window_id);
            return Ok(api_error!("window.get_instance_failed"));
        }
    };

    for operation_request in request.operations {
        let operation_result =
            process_single_window_operation(&operation_request, &window, &state).await;

        if !operation_result.success {
            overall_success = false;
        }

        results.push(operation_result);
    }

    let processing_time = start_time.elapsed().as_millis();

    if !overall_success {
        warn!(
            "Batch window state operations partially failed: operations_count={}, elapsed_ms={}",
            results.len(),
            processing_time
        );
    }

    Ok(ApiResponse::ok(WindowStateBatchResponse {
        results,
        overall_success,
    }))
}

// 处理单个窗口操作
async fn process_single_window_operation<R: Runtime>(
    request: &WindowStateOperationRequest,
    window: &tauri::WebviewWindow<R>,
    state: &State<'_, WindowState>,
) -> WindowStateOperationResult {
    let operation_start = Instant::now();

    let result = match &request.operation {
        WindowStateOperation::GetState => handle_get_state(state).await,
        WindowStateOperation::SetAlwaysOnTop => {
            handle_set_always_on_top(request, window, state).await
        }
        WindowStateOperation::ToggleAlwaysOnTop => handle_toggle_always_on_top(window, state).await,
        WindowStateOperation::ResetState => handle_reset_state(window, state).await,
    };

    let processing_time = operation_start.elapsed().as_millis();

    match result {
        Ok(data) => WindowStateOperationResult {
            operation: request.operation.clone(),
            success: true,
            data: Some(data),
            error: None,
        },
        Err(error) => {
            error!(
                "Window operation failed: {:?}, error: {}, elapsed_ms: {}",
                request.operation, error, processing_time
            );
            WindowStateOperationResult {
                operation: request.operation.clone(),
                success: false,
                data: None,
                error: Some(error),
            }
        }
    }
}

// 处理获取窗口状态操作
async fn handle_get_state(state: &State<'_, WindowState>) -> Result<serde_json::Value, String> {
    let always_on_top = state
        .with_state_manager(|manager| Ok(manager.get_always_on_top()))
        .await
        .map_err(|_| t!("window.get_state_failed"))?;

    let current_directory = if let Some(cached_dir) = state.cache.get("current_dir").await {
        cached_dir.as_str().unwrap_or("/").to_string()
    } else {
        env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/".to_string())
    };

    let home_directory = if let Some(cached_dir) = state.cache.get("home_dir").await {
        cached_dir.as_str().unwrap_or("/").to_string()
    } else {
        env::var("HOME")
            .or_else(|_| env::current_dir().map(|p| p.to_string_lossy().to_string()))
            .unwrap_or_else(|_| "/".to_string())
    };

    let platform_info = state
        .with_config_manager(|config| Ok(config.window_get_platform_info().cloned()))
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| PlatformInfo {
            platform: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            os_version: "Unknown".to_string(),
            is_mac: cfg!(target_os = "macos"),
        });

    let complete_state = CompleteWindowState {
        always_on_top,
        current_directory,
        home_directory,
        platform_info,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    serialize_to_value(&complete_state, "window state payload")
        .map_err(|_| t!("window.get_state_failed"))
}

// handle the "set always on top" operation
async fn handle_set_always_on_top<R: Runtime>(
    request: &WindowStateOperationRequest,
    window: &tauri::WebviewWindow<R>,
    state: &State<'_, WindowState>,
) -> Result<serde_json::Value, String> {
    let always_on_top = match request.params.as_ref().and_then(|p| p.always_on_top) {
        Some(value) => value,
        None => return Err(t!("window.missing_always_on_top_param")),
    };

    if window.set_always_on_top(always_on_top).is_err() {
        return Err(t!("window.set_always_on_top_failed"));
    }

    if state
        .with_state_manager_mut(|manager| {
            manager.set_always_on_top(always_on_top);
            Ok(())
        })
        .await
        .is_err()
    {
        return Err(t!("window.set_always_on_top_failed"));
    }

    serialize_to_value(&always_on_top, "always on top flag")
        .map_err(|_| t!("window.set_always_on_top_failed"))
}

// handle toggle always-on-top
async fn handle_toggle_always_on_top<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    state: &State<'_, WindowState>,
) -> Result<serde_json::Value, String> {
    let new_state = state
        .with_state_manager_mut(|manager| Ok(manager.toggle_always_on_top()))
        .await
        .map_err(|_| t!("window.toggle_always_on_top_failed"))?;

    if window.set_always_on_top(new_state).is_err() {
        return Err(t!("window.toggle_always_on_top_failed"));
    }

    serialize_to_value(&new_state, "toggle result")
        .map_err(|_| t!("window.toggle_always_on_top_failed"))
}

// handle resetting window state
async fn handle_reset_state<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    state: &State<'_, WindowState>,
) -> Result<serde_json::Value, String> {
    if state
        .with_state_manager_mut(|manager| {
            manager.reset();
            Ok(())
        })
        .await
        .is_err()
    {
        return Err(t!("window.reset_state_failed"));
    }

    if window.set_always_on_top(false).is_err() {
        return Err(t!("window.reset_state_failed"));
    }

    let _ = state.cache.remove("current_dir").await;
    let _ = state.cache.remove("home_dir").await;

    serialize_to_value(&true, "reset result").map_err(|_| t!("window.reset_state_failed"))
}
