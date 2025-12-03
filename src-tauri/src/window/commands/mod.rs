// Window command handlers exposed to Tauri

pub mod directory;
pub mod opacity;
pub mod platform;
pub mod state;

pub use directory::*;
pub use opacity::*;
pub use platform::*;
pub use state::*;

use crate::window::WindowStateResult;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime, State};
use tokio::sync::Mutex;
use tracing::{error, warn};

// Window state management container
pub struct WindowState {
    pub cache: crate::storage::cache::UnifiedCache,
    pub config_manager: Arc<Mutex<WindowConfigManager>>,
    pub state_manager: Arc<Mutex<WindowStateManager>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub platform: String,
    pub arch: String,
    pub os_version: String,
    pub is_mac: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowStateOperation {
    GetState,
    SetAlwaysOnTop,
    ToggleAlwaysOnTop,
    ResetState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStateOperationParams {
    pub always_on_top: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStateOperationRequest {
    pub operation: WindowStateOperation,
    pub params: Option<WindowStateOperationParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStateBatchRequest {
    pub operations: Vec<WindowStateOperationRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStateOperationResult {
    pub operation: WindowStateOperation,
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStateBatchResponse {
    pub results: Vec<WindowStateOperationResult>,
    pub overall_success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteWindowState {
    pub always_on_top: bool,
    pub current_directory: String,
    pub home_directory: String,
    pub platform_info: PlatformInfo,
    pub timestamp: u64,
}

#[derive(Debug)]
pub struct WindowConfigManager {
    platform_info: Option<PlatformInfo>,
    default_window_id: String,
    operation_timeout: u64,
}

#[derive(Debug)]
pub struct WindowStateManager {
    cached_cwd: Option<PathBuf>,
    cached_home: Option<PathBuf>,
    always_on_top: AtomicBool,
    last_update: Option<Instant>,
    cache_ttl: std::time::Duration,
}

impl Default for WindowStateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowStateManager {
    pub fn new() -> Self {
        Self {
            cached_cwd: None,
            cached_home: None,
            always_on_top: AtomicBool::new(false),
            last_update: None,
            cache_ttl: std::time::Duration::from_secs(30),
        }
    }

    pub fn set_always_on_top(&mut self, value: bool) {
        self.always_on_top.store(value, Ordering::Release);  // 原子写入
        self.last_update = Some(Instant::now());
    }

    pub fn get_always_on_top(&self) -> bool {
        self.always_on_top.load(Ordering::Acquire)  // 原子读取
    }

    pub fn toggle_always_on_top(&mut self) -> bool {
        let new_value = !self.always_on_top.load(Ordering::Acquire);
        self.always_on_top.store(new_value, Ordering::Release);
        self.last_update = Some(Instant::now());
        new_value
    }

    pub fn set_cached_cwd(&mut self, path: PathBuf) {
        self.cached_cwd = Some(path);
        self.last_update = Some(Instant::now());
    }

    pub fn get_cached_cwd(&self) -> Option<&PathBuf> {
        if self.is_cache_valid() {
            self.cached_cwd.as_ref()
        } else {
            None
        }
    }

    pub fn set_cached_home(&mut self, path: PathBuf) {
        self.cached_home = Some(path);
    }

    pub fn get_cached_home(&self) -> Option<&PathBuf> {
        self.cached_home.as_ref()
    }

    pub fn clear_cache(&mut self) {
        self.cached_cwd = None;
        self.cached_home = None;
        self.last_update = None;
    }

    fn is_cache_valid(&self) -> bool {
        self.last_update
            .map(|last| last.elapsed() < self.cache_ttl)
            .unwrap_or(false)
    }

    pub fn reset(&mut self) {
        self.always_on_top.store(false, Ordering::Release);
        self.clear_cache();
    }
}

impl WindowConfigManager {
    pub fn new() -> Self {
        Self {
            platform_info: None,
            default_window_id: "main".to_string(),
            operation_timeout: 5000,
        }
    }

    pub fn set_platform_info(&mut self, info: PlatformInfo) {
        self.platform_info = Some(info);
    }

    pub fn window_get_platform_info(&self) -> Option<&PlatformInfo> {
        self.platform_info.as_ref()
    }

    pub fn get_default_window_id(&self) -> &str {
        &self.default_window_id
    }

    pub fn set_default_window_id(&mut self, id: String) {
        self.default_window_id = id;
    }

    pub fn get_operation_timeout(&self) -> u64 {
        self.operation_timeout
    }

    pub fn set_operation_timeout(&mut self, timeout: u64) {
        self.operation_timeout = timeout;
    }
}

impl Default for WindowConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowState {
    pub fn new() -> WindowStateResult<Self> {
        let config_manager = Arc::new(Mutex::new(WindowConfigManager::new()));
        let state_manager = Arc::new(Mutex::new(WindowStateManager::new()));

        let state = Self {
            cache: crate::storage::cache::UnifiedCache::new(),
            config_manager,
            state_manager,
        };

        Ok(state)
    }

    pub async fn with_config_manager<F, R>(&self, f: F) -> WindowStateResult<R>
    where
        F: FnOnce(&WindowConfigManager) -> WindowStateResult<R>,
    {
        let config_manager = self.config_manager.lock().await;
        f(&*config_manager)
    }

    pub async fn with_config_manager_mut<F, R>(&self, f: F) -> WindowStateResult<R>
    where
        F: FnOnce(&mut WindowConfigManager) -> WindowStateResult<R>,
    {
        let mut config_manager = self.config_manager.lock().await;
        f(&mut *config_manager)
    }

    pub async fn with_state_manager<F, R>(&self, f: F) -> WindowStateResult<R>
    where
        F: FnOnce(&WindowStateManager) -> WindowStateResult<R>,
    {
        let state_manager = self.state_manager.lock().await;
        f(&*state_manager)
    }

    pub async fn with_state_manager_mut<F, R>(&self, f: F) -> WindowStateResult<R>
    where
        F: FnOnce(&mut WindowStateManager) -> WindowStateResult<R>,
    {
        let mut state_manager = self.state_manager.lock().await;
        f(&mut *state_manager)
    }
}
