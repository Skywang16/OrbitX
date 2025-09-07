use std::sync::Arc;
use tauri::{ipc::Channel, State};
use tokio_stream::StreamExt;

use super::{
    registry::{LLMRegistry, ModelInfo, ProviderInfo},
    service::LLMService,
    types::{LLMProviderType, LLMRequest, LLMResponse, LLMStreamChunk},
};
use crate::storage::repositories::RepositoryManager;
use crate::utils::error::ToTauriResult;

/// LLM 管理器状态
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

/// 非流式LLM调用
#[tauri::command]
pub async fn llm_call(
    state: State<'_, LLMManagerState>,
    request: LLMRequest,
) -> Result<LLMResponse, String> {
    state.service.call(request).await.to_tauri()
}

/// 流式LLM调用
#[tauri::command]
pub async fn llm_call_stream(
    state: State<'_, LLMManagerState>,
    request: LLMRequest,
    on_chunk: Channel<LLMStreamChunk>,
) -> Result<(), String> {
    tracing::debug!("Starting stream call for model: {}", request.model);

    let mut stream = state
        .service
        .call_stream(request)
        .await
        .to_tauri()?;

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
    Ok(())
}

/// 获取可用的模型列表
#[tauri::command]
pub async fn llm_get_available_models(
    state: State<'_, LLMManagerState>,
) -> Result<Vec<String>, String> {
    state
        .service
        .get_available_models()
        .await
        .to_tauri()
}

/// 测试模型连接
#[tauri::command]
pub async fn llm_test_model_connection(
    state: State<'_, LLMManagerState>,
    model_id: String,
) -> Result<bool, String> {
    state
        .service
        .test_model_connection(&model_id)
        .await
        .to_tauri()
}

/// 获取所有供应商信息
#[tauri::command]
pub async fn llm_get_providers(
    state: State<'_, LLMManagerState>,
) -> Result<Vec<ProviderInfo>, String> {
    Ok(state
        .registry
        .get_all_providers()
        .into_iter()
        .cloned()
        .collect())
}

/// 获取指定供应商的模型列表
#[tauri::command]
pub async fn llm_get_provider_models(
    state: State<'_, LLMManagerState>,
    provider_type: LLMProviderType,
) -> Result<Vec<ModelInfo>, String> {
    Ok(state
        .registry
        .get_models_for_provider(&provider_type)
        .into_iter()
        .cloned()
        .collect())
}

/// 根据模型ID获取模型信息
#[tauri::command]
pub async fn llm_get_model_info(
    state: State<'_, LLMManagerState>,
    model_id: String,
) -> Result<Option<(ProviderInfo, ModelInfo)>, String> {
    Ok(state
        .registry
        .find_model(&model_id)
        .map(|(provider, model)| (provider.clone(), model.clone())))
}

/// 检查模型是否支持指定功能
#[tauri::command]
pub async fn llm_check_model_feature(
    state: State<'_, LLMManagerState>,
    model_id: String,
    feature: String,
) -> Result<bool, String> {
    Ok(state.registry.model_supports_feature(&model_id, &feature))
}
