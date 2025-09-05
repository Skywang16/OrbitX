use std::sync::Arc;
use tauri::{ipc::Channel, State};
use tokio_stream::StreamExt;

use super::{
    service::LLMService,
    types::{LLMRequest, LLMResponse, LLMStreamChunk},
};
use crate::storage::repositories::RepositoryManager;

/// LLM 管理器状态
pub struct LLMManagerState {
    pub service: Arc<LLMService>,
}

impl LLMManagerState {
    pub fn new(repositories: Arc<RepositoryManager>) -> Self {
        let service = Arc::new(LLMService::new(repositories.clone()));
        Self { service }
    }
}

/// 非流式LLM调用
#[tauri::command]
pub async fn llm_call(
    state: State<'_, LLMManagerState>,
    request: LLMRequest,
) -> Result<LLMResponse, String> {
    state.service.call(request).await.map_err(|e| e.to_string())
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
        .map_err(|e| e.to_string())?;

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
        .map_err(|e| e.to_string())
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
        .map_err(|e| e.to_string())
}
