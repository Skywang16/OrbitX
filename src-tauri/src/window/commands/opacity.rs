// 窗口透明度相关命令

use super::*;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};

// 设置窗口透明度
#[tauri::command]
pub async fn window_set_opacity<R: Runtime>(
    opacity: f64,
    app: AppHandle<R>,
    state: State<'_, WindowState>,
    config_state: State<'_, crate::config::ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    if !(0.0..=1.0).contains(&opacity) {
        return Ok(api_error!("window.opacity_out_of_range"));
    }

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

    let script = format!("document.documentElement.style.opacity = '{}';", opacity);

    match window.eval(&script) {
        Ok(_) => (),
        Err(_) => {
            return Ok(api_error!("window.set_opacity_failed"));
        }
    }

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
