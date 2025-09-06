/*!
 * 目录和路径操作相关命令
 *
 * 负责当前目录获取、家目录获取、路径处理等功能
 */

use super::*;

/// 获取当前目录
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn get_current_directory(
    use_cache: Option<bool>,
    state: State<'_, WindowState>,
) -> Result<String, String> {
    let use_cache = use_cache.unwrap_or(true);
    debug!("开始获取当前目录: use_cache={}", use_cache);

    // 如果启用缓存，先尝试从缓存获取
    if use_cache {
        if let Some(cached_dir) = state.cache.get("current_dir").await {
            if let Some(dir) = cached_dir.as_str() {
                debug!("从缓存获取当前目录: {}", dir);
                return Ok(dir.to_string());
            }
        }
    }

    debug!("从系统获取当前目录");

    // 尝试获取当前目录
    let current_dir = env::current_dir()
        .map_err(|e| format!("目录操作失败: 获取当前目录失败: {}", e))?
        .to_string_lossy()
        .to_string();

    debug!("系统当前目录: {}", current_dir);

    // 更新缓存
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
    Ok(current_dir)
}

/// 获取用户家目录
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn get_home_directory(
    use_cache: Option<bool>,
    state: State<'_, WindowState>,
) -> Result<String, String> {
    let use_cache = use_cache.unwrap_or(true);
    debug!("开始获取家目录: use_cache={}", use_cache);

    // 如果启用缓存，先尝试从缓存获取
    if use_cache {
        if let Some(cached_dir) = state.cache.get("home_dir").await {
            if let Some(dir) = cached_dir.as_str() {
                debug!("从缓存获取家目录: {}", dir);
                return Ok(dir.to_string());
            }
        }
    }

    debug!("从系统获取家目录");

    // 尝试获取家目录，优先使用平台特定的环境变量
    let home_dir = if cfg!(windows) {
        // Windows平台优先使用USERPROFILE，然后是HOME
        env::var("USERPROFILE")
            .or_else(|_| env::var("HOME"))
            .or_else(|_| {
                // 如果都不存在，尝试获取当前目录
                env::current_dir().map(|p| p.to_string_lossy().to_string())
            })
    } else {
        // Unix平台使用HOME
        env::var("HOME").or_else(|_| {
            // 如果HOME环境变量不存在，尝试获取当前目录
            env::current_dir().map(|p| p.to_string_lossy().to_string())
        })
    }
    .map_err(|e| format!("目录操作失败: 获取家目录失败: {}", e))?;

    debug!("系统家目录: {}", home_dir);

    // 更新缓存
    if let Err(e) = state
        .cache
        .set("home_dir", serde_json::Value::String(home_dir.clone()))
        .await
    {
        warn!("更新目录缓存失败: {}", e);
    }

    debug!("家目录获取成功: {}", home_dir);
    Ok(home_dir)
}

/// 清除目录缓存
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn clear_directory_cache(state: State<'_, WindowState>) -> Result<(), String> {
    debug!("开始清除目录缓存");

    // 清除目录相关的缓存
    let _ = state.cache.remove("current_dir").await;
    let _ = state.cache.remove("home_dir").await;

    debug!("目录缓存清除成功");
    Ok(())
}

// ===== 路径处理工具命令 =====

/// 规范化路径
#[tauri::command]
pub async fn normalize_path(path: String) -> Result<String, String> {
    debug!("开始规范化路径: {}", path);

    if path.is_empty() {
        return Err("输入验证错误: 路径不能为空".to_string());
    }

    let normalized = Path::new(&path)
        .canonicalize()
        .map_err(|e| format!("路径处理失败 ({}): 路径规范化失败: {}", path, e))?
        .to_string_lossy()
        .to_string();

    debug!("路径规范化成功: {} -> {}", path, normalized);
    Ok(normalized)
}

/// 连接路径
#[tauri::command]
pub async fn join_paths(paths: Vec<String>) -> Result<String, String> {
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
    Ok(joined)
}

/// 检查路径是否存在
#[tauri::command]
pub async fn path_exists(path: String) -> Result<bool, String> {
    debug!("开始检查路径是否存在: {}", path);

    if path.is_empty() {
        return Err("输入验证错误: 路径不能为空".to_string());
    }

    let exists = Path::new(&path).exists();
    debug!("路径存在性检查完成: {} -> {}", path, exists);
    Ok(exists)
}
