/*!
 * 窗口透明度相关命令
 *
 * 负责窗口透明度的设置和获取功能
 */

use super::*;
use crate::utils::{ApiResponse, EmptyData as _EmptyDataAlias};
use crate::utils::{EmptyData, TauriApiResult};

/// 设置窗口透明度
#[tauri::command]
pub async fn set_window_opacity<R: Runtime>(
    opacity: f64,
    app: AppHandle<R>,
    state: State<'_, WindowState>,
    config_state: State<'_, crate::config::ConfigManagerState>,
) -> TauriApiResult<EmptyData> {
    debug!("设置窗口透明度: {}", opacity);

    // 验证透明度值范围
    if !(0.0..=1.0).contains(&opacity) {
        return Err("透明度值必须在 0.0 到 1.0 之间".to_string());
    }

    // 获取窗口实例
    let window_id = state
        .with_config_manager(|config| Ok(config.get_default_window_id().to_string()))
        .await
        .to_tauri()?;

    let window = app
        .get_webview_window(&window_id)
        .ok_or_else(|| "无法获取窗口实例".to_string())?;

    // 设置整体透明度
    let script = format!("document.documentElement.style.opacity = '{}';", opacity);

    window
        .eval(&script)
        .context("设置窗口透明度失败")
        .to_tauri()?;

    // 使用统一的配置更新风格
    config_state
        .toml_manager
        .update_config(|config| {
            config.appearance.opacity = opacity;
            Ok(())
        })
        .await
        .to_tauri()?;

    debug!("窗口透明度设置成功并已保存到配置: {}", opacity);
    Ok(ApiResponse::ok(_EmptyDataAlias::default()))
}

/// 获取窗口透明度
#[tauri::command]
pub async fn get_window_opacity(
    config_state: State<'_, crate::config::ConfigManagerState>,
) -> TauriApiResult<f64> {
    debug!("获取窗口透明度");

    // 从配置文件获取当前透明度
    let config = config_state.toml_manager.get_config().await.to_tauri()?;
    let opacity = config.appearance.opacity;

    debug!("当前窗口透明度: {}", opacity);
    Ok(ApiResponse::ok(opacity))
}
