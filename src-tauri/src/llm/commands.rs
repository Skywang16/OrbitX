use std::sync::Arc;
use tauri::{ipc::Channel, State};
use tokio_stream::StreamExt;

use super::{
    registry::{LLMRegistry, ModelInfo, ProviderInfo},
    service::LLMService,
    types::{LLMProviderType, LLMRequest, LLMResponse, LLMStreamChunk},
};
use crate::storage::repositories::RepositoryManager;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};

#[tauri::command]
pub async fn llm_cancel_stream(state: State<'_, LLMManagerState>) -> TauriApiResult<EmptyData> {
    match state.service.cancel_stream().await {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("llm.cancel_failed")),
    }
}

pub struct LLMManagerState {
    pub service: Arc<LLMService>,
    pub registry: Arc<LLMRegistry>,
}

impl LLMManagerState {
    pub fn new(repositories: Arc<RepositoryManager>) -> Self {
        let service = Arc::new(LLMService::new(repositories.clone()));
        let registry = Arc::new(LLMRegistry::new());
        Self { service, registry }
    }
}

#[tauri::command]
pub async fn llm_call(
    state: State<'_, LLMManagerState>,
    request: LLMRequest,
) -> TauriApiResult<LLMResponse> {
    match state.service.call(request).await {
        Ok(response) => Ok(api_success!(response)),
        Err(_) => Ok(api_error!("llm.call_failed")),
    }
}

#[tauri::command]
pub async fn llm_call_stream(
    state: State<'_, LLMManagerState>,
    request: LLMRequest,
    on_chunk: Channel<LLMStreamChunk>,
) -> TauriApiResult<EmptyData> {
    tracing::debug!("Starting stream call for model: {}", request.model);

    let mut stream = match state.service.call_stream(request).await {
        Ok(stream) => stream,
        Err(_) => return Ok(api_error!("llm.stream_failed")),
    };

    tracing::debug!("Stream created successfully, starting to read chunks");
    let mut chunk_count = 0;

    while let Some(chunk_result) = stream.next().await {
        chunk_count += 1;
        tracing::debug!("Received chunk #{}: {:?}", chunk_count, chunk_result);

        match chunk_result {
            Ok(chunk) => {
                tracing::debug!("Sending chunk to frontend: {:?}", chunk);
                if let Err(e) = on_chunk.send(chunk) {
                    tracing::error!("Failed to send chunk: {}", e);
                    break;
                }
            }
            Err(e) => {
                tracing::error!("Stream error: {}", e);
                let error_chunk = LLMStreamChunk::Error {
                    error: e.to_string(),
                };
                if let Err(e) = on_chunk.send(error_chunk) {
                    tracing::error!("Failed to send error chunk: {}", e);
                }
                break;
            }
        }
    }

    tracing::debug!("Stream completed, total chunks: {}", chunk_count);
    Ok(api_success!())
}

/// 获取可用的模型列表
#[tauri::command]
pub async fn llm_get_available_models(
    state: State<'_, LLMManagerState>,
) -> TauriApiResult<Vec<String>> {
    match state.service.get_available_models().await {
        Ok(models) => Ok(api_success!(models)),
        Err(_) => Ok(api_error!("llm.get_models_failed")),
    }
}

/// 测试模型连接
#[tauri::command]
pub async fn llm_test_model_connection(
    state: State<'_, LLMManagerState>,
    model_id: String,
) -> TauriApiResult<bool> {
    match state.service.test_model_connection(&model_id).await {
        Ok(result) => Ok(api_success!(result)),
        Err(_) => Ok(api_error!("llm.test_connection_failed")),
    }
}

/// 获取所有供应商信息
#[tauri::command]
pub async fn llm_get_providers(
    state: State<'_, LLMManagerState>,
) -> TauriApiResult<Vec<ProviderInfo>> {
    let providers = state
        .registry
        .get_all_providers()
        .into_iter()
        .cloned()
        .collect();
    Ok(api_success!(providers))
}

/// 获取指定供应商的模型列表
#[tauri::command]
pub async fn llm_get_provider_models(
    state: State<'_, LLMManagerState>,
    provider_type: LLMProviderType,
) -> TauriApiResult<Vec<ModelInfo>> {
    let models = state
        .registry
        .get_models_for_provider(&provider_type)
        .into_iter()
        .cloned()
        .collect();
    Ok(api_success!(models))
}

/// 根据模型ID获取模型信息
#[tauri::command]
pub async fn llm_get_model_info(
    state: State<'_, LLMManagerState>,
    model_id: String,
) -> TauriApiResult<Option<(ProviderInfo, ModelInfo)>> {
    let model_info = state
        .registry
        .find_model(&model_id)
        .map(|(provider, model)| (provider.clone(), model.clone()));
    Ok(api_success!(model_info))
}

/// 检查模型是否支持指定功能
#[tauri::command]
pub async fn llm_check_model_feature(
    state: State<'_, LLMManagerState>,
    model_id: String,
    feature: String,
) -> TauriApiResult<bool> {
    let supports = state.registry.model_supports_feature(&model_id, &feature);
    Ok(api_success!(supports))
}
