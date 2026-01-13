use std::sync::Arc;
use tauri::{ipc::Channel, State};
use tokio_stream::StreamExt;

use super::{provider_registry::ProviderRegistry, service::LLMService};
use crate::llm::anthropic_types::{CreateMessageRequest, Message, StreamEvent};
use crate::storage::DatabaseManager;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};

pub struct LLMManagerState {
    pub service: Arc<LLMService>,
}

impl LLMManagerState {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        let service = Arc::new(LLMService::new(database.clone()));
        Self { service }
    }
}

#[tauri::command]
pub async fn llm_call(
    state: State<'_, LLMManagerState>,
    request: CreateMessageRequest,
) -> TauriApiResult<Message> {
    match state.service.call(request).await {
        Ok(response) => Ok(api_success!(response)),
        Err(_) => Ok(api_error!("llm.call_failed")),
    }
}

#[tauri::command]
pub async fn llm_call_stream(
    state: State<'_, LLMManagerState>,
    request: CreateMessageRequest,
    on_chunk: Channel<StreamEvent>,
) -> TauriApiResult<EmptyData> {
    use tokio_util::sync::CancellationToken;
    let token = CancellationToken::new();
    let mut stream = match state.service.call_stream(request, token).await {
        Ok(stream) => stream,
        Err(_) => return Ok(api_error!("llm.stream_failed")),
    };

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if let Err(e) = on_chunk.send(chunk) {
                    tracing::error!("Failed to send chunk: {}", e);
                    break;
                }
            }
            Err(e) => {
                tracing::error!("Stream error: {}", e);
                let error_chunk = StreamEvent::Error {
                    error: crate::llm::anthropic_types::ErrorData {
                        error_type: "stream_error".to_string(),
                        message: e.to_string(),
                    },
                };
                if let Err(e) = on_chunk.send(error_chunk) {
                    tracing::error!("Failed to send error chunk: {}", e);
                }
                break;
            }
        }
    }

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
    _state: State<'_, LLMManagerState>,
) -> TauriApiResult<Vec<super::provider_registry::ProviderMetadata>> {
    let providers = ProviderRegistry::global()
        .get_all_providers_metadata().to_vec();
    Ok(api_success!(providers))
}
