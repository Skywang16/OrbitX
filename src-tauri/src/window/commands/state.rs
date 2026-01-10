use super::*;
use crate::utils::TauriApiResult;
use crate::{api_error, api_success};
use serde::Serialize;
use tauri::Emitter;
use tracing::warn;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpacityChangedPayload {
    pub opacity: f64,
}

#[tauri::command]
pub async fn window_state_get(
    refresh: Option<bool>,
    state: State<'_, WindowState>,
    config_state: State<'_, crate::config::ConfigManagerState>,
) -> TauriApiResult<WindowStateSnapshot> {
    let refresh = refresh.unwrap_or(false);

    if refresh {
        let _ = state.cache.remove("current_dir").await;
        let _ = state.cache.remove("home_dir").await;
    }

    let always_on_top = match state
        .with_state_manager(|manager| Ok(manager.get_always_on_top()))
        .await
    {
        Ok(value) => value,
        Err(_) => return Ok(api_error!("window.get_state_failed")),
    };

    let current_directory = if let Some(cached_dir) = state.cache.get("current_dir").await {
        cached_dir.as_str().unwrap_or("/").to_string()
    } else {
        let dir = env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/".to_string());
        if let Err(e) = state
            .cache
            .set("current_dir", serde_json::Value::String(dir.clone()))
            .await
        {
            warn!("Failed to update current directory cache: {}", e);
        }
        dir
    };

    let home_directory = if let Some(cached_dir) = state.cache.get("home_dir").await {
        cached_dir.as_str().unwrap_or("/").to_string()
    } else {
        let dir = env::var("HOME")
            .or_else(|_| env::current_dir().map(|p| p.to_string_lossy().to_string()))
            .unwrap_or_else(|_| "/".to_string());
        if let Err(e) = state
            .cache
            .set("home_dir", serde_json::Value::String(dir.clone()))
            .await
        {
            warn!("Failed to update home directory cache: {}", e);
        }
        dir
    };

    let platform_info = match state
        .with_config_manager(|config| Ok(config.window_get_platform_info().cloned()))
        .await
    {
        Ok(Some(info)) => info,
        _ => {
            let info = PlatformInfo {
                platform: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
                os_version: detect_os_version(),
                is_mac: cfg!(target_os = "macos"),
            };
            let _ = state
                .with_config_manager_mut(|config| {
                    config.set_platform_info(info.clone());
                    Ok(())
                })
                .await;
            info
        }
    };

    let opacity = match config_state.toml_manager.config_get().await {
        Ok(config) => config.appearance.opacity,
        Err(_) => return Ok(api_error!("config.get_failed")),
    };

    Ok(api_success!(WindowStateSnapshot {
        always_on_top,
        current_directory,
        home_directory,
        platform_info,
        opacity,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }))
}

#[tauri::command]
pub async fn window_state_update<R: Runtime>(
    update: WindowStateUpdate,
    app: AppHandle<R>,
    state: State<'_, WindowState>,
    config_state: State<'_, crate::config::ConfigManagerState>,
) -> TauriApiResult<WindowStateSnapshot> {
    let needs_window = update.always_on_top.is_some() || update.opacity.is_some();
    let window = if needs_window {
        let window_id = match state
            .with_config_manager(|config| Ok(config.get_default_window_id().to_string()))
            .await
        {
            Ok(id) => id,
            Err(_) => return Ok(api_error!("window.get_window_id_failed")),
        };

        match app.get_webview_window(&window_id) {
            Some(window) => Some(window),
            None => return Ok(api_error!("window.get_instance_failed")),
        }
    } else {
        None
    };

    if let Some(always_on_top) = update.always_on_top {
        if let Some(window) = &window {
            if window.set_always_on_top(always_on_top).is_err() {
                return Ok(api_error!("window.set_always_on_top_failed"));
            }
        }

        if state
            .with_state_manager_mut(|manager| {
                manager.set_always_on_top(always_on_top);
                Ok(())
            })
            .await
            .is_err()
        {
            return Ok(api_error!("window.set_always_on_top_failed"));
        }
    }

    if let Some(opacity) = update.opacity {
        if !(0.05..=1.0).contains(&opacity) {
            return Ok(api_error!("window.opacity_out_of_range"));
        }

        if config_state
            .toml_manager
            .config_update(|config| {
                config.appearance.opacity = opacity;
                Ok(())
            })
            .await
            .is_err()
        {
            return Ok(api_error!("config.save_failed"));
        }

        if let Some(window) = &window {
            if window
                .emit("opacity-changed", OpacityChangedPayload { opacity })
                .is_err()
            {
                return Ok(api_error!("window.emit_event_failed"));
            }
        }
    }

    if update.refresh_directories.unwrap_or(false) {
        let _ = state.cache.remove("current_dir").await;
        let _ = state.cache.remove("home_dir").await;
    }

    window_state_get(Some(false), state, config_state).await
}

fn detect_os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
        {
            if let Ok(version) = String::from_utf8(output.stdout) {
                return version.trim().to_string();
            }
        }
        "macOS Unknown".to_string()
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = std::fs::read_to_string("/etc/os-release") {
            for line in contents.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    let version = line.trim_start_matches("PRETTY_NAME=").trim_matches('\"');
                    return version.to_string();
                }
            }
        }
        "Linux Unknown".to_string()
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("cmd")
            .args(&["/C", "ver"])
            .output()
        {
            if let Ok(version) = String::from_utf8(output.stdout) {
                return version.trim().to_string();
            }
        }
        "Windows Unknown".to_string()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        "Unknown OS".to_string()
    }
}
