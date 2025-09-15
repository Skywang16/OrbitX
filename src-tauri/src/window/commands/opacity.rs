/*!
 * 窗口透明度相关命令
 *
 * 负责窗口透明度的设置和获取功能
 */

use super::*;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};

/// 设置窗口透明度
#[tauri::command]
pub async fn window_set_opacity<R: Runtime>(
    opacity: f64,
    app: AppHandle<R>,
    state: State<'_, WindowState>,
    config_state: State<'_, crate::config::ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    debug!("设置窗口透明度: {}", opacity);

    // 验证透明度值范围
    if !(0.0..=1.0).contains(&opacity) {
        return Ok(api_error!("window.opacity_out_of_range"));
    }

    // 获取窗口实例
    let window_id = state
        .with_config_manager(|config| Ok(config.get_default_window_id().to_string()))
        .await
        .to_tauri()?;

    let window = match app.get_webview_window(&window_id) {
        Some(window) => window,
        None => return Ok(api_error!("window.get_instance_failed")),
    };

    // 设置整体透明度
    let script = format!("document.documentElement.style.opacity = '{}';", opacity);

    match window.eval(&script) {
        Ok(_) => (),
        Err(e) => {
            error!("设置窗口透明度失败: {}", e);
            return Ok(api_error!("window.set_opacity_failed"));
        }
    }

    // 使用统一的配置更新风格
    match config_state
        .toml_manager
        .config_update(|config| {
            config.appearance.opacity = opacity;
            Ok(())
        })
        .await
    {
        Ok(_) => (),
        Err(e) => {
            error!("保存透明度配置失败: {}", e);
            return Ok(api_error!("config.save_failed"));
        }
    }

    debug!("窗口透明度设置成功并已保存到配置: {}", opacity);
    Ok(api_success!())
}

/// 获取窗口透明度
#[tauri::command]
pub async fn window_get_opacity(
    config_state: State<'_, crate::config::ConfigManagerState>,
) -> TauriApiResult<f64> {
    debug!("获取窗口透明度");

    // 从配置文件获取当前透明度
    match config_state.toml_manager.config_get().await {
        Ok(config) => {
            let opacity = config.appearance.opacity;
            debug!("当前窗口透明度: {}", opacity);
            Ok(api_success!(opacity))
        }
        Err(e) => {
            error!("获取透明度配置失败: {}", e);
            Ok(api_error!("config.get_failed"))
        }
    }
}
