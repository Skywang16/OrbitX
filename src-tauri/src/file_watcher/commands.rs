use std::sync::Arc;

use tauri::{AppHandle, Runtime, State};

use crate::utils::TauriApiResult;
use crate::{api_error, api_success};

use super::{FileWatcherConfig, UnifiedFileWatcher, WatcherStatus};

#[tauri::command]
pub async fn file_watcher_start<R: Runtime>(
    app_handle: AppHandle<R>,
    watcher: State<'_, Arc<UnifiedFileWatcher>>,
    path: String,
    config: Option<FileWatcherConfig>,
) -> TauriApiResult<WatcherStatus> {
    if path.trim().is_empty() {
        return Ok(api_error!("common.invalid_path"));
    }

    let cfg = config.unwrap_or_default();
    match watcher.start(app_handle, path, cfg).await {
        Ok(status) => Ok(api_success!(status)),
        Err(e) => Ok(api_error!(e.as_str())),
    }
}

#[tauri::command]
pub async fn file_watcher_stop(watcher: State<'_, Arc<UnifiedFileWatcher>>) -> TauriApiResult<()> {
    watcher.stop().await;
    Ok(api_success!(()))
}

#[tauri::command]
pub async fn file_watcher_status(
    watcher: State<'_, Arc<UnifiedFileWatcher>>,
) -> TauriApiResult<WatcherStatus> {
    Ok(api_success!(watcher.status().await))
}
