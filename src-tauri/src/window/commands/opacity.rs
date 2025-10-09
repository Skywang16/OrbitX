// 窗口透明度相关命令

use super::*;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use serde::Serialize;
use tauri::Emitter;

// 透明度变化事件的 payload
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpacityChangedPayload {
    pub opacity: f64,
}

// 设置窗口透明度
#[tauri::command]
pub async fn window_set_opacity<R: Runtime>(
    opacity: f64,
    app: AppHandle<R>,
    state: State<'_, WindowState>,
    config_state: State<'_, crate::config::ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    // 1. 验证输入范围
    if !(0.05..=1.0).contains(&opacity) {
        return Ok(api_error!("window.opacity_out_of_range"));
    }

    // 2. 获取窗口实例
    let window_id = match state
        .with_config_manager(|config| Ok(config.get_default_window_id().to_string()))
        .await
    {
        Ok(id) => id,
        Err(_) => return Ok(api_error!("window.get_window_id_failed")),
    };

    let window = match app.get_webview_window(&window_id) {
        Some(window) => window,
        None => return Ok(api_error!("window.get_instance_failed")),
    };

    // 3. 持久化配置
    match config_state
        .toml_manager
        .config_update(|config| {
            config.appearance.opacity = opacity;
            Ok(())
        })
        .await
    {
        Ok(_) => (),
        Err(_) => {
            return Ok(api_error!("config.save_failed"));
        }
    }

    // 4. 发送事件通知前端
    let payload = OpacityChangedPayload { opacity };
    match window.emit("opacity-changed", payload) {
        Ok(_) => (),
        Err(_) => {
            return Ok(api_error!("window.emit_event_failed"));
        }
    }

    Ok(api_success!())
}

// 获取窗口透明度
#[tauri::command]
pub async fn window_get_opacity(
    config_state: State<'_, crate::config::ConfigManagerState>,
) -> TauriApiResult<f64> {
    match config_state.toml_manager.config_get().await {
        Ok(config) => {
            let opacity = config.appearance.opacity;

            Ok(api_success!(opacity))
        }
        Err(_) => Ok(api_error!("config.get_failed")),
    }
}
