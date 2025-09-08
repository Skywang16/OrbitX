/*!
 * 窗口状态管理相关命令
 *
 * 负责窗口状态的批量管理、置顶状态控制等功能
 */

use super::*;
use crate::utils::{ApiResponse, TauriApiResult};

/// 批量窗口状态管理命令
///
#[tauri::command]
pub async fn manage_window_state<R: Runtime>(
    request: WindowStateBatchRequest,
    app: AppHandle<R>,
    state: State<'_, WindowState>,
) -> TauriApiResult<WindowStateBatchResponse> {
    let start_time = Instant::now();
    debug!(
        "开始批量窗口状态管理: operations_count={}",
        request.operations.len()
    );

    let mut results = Vec::new();
    let mut overall_success = true;

    // 获取默认窗口ID
    let window_id = state
        .with_config_manager(|config| Ok(config.get_default_window_id().to_string()))
        .await
        .to_tauri()?;

    debug!("使用窗口ID: {}", window_id);

    // 获取窗口实例
    let window = app
        .get_webview_window(&window_id)
        .ok_or_else(|| format!("窗口操作失败 ({}): 无法获取窗口实例", window_id))?;

    // 逐个处理操作
    for operation_request in request.operations {
        let operation_result =
            process_single_window_operation(&operation_request, &window, &state).await;

        if !operation_result.success {
            overall_success = false;
        }

        results.push(operation_result);
    }

    let processing_time = start_time.elapsed().as_millis();

    if overall_success {
        debug!(
            "批量窗口状态管理成功: operations_count={}, 耗时: {}ms",
            results.len(),
            processing_time
        );
    } else {
        warn!(
            "批量窗口状态管理部分失败: operations_count={}, 耗时: {}ms",
            results.len(),
            processing_time
        );
    }

    Ok(ApiResponse::ok(WindowStateBatchResponse {
        results,
        overall_success,
    }))
}

/// 处理单个窗口操作
async fn process_single_window_operation<R: Runtime>(
    request: &WindowStateOperationRequest,
    window: &tauri::WebviewWindow<R>,
    state: &State<'_, WindowState>,
) -> WindowStateOperationResult {
    let operation_start = Instant::now();
    debug!("处理窗口操作: {:?}", request.operation);

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
        Ok(data) => {
            debug!(
                "窗口操作成功: {:?}, 耗时: {}ms",
                request.operation, processing_time
            );
            WindowStateOperationResult {
                operation: request.operation.clone(),
                success: true,
                data: Some(data),
                error: None,
            }
        }
        Err(error) => {
            error!(
                "窗口操作失败: {:?}, 错误: {}, 耗时: {}ms",
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

// ===== 窗口操作处理函数 =====

/// 处理获取窗口状态操作
async fn handle_get_state(state: &State<'_, WindowState>) -> Result<serde_json::Value, String> {
    debug!("处理获取窗口状态操作");

    // 获取当前窗口状态
    let always_on_top = state
        .with_state_manager(|manager| Ok(manager.get_always_on_top()))
        .await
        .to_tauri()?;

    // 获取目录信息
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

    // 获取平台信息
    let platform_info = state
        .with_config_manager(|config| Ok(config.get_platform_info().cloned()))
        .await
        .to_tauri()?
        .unwrap_or_else(|| PlatformInfo {
            platform: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            os_version: "Unknown".to_string(),
            is_mac: cfg!(target_os = "macos"),
        });

    // 构建完整状态
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

    serialize_to_value(&complete_state, "窗口状态")
}

/// 处理设置置顶状态操作
async fn handle_set_always_on_top<R: Runtime>(
    request: &WindowStateOperationRequest,
    window: &tauri::WebviewWindow<R>,
    state: &State<'_, WindowState>,
) -> Result<serde_json::Value, String> {
    debug!("处理设置置顶状态操作");

    let always_on_top = request
        .params
        .as_ref()
        .and_then(|p| p.always_on_top)
        .ok_or_else(|| "设置置顶状态需要alwaysOnTop参数".to_string())?;

    // 设置窗口置顶
    window
        .set_always_on_top(always_on_top)
        .context("设置窗口置顶失败")
        .to_tauri()?;

    // 更新状态管理器
    state
        .with_state_manager_mut(|manager| {
            manager.set_always_on_top(always_on_top);
            Ok(())
        })
        .await
        .to_tauri()?;

    serialize_to_value(&always_on_top, "置顶状态")
}

/// 处理切换置顶状态操作
async fn handle_toggle_always_on_top<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    state: &State<'_, WindowState>,
) -> Result<serde_json::Value, String> {
    debug!("处理切换置顶状态操作");

    // 切换状态管理器中的状态
    let new_state = state
        .with_state_manager_mut(|manager| Ok(manager.toggle_always_on_top()))
        .await
        .to_tauri()?;

    // 设置窗口置顶
    window
        .set_always_on_top(new_state)
        .context("设置窗口置顶失败")
        .to_tauri()?;

    serialize_to_value(&new_state, "切换状态")
}

/// 处理重置窗口状态操作
async fn handle_reset_state<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    state: &State<'_, WindowState>,
) -> Result<serde_json::Value, String> {
    debug!("处理重置窗口状态操作");

    // 重置状态管理器
    state
        .with_state_manager_mut(|manager| {
            manager.reset();
            Ok(())
        })
        .await
        .to_tauri()?;

    // 重置窗口置顶状态
    window
        .set_always_on_top(false)
        .context("重置窗口置顶失败")
        .to_tauri()?;

    // 清除目录缓存
    let _ = state.cache.remove("current_dir").await;
    let _ = state.cache.remove("home_dir").await;

    serialize_to_value(&true, "重置结果")
}
