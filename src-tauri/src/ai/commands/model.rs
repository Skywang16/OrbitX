//! AI模型管理命令

use super::AIManagerState;
use crate::ai::types::AIModelConfig;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success, validate_not_empty};

use tauri::State;
use tracing::warn;

/// 获取所有AI模型配置
#[tauri::command]
pub async fn ai_models_get(state: State<'_, AIManagerState>) -> TauriApiResult<Vec<AIModelConfig>> {
    match state.ai_service.get_models().await {
        Ok(models) => {
            let sanitized: Vec<AIModelConfig> = models
                .into_iter()
                .map(|mut m| {
                    m.api_key.clear();
                    m
                })
                .collect();
            Ok(api_success!(sanitized))
        }
        Err(error) => {
            warn!(error = %error, "加载AI模型配置失败");
            Ok(api_error!("ai.get_models_failed"))
        }
    }
}

/// 添加AI模型配置
#[tauri::command]
pub async fn ai_models_add(
    config: AIModelConfig,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<AIModelConfig> {
    match state.ai_service.add_model(config.clone()).await {
        Ok(_) => {
            let mut sanitized = config.clone();
            sanitized.api_key.clear();
            Ok(api_success!(sanitized, "ai.add_model_success"))
        }
        Err(error) => {
            warn!(error = %error, "添加AI模型失败");
            Ok(api_error!("ai.add_model_failed"))
        }
    }
}

/// 删除AI模型配置
#[tauri::command]
pub async fn ai_models_remove(
    model_id: String,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    validate_not_empty!(model_id, "common.invalid_params");

    match state.ai_service.remove_model(&model_id).await {
        Ok(_) => Ok(api_success!(EmptyData::default(), "ai.remove_model_success")),
        Err(error) => {
            warn!(error = %error, model_id = %model_id, "删除AI模型失败");
            Ok(api_error!("ai.remove_model_failed"))
        }
    }
}

/// 更新AI模型配置
#[tauri::command]
pub async fn ai_models_update(
    model_id: String,
    updates: serde_json::Value,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    validate_not_empty!(model_id, "common.invalid_params");

    match state.ai_service.update_model(&model_id, updates).await {
        Ok(_) => Ok(api_success!(EmptyData::default(), "ai.update_model_success")),
        Err(error) => {
            warn!(error = %error, model_id = %model_id, "更新AI模型失败");
            Ok(api_error!("ai.update_model_failed"))
        }
    }
}

/// 测试AI模型连接
#[tauri::command]
pub async fn ai_models_test_connection(
    config: AIModelConfig,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    if config.api_url.trim().is_empty() {
        return Ok(api_error!("ai.api_url_empty"));
    }
    if config.api_key.trim().is_empty() {
        return Ok(api_error!("ai.api_key_empty"));
    }
    if config.model.trim().is_empty() {
        return Ok(api_error!("ai.model_name_empty"));
    }

    match state.ai_service.test_connection_with_config(&config).await {
        Ok(_result) => Ok(api_success!(EmptyData::default(), "ai.test_connection_success")),
        Err(e) => Ok(api_error!("ai.test_connection_error", "error" => e.to_string())),
    }
}
