//! Completion command handlers for Tauri

use crate::completion::engine::{CompletionEngine, CompletionEngineConfig};
use crate::completion::error::{CompletionStateError, CompletionStateResult};
use crate::completion::types::{CompletionContext, CompletionResponse};
use crate::storage::DatabaseManager;
use crate::storage::UnifiedCache;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tauri::State;

pub struct CompletionState {
    engine: OnceLock<Arc<CompletionEngine>>,
}

impl Default for CompletionState {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionState {
    pub fn new() -> Self {
        Self {
            engine: OnceLock::new(),
        }
    }

    pub fn validate(&self) -> CompletionStateResult<()> {
        self.engine
            .get()
            .map(|_| ())
            .ok_or(CompletionStateError::NotInitialized)
    }

    /// 获取引擎实例
    pub fn get_engine(&self) -> CompletionStateResult<Arc<CompletionEngine>> {
        self.engine
            .get()
            .map(Arc::clone)
            .ok_or(CompletionStateError::NotInitialized)
    }

    /// 设置引擎实例
    pub fn set_engine(&self, engine: Arc<CompletionEngine>) -> CompletionStateResult<()> {
        match self.engine.set(engine) {
            Ok(()) => Ok(()),
            Err(_) => Ok(()),
        }
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
    let engine = match state.get_engine() {
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
    cache: State<'_, Arc<UnifiedCache>>,
    database: State<'_, Arc<DatabaseManager>>,
) -> TauriApiResult<EmptyData> {
    if state.validate().is_ok() {
        return Ok(api_success!());
    }

    let config = CompletionEngineConfig::default();

    match CompletionEngine::with_default_providers(
        config,
        cache.inner().clone(),
        database.inner().clone(),
    )
    .await
    {
        Ok(engine) => match state.set_engine(Arc::new(engine)) {
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
    let engine = match state.get_engine() {
        Ok(engine) => engine,
        Err(_) => return Ok(api_error!("completion.engine_not_initialized")),
    };

    match engine.clear_cached_results().await {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("completion.clear_cache_failed")),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionStats {
    pub provider_count: usize,
}

/// 获取统计信息命令
#[tauri::command]
pub async fn completion_get_stats(
    state: State<'_, CompletionState>,
) -> TauriApiResult<CompletionStats> {
    let engine = match state.get_engine() {
        Ok(engine) => engine,
        Err(_) => return Ok(api_error!("completion.engine_not_initialized")),
    };

    match engine.get_stats() {
        Ok(stats) => Ok(api_success!(CompletionStats {
            provider_count: stats.provider_count,
        })),
        Err(_) => Ok(api_error!("completion.stats_failed")),
    }
}
