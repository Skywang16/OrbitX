//! 补全功能命令

use crate::ai::tool::storage::StorageCoordinatorState;
use crate::completion::engine::{CompletionEngine, CompletionEngineConfig};
use crate::completion::types::{CompletionContext, CompletionResponse};
use crate::utils::error::{TauriResult, ToTauriResult};
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use anyhow::anyhow;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct CompletionState {
    pub engine: Arc<Mutex<Option<Arc<CompletionEngine>>>>,
}

impl Default for CompletionState {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionState {
    pub fn new() -> Self {
        Self {
            engine: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn validate(&self) -> TauriResult<()> {
        let engine_state = self
            .engine
            .lock()
            .map_err(|_| "获取引擎状态锁失败".to_string())?;

        match engine_state.as_ref() {
            Some(_) => Ok(()),
            None => Err(anyhow!("[配置错误] 补全引擎未初始化")).to_tauri(),
        }
    }

    /// 获取引擎实例
    pub async fn get_engine(&self) -> TauriResult<Arc<CompletionEngine>> {
        let engine_state = self
            .engine
            .lock()
            .map_err(|_| "获取引擎状态锁失败".to_string())?;

        match engine_state.as_ref() {
            Some(engine) => Ok(Arc::clone(engine)),
            None => Err(anyhow!("[配置错误] 补全引擎未初始化")).to_tauri(),
        }
    }

    /// 设置引擎实例
    pub async fn set_engine(&self, engine: Arc<CompletionEngine>) -> TauriResult<()> {
        let mut engine_state = self
            .engine
            .lock()
            .map_err(|_| "获取引擎状态锁失败".to_string())?;

        *engine_state = Some(engine);
        Ok(())
    }
}

/// 获取补全建议命令
#[tauri::command]
pub async fn completion_get(
    input: String,
    cursor_position: usize,
    working_directory: String,
    max_results: Option<usize>,
    state: State<'_, CompletionState>,
) -> TauriApiResult<CompletionResponse> {
    let engine = match state.get_engine().await {
        Ok(engine) => engine,
        Err(_) => return Ok(api_error!("completion.engine_not_initialized")),
    };

    let working_directory = PathBuf::from(&working_directory);
    let context = CompletionContext::new(input, cursor_position, working_directory);

    match engine.completion_get(&context).await {
        Ok(mut response) => {
            if let Some(max_results) = max_results {
                if response.items.len() > max_results {
                    response.items.truncate(max_results);
                    response.has_more = true;
                }
            }

            Ok(api_success!(response))
        }
        Err(_) => Ok(api_error!("completion.get_failed")),
    }
}

/// 初始化补全引擎命令
#[tauri::command]
pub async fn completion_init_engine(
    state: State<'_, CompletionState>,
    storage_state: State<'_, StorageCoordinatorState>,
) -> TauriApiResult<EmptyData> {
    let config = CompletionEngineConfig::default();
    let cache = storage_state.coordinator.cache();

    match CompletionEngine::with_default_providers(config, cache).await {
        Ok(engine) => match state.set_engine(Arc::new(engine)).await {
            Ok(_) => Ok(api_success!()),
            Err(_) => Ok(api_error!("completion.init_failed")),
        },
        Err(_) => Ok(api_error!("completion.init_failed")),
    }
}

/// 清理缓存命令
#[tauri::command]
pub async fn completion_clear_cache(
    state: State<'_, CompletionState>,
) -> TauriApiResult<EmptyData> {
    let engine = match state.get_engine().await {
        Ok(engine) => engine,
        Err(_) => return Ok(api_error!("completion.engine_not_initialized")),
    };

    match engine.clear_cached_results().await {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("completion.clear_cache_failed")),
    }
}

/// 获取统计信息命令
#[tauri::command]
pub async fn completion_get_stats(state: State<'_, CompletionState>) -> TauriApiResult<String> {
    let engine = match state.get_engine().await {
        Ok(engine) => engine,
        Err(_) => return Ok(api_error!("completion.engine_not_initialized")),
    };

    match engine.get_stats() {
        Ok(stats) => {
            let stats_json = serde_json::json!({
                "provider_count": stats.provider_count
            });

            Ok(api_success!(stats_json.to_string()))
        }
        Err(_) => Ok(api_error!("completion.stats_failed")),
    }
}
