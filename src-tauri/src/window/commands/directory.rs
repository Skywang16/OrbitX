// 目录和路径操作相关命令

use super::*;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};

// 获取当前目录
#[tauri::command]
pub async fn window_get_current_directory(
    use_cache: Option<bool>,
    state: State<'_, WindowState>,
) -> TauriApiResult<String> {
    let use_cache = use_cache.unwrap_or(true);
    debug!("开始获取当前目录: use_cache={}", use_cache);

    if use_cache {
        if let Some(cached_dir) = state.cache.get("current_dir").await {
            if let Some(dir) = cached_dir.as_str() {
                debug!("从缓存获取当前目录: {}", dir);
                return Ok(api_success!(dir.to_string()));
            }
        }
    }

    debug!("从系统获取当前目录");

    let current_dir = match env::current_dir() {
        Ok(dir) => dir.to_string_lossy().to_string(),
        Err(_) => return Ok(api_error!("window.get_current_directory_failed")),
    };

    debug!("系统当前目录: {}", current_dir);

    if let Err(e) = state
        .cache
        .set(
            "current_dir",
            serde_json::Value::String(current_dir.clone()),
        )
        .await
    {
        warn!("更新目录缓存失败: {}", e);
    }

    debug!("当前目录获取成功: {}", current_dir);
    Ok(api_success!(current_dir))
}

// 获取用户家目录
#[tauri::command]
pub async fn window_get_home_directory(
    use_cache: Option<bool>,
    state: State<'_, WindowState>,
) -> TauriApiResult<String> {
    let use_cache = use_cache.unwrap_or(true);
    debug!("开始获取家目录: use_cache={}", use_cache);

    if use_cache {
        if let Some(cached_dir) = state.cache.get("home_dir").await {
            if let Some(dir) = cached_dir.as_str() {
                debug!("从缓存获取家目录: {}", dir);
                return Ok(api_success!(dir.to_string()));
            }
        }
    }

    debug!("从系统获取家目录");

    let home_dir = if cfg!(windows) {
        env::var("USERPROFILE")
            .or_else(|_| env::var("HOME"))
            .or_else(|_| {
                env::current_dir().map(|p| p.to_string_lossy().to_string())
            })
    } else {
        env::var("HOME").or_else(|_| {
            env::current_dir().map(|p| p.to_string_lossy().to_string())
        })
    }
    .context("获取家目录失败")
    .to_tauri()?;

    debug!("系统家目录: {}", home_dir);

    if let Err(e) = state
        .cache
        .set("home_dir", serde_json::Value::String(home_dir.clone()))
        .await
    {
        warn!("更新目录缓存失败: {}", e);
    }

    debug!("家目录获取成功: {}", home_dir);
    Ok(api_success!(home_dir))
}

// 清除目录缓存
#[tauri::command]
pub async fn window_clear_directory_cache(
    state: State<'_, WindowState>,
) -> TauriApiResult<EmptyData> {
    debug!("开始清除目录缓存");

    let _ = state.cache.remove("current_dir").await;
    let _ = state.cache.remove("home_dir").await;

    debug!("目录缓存清除成功");
    Ok(api_success!())
}


// 规范化路径
#[tauri::command]
pub async fn window_normalize_path(path: String) -> TauriApiResult<String> {
    debug!("开始规范化路径: {}", path);

    if path.is_empty() {
        return Err("输入验证错误: 路径不能为空".to_string());
    }

    let normalized = Path::new(&path)
        .canonicalize()
        .with_context(|| format!("路径规范化失败: {}", path))
        .to_tauri()?
        .to_string_lossy()
        .to_string();

    debug!("路径规范化成功: {} -> {}", path, normalized);
    Ok(api_success!(normalized))
}

// 连接路径
#[tauri::command]
pub async fn window_join_paths(paths: Vec<String>) -> TauriApiResult<String> {
    debug!("开始连接路径: {:?}", paths);

    if paths.is_empty() {
        return Err("输入验证错误: 路径列表不能为空".to_string());
    }

    let mut result = PathBuf::new();
    for path in &paths {
        if path.is_empty() {
            return Err("输入验证错误: 路径组件不能为空".to_string());
        }
        result.push(path);
    }

    let joined = result.to_string_lossy().to_string();
    debug!("路径连接成功: {:?} -> {}", paths, joined);
    Ok(api_success!(joined))
}

// 检查路径是否存在
#[tauri::command]
pub async fn window_path_exists(path: String) -> TauriApiResult<bool> {
    debug!("开始检查路径是否存在: {}", path);

    if path.is_empty() {
        return Err("输入验证错误: 路径不能为空".to_string());
    }

    let exists = Path::new(&path).exists();
    debug!("路径存在性检查完成: {} -> {}", path, exists);
    Ok(api_success!(exists))
}
