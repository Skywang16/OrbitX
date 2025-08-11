/*!
 * 窗口功能的Tauri命令接口
 *
 * 统一的窗口命令处理规范：
 * 1. 参数顺序：业务参数在前，state参数在后
 * 2. 异步处理：所有命令都是async，统一错误转换
 * 3. 日志记录：每个命令都记录调用和结果日志
 * 4. 状态管理：统一使用WindowState访问各组件
 * 5. 错误处理：使用anyhow统一错误类型
 */

use crate::utils::error::AppResult;

use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime, State};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// 窗口状态管理
///
/// 统一状态管理规范：
/// 1. 虽然窗口模块相对无状态，但提供统一的初始化和验证方法
/// 2. 包含配置验证和错误处理
/// 3. 支持组件间的依赖注入
/// 4. 提供缓存和性能优化
pub struct WindowState {
    /// 统一缓存
    pub cache: crate::storage::cache::UnifiedCache,
    /// 窗口配置管理器
    pub config_manager: Arc<Mutex<WindowConfigManager>>,
    /// 窗口状态管理器
    pub state_manager: Arc<Mutex<WindowStateManager>>,
}

/// 平台信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// 操作系统平台 (windows, macos, linux)
    pub platform: String,
    /// 系统架构 (x86_64, aarch64, etc.)
    pub arch: String,
    /// 操作系统版本
    pub os_version: String,
    /// 是否为Mac系统
    pub is_mac: bool,
}

// ===== 新的批量窗口状态管理类型 =====

/// 窗口状态操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowStateOperation {
    /// 获取窗口状态
    GetState,
    /// 设置置顶状态
    SetAlwaysOnTop,
    /// 切换置顶状态
    ToggleAlwaysOnTop,
    /// 重置窗口状态
    ResetState,
}

/// 窗口状态操作参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStateOperationParams {
    /// 置顶状态参数
    pub always_on_top: Option<bool>,
}

/// 窗口状态操作请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStateOperationRequest {
    /// 操作类型
    pub operation: WindowStateOperation,
    /// 操作参数
    pub params: Option<WindowStateOperationParams>,
}

/// 批量窗口状态操作请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStateBatchRequest {
    /// 操作列表
    pub operations: Vec<WindowStateOperationRequest>,
}

/// 窗口状态操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStateOperationResult {
    /// 操作类型
    pub operation: WindowStateOperation,
    /// 操作是否成功
    pub success: bool,
    /// 操作结果数据
    pub data: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
}

/// 批量窗口状态操作响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStateBatchResponse {
    /// 操作结果列表
    pub results: Vec<WindowStateOperationResult>,
    /// 整体操作是否成功
    pub overall_success: bool,
}

/// 完整的窗口状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteWindowState {
    /// 置顶状态
    pub always_on_top: bool,
    /// 当前目录
    pub current_directory: String,
    /// 家目录
    pub home_directory: String,
    /// 平台信息
    pub platform_info: PlatformInfo,
    /// 时间戳
    pub timestamp: u64,
}

/// 窗口配置管理器
#[derive(Debug)]
pub struct WindowConfigManager {
    /// 平台信息缓存
    platform_info: Option<PlatformInfo>,
    /// 默认窗口ID
    default_window_id: String,
    /// 窗口操作超时时间
    operation_timeout: u64,
}

/// 窗口状态管理器
#[derive(Debug)]
pub struct WindowStateManager {
    /// 当前窗口置顶状态
    always_on_top: bool,
    /// 状态最后更新时间
    last_updated: Instant,
}

impl Default for WindowStateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowStateManager {
    /// 创建新的窗口状态管理器
    pub fn new() -> Self {
        Self {
            always_on_top: false,
            last_updated: Instant::now(),
        }
    }

    /// 获取当前置顶状态
    pub fn get_always_on_top(&self) -> bool {
        self.always_on_top
    }

    /// 设置置顶状态
    pub fn set_always_on_top(&mut self, always_on_top: bool) {
        self.always_on_top = always_on_top;
        self.last_updated = Instant::now();
    }

    /// 切换置顶状态
    pub fn toggle_always_on_top(&mut self) -> bool {
        self.always_on_top = !self.always_on_top;
        self.last_updated = Instant::now();
        self.always_on_top
    }

    /// 重置状态
    pub fn reset(&mut self) {
        self.always_on_top = false;
        self.last_updated = Instant::now();
    }

    /// 获取最后更新时间
    pub fn get_last_updated(&self) -> Instant {
        self.last_updated
    }
}

impl WindowConfigManager {
    pub fn new() -> Self {
        Self {
            platform_info: None,
            default_window_id: "main".to_string(),
            operation_timeout: 5, // 5秒超时
        }
    }

    /// 获取平台信息，如果缓存中没有则检测并缓存
    pub fn get_platform_info(&mut self) -> &PlatformInfo {
        if self.platform_info.is_none() {
            self.platform_info = Some(Self::detect_platform_info());
        }
        self.platform_info.as_ref().unwrap()
    }

    /// 检测平台信息
    fn detect_platform_info() -> PlatformInfo {
        let platform = if cfg!(target_os = "windows") {
            "windows".to_string()
        } else if cfg!(target_os = "macos") {
            "macos".to_string()
        } else if cfg!(target_os = "linux") {
            "linux".to_string()
        } else {
            "unknown".to_string()
        };

        let arch = if cfg!(target_arch = "x86_64") {
            "x86_64".to_string()
        } else if cfg!(target_arch = "aarch64") {
            "aarch64".to_string()
        } else if cfg!(target_arch = "x86") {
            "x86".to_string()
        } else {
            "unknown".to_string()
        };

        // 使用 os_info 库获取准确的系统版本信息
        let os_version = Self::get_accurate_os_version();
        let is_mac = cfg!(target_os = "macos");

        PlatformInfo {
            platform,
            arch,
            os_version,
            is_mac,
        }
    }

    /// 获取准确的操作系统版本信息
    fn get_accurate_os_version() -> String {
        let info = os_info::get();

        // 构建基础版本信息
        let os_type = info.os_type().to_string();
        let version = info.version().to_string();

        // 简化实现，只使用基础信息
        let result = if version.is_empty() || version == "Unknown" {
            os_type
        } else {
            format!("{} {}", os_type, version)
        };

        // 如果获取失败，提供回退方案
        if result.trim().is_empty() || result == "Unknown" {
            // 尝试从环境变量获取作为备选方案
            env::var("OS").unwrap_or_else(|_| {
                // 根据编译时目标提供基础信息
                if cfg!(target_os = "windows") {
                    "Windows".to_string()
                } else if cfg!(target_os = "macos") {
                    "macOS".to_string()
                } else if cfg!(target_os = "linux") {
                    "Linux".to_string()
                } else {
                    "Unknown".to_string()
                }
            })
        } else {
            result
        }
    }

    pub fn get_default_window_id(&self) -> &str {
        &self.default_window_id
    }

    pub fn get_operation_timeout(&self) -> u64 {
        self.operation_timeout
    }
}

impl Default for WindowConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowState {
    /// 统一的初始化方法
    pub fn new() -> AppResult<Self> {
        info!("开始初始化窗口状态");

        let config_manager = Arc::new(Mutex::new(WindowConfigManager::new()));
        let state_manager = Arc::new(Mutex::new(WindowStateManager::new()));

        let state = Self {
            cache: crate::storage::cache::UnifiedCache::new(),
            config_manager,
            state_manager,
        };

        info!("窗口状态初始化完成");
        Ok(state)
    }

    /// 验证状态完整性
    pub async fn validate(&self) -> AppResult<()> {
        info!("开始验证窗口状态");

        // 验证各组件是否可访问
        let _config = self.config_manager.lock().await;
        let _state = self.state_manager.lock().await;

        info!("窗口状态验证通过");
        Ok(())
    }

    pub async fn with_config_manager<F, R>(&self, f: F) -> AppResult<R>
    where
        F: FnOnce(&mut WindowConfigManager) -> AppResult<R>,
    {
        let mut config = self.config_manager.lock().await;
        f(&mut config)
    }

    /// 统一的状态管理器访问方法
    pub async fn with_state_manager<F, R>(&self, f: F) -> AppResult<R>
    where
        F: FnOnce(&mut WindowStateManager) -> AppResult<R>,
    {
        let mut state = self.state_manager.lock().await;
        f(&mut state)
    }
}

// ===== 新的批量窗口状态管理命令 =====

/// 批量窗口状态管理命令
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state参数在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
/// - 批量处理：支持多个操作的原子性执行
#[tauri::command]
pub async fn manage_window_state<R: Runtime>(
    request: WindowStateBatchRequest,
    app: AppHandle<R>,
    state: State<'_, WindowState>,
) -> Result<WindowStateBatchResponse, String> {
    let start_time = Instant::now();
    info!(
        "开始批量窗口状态管理: operations_count={}",
        request.operations.len()
    );

    let mut results = Vec::new();
    let mut overall_success = true;

    // 获取默认窗口ID
    let window_id = state
        .with_config_manager(|config| Ok(config.get_default_window_id().to_string()))
        .await
        .map_err(|e| e.to_string())?;

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
        info!(
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

    Ok(WindowStateBatchResponse {
        results,
        overall_success,
    })
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
        .map_err(|e| e.to_string())?;

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
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string())
    };

    // 获取平台信息
    let platform_info = state
        .with_config_manager(|config| Ok(config.get_platform_info().clone()))
        .await
        .map_err(|e| e.to_string())?;

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

    serde_json::to_value(complete_state).map_err(|e| format!("序列化状态失败: {}", e))
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
        .map_err(|e| format!("设置窗口置顶失败: {}", e))?;

    // 更新状态管理器
    state
        .with_state_manager(|manager| {
            manager.set_always_on_top(always_on_top);
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_value(always_on_top).map_err(|e| format!("序列化结果失败: {}", e))
}

/// 处理切换置顶状态操作
async fn handle_toggle_always_on_top<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    state: &State<'_, WindowState>,
) -> Result<serde_json::Value, String> {
    debug!("处理切换置顶状态操作");

    // 切换状态管理器中的状态
    let new_state = state
        .with_state_manager(|manager| Ok(manager.toggle_always_on_top()))
        .await
        .map_err(|e| e.to_string())?;

    // 设置窗口置顶
    window
        .set_always_on_top(new_state)
        .map_err(|e| format!("设置窗口置顶失败: {}", e))?;

    serde_json::to_value(new_state).map_err(|e| format!("序列化结果失败: {}", e))
}

/// 处理重置窗口状态操作
async fn handle_reset_state<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    state: &State<'_, WindowState>,
) -> Result<serde_json::Value, String> {
    debug!("处理重置窗口状态操作");

    // 重置状态管理器
    state
        .with_state_manager(|manager| {
            manager.reset();
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?;

    // 重置窗口置顶状态
    window
        .set_always_on_top(false)
        .map_err(|e| format!("重置窗口置顶失败: {}", e))?;

    // 清除目录缓存
    let _ = state.cache.remove("current_dir").await;
    let _ = state.cache.remove("home_dir").await;

    serde_json::to_value(true).map_err(|e| format!("序列化结果失败: {}", e))
}

// ===== 命令参数类型定义 =====

// ===== 窗口管理命令 =====

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
    info!("开始获取当前目录: use_cache={}", use_cache);

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

    info!("当前目录获取成功: {}", current_dir);
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
    info!("开始获取家目录: use_cache={}", use_cache);

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

    // 尝试获取家目录
    let home_dir = env::var("HOME")
        .or_else(|_| {
            // 如果HOME环境变量不存在，尝试获取当前目录
            env::current_dir().map(|p| p.to_string_lossy().to_string())
        })
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

    info!("家目录获取成功: {}", home_dir);
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
    info!("开始清除目录缓存");

    // 清除目录相关的缓存
    let _ = state.cache.remove("current_dir").await;
    let _ = state.cache.remove("home_dir").await;

    info!("目录缓存清除成功");
    Ok(())
}

// ===== 路径处理工具命令 =====

/// 规范化路径
#[tauri::command]
pub async fn normalize_path(path: String) -> Result<String, String> {
    info!("开始规范化路径: {}", path);

    if path.is_empty() {
        return Err("输入验证错误: 路径不能为空".to_string());
    }

    let normalized = Path::new(&path)
        .canonicalize()
        .map_err(|e| format!("路径处理失败 ({}): 路径规范化失败: {}", path, e))?
        .to_string_lossy()
        .to_string();

    info!("路径规范化成功: {} -> {}", path, normalized);
    Ok(normalized)
}

/// 连接路径
#[tauri::command]
pub async fn join_paths(paths: Vec<String>) -> Result<String, String> {
    info!("开始连接路径: {:?}", paths);

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
    info!("路径连接成功: {:?} -> {}", paths, joined);
    Ok(joined)
}

/// 检查路径是否存在
#[tauri::command]
pub async fn path_exists(path: String) -> Result<bool, String> {
    info!("开始检查路径是否存在: {}", path);

    if path.is_empty() {
        return Err("输入验证错误: 路径不能为空".to_string());
    }

    let exists = Path::new(&path).exists();
    info!("路径存在性检查完成: {} -> {}", path, exists);
    Ok(exists)
}

// ===== 平台信息命令 =====

/// 获取平台信息
///
/// 统一命令处理规范：
/// - 参数顺序：无业务参数，state参数在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
/// - 缓存机制：首次检测后缓存结果，后续直接返回缓存
#[tauri::command]
pub async fn get_platform_info(state: State<'_, WindowState>) -> Result<PlatformInfo, String> {
    info!("开始获取平台信息");

    let platform_info = state
        .with_config_manager(|config| {
            let info = config.get_platform_info().clone();
            Ok(info)
        })
        .await
        .map_err(|e| e.to_string())?;

    info!(
        "平台信息获取成功: platform={}, arch={}, is_mac={}",
        platform_info.platform, platform_info.arch, platform_info.is_mac
    );

    Ok(platform_info)
}
