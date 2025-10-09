// Directory and path related window commands

use super::*;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use tracing::{debug, warn};

// 获取当前目录
#[tauri::command]
pub async fn window_get_current_directory(
    use_cache: Option<bool>,
    state: State<'_, WindowState>,
) -> TauriApiResult<String> {
    let use_cache = use_cache.unwrap_or(true);
    debug!("Fetching current directory (use_cache={})", use_cache);

    if use_cache {
        if let Some(cached_dir) = state.cache.get("current_dir").await {
            if let Some(dir) = cached_dir.as_str() {
                debug!("Resolved current directory from cache: {}", dir);
                return Ok(api_success!(dir.to_string()));
            }
        }
    }

    debug!("Querying current directory from OS");

    let current_dir = match env::current_dir() {
        Ok(dir) => dir.to_string_lossy().to_string(),
        Err(_) => return Ok(api_error!("window.get_current_directory_failed")),
    };

    debug!("Current directory reported by OS: {}", current_dir);

    if let Err(e) = state
        .cache
        .set(
            "current_dir",
            serde_json::Value::String(current_dir.clone()),
        )
        .await
    {
        warn!("Failed to update current directory cache: {}", e);
    }

    debug!("Current directory lookup succeeded: {}", current_dir);
    Ok(api_success!(current_dir))
}

// 获取用户家目录
#[tauri::command]
pub async fn window_get_home_directory(
    use_cache: Option<bool>,
    state: State<'_, WindowState>,
) -> TauriApiResult<String> {
    let use_cache = use_cache.unwrap_or(true);
    debug!("Fetching home directory (use_cache={})", use_cache);

    if use_cache {
        if let Some(cached_dir) = state.cache.get("home_dir").await {
            if let Some(dir) = cached_dir.as_str() {
                debug!("Resolved home directory from cache: {}", dir);
                return Ok(api_success!(dir.to_string()));
            }
        }
    }

    debug!("Querying home directory from OS");

    let home_dir = if cfg!(windows) {
        env::var("USERPROFILE")
            .or_else(|_| env::var("HOME"))
            .or_else(|_| env::current_dir().map(|p| p.to_string_lossy().to_string()))
    } else {
        env::var("HOME").or_else(|_| env::current_dir().map(|p| p.to_string_lossy().to_string()))
    };

    let home_dir = match home_dir {
        Ok(dir) => dir,
        Err(_) => {
            return Ok(api_error!("window.get_home_directory_failed"));
        }
    };

    debug!("Home directory reported by OS: {}", home_dir);

    if let Err(e) = state
        .cache
        .set("home_dir", serde_json::Value::String(home_dir.clone()))
        .await
    {
        warn!("Failed to update home directory cache: {}", e);
    }

    debug!("Home directory lookup succeeded: {}", home_dir);
    Ok(api_success!(home_dir))
}

// 清除目录缓存
#[tauri::command]
pub async fn window_clear_directory_cache(
    state: State<'_, WindowState>,
) -> TauriApiResult<EmptyData> {
    debug!("Clearing directory cache entries");

    let _ = state.cache.remove("current_dir").await;
    let _ = state.cache.remove("home_dir").await;

    debug!("Directory cache cleared");
    Ok(api_success!())
}

// 规范化路径
#[tauri::command]
pub async fn window_normalize_path(path: String) -> TauriApiResult<String> {
    debug!("Normalising path: {}", path);

    if path.trim().is_empty() {
        return Ok(api_error!("common.path_empty"));
    }

    let normalized = match Path::new(&path).canonicalize() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => {
            return Ok(api_error!("window.normalize_path_failed"));
        }
    };

    debug!("Normalised path {} -> {}", path, normalized);
    Ok(api_success!(normalized))
}

// 连接路径
#[tauri::command]
pub async fn window_join_paths(paths: Vec<String>) -> TauriApiResult<String> {
    debug!("Joining path segments: {:?}", paths);

    if paths.is_empty() {
        return Ok(api_error!("window.path_list_empty"));
    }

    let mut result = PathBuf::new();
    for path in &paths {
        if path.trim().is_empty() {
            return Ok(api_error!("window.path_component_empty"));
        }
        result.push(path);
    }

    let joined = result.to_string_lossy().to_string();
    debug!("Joined path {:?} -> {}", paths, joined);
    Ok(api_success!(joined))
}

// 检查路径是否存在
#[tauri::command]
pub async fn window_path_exists(path: String) -> TauriApiResult<bool> {
    debug!("Checking path existence: {}", path);

    if path.trim().is_empty() {
        return Ok(api_error!("common.path_empty"));
    }

    let exists = Path::new(&path).exists();
    debug!("Path existence check {} -> {}", path, exists);
    Ok(api_success!(exists))
}
