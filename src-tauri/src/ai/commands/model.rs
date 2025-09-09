/*!
 * AI模型管理命令
 *
 * 负责AI模型的配置、管理和连接测试功能
 */

use super::AIManagerState;
use crate::ai::types::AIModelConfig;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success, validate_not_empty};

use tauri::State;

// ===== AI模型管理命令 =====

/// 获取所有AI模型配置
#[tauri::command]
pub async fn get_ai_models(state: State<'_, AIManagerState>) -> TauriApiResult<Vec<AIModelConfig>> {
    let models = state.ai_service.get_models().await;
    // 不回传明文 API Key：对外返回时清空 api_key 字段
    let sanitized: Vec<AIModelConfig> = models
        .into_iter()
        .map(|mut m| {
            m.api_key.clear();
            m
        })
        .collect();
    Ok(api_success!(sanitized))
}

/// 添加AI模型配置
#[tauri::command]
pub async fn add_ai_model(
    config: AIModelConfig,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<AIModelConfig> {
    // 保存模型配置，如果保存失败会抛出异常
    match state.ai_service.add_model(config.clone()).await {
        Ok(_) => {
            // 不回传明文 API Key
            let mut sanitized = config.clone();
            sanitized.api_key.clear();
            Ok(api_success!(sanitized))
        }
        Err(_) => Ok(api_error!("ai.add_model_failed")),
    }
}

/// 删除AI模型配置
#[tauri::command]
pub async fn remove_ai_model(
    model_id: String,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    validate_not_empty!(model_id, "common.invalid_params");

    match state.ai_service.remove_model(&model_id).await {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("ai.remove_model_failed")),
    }
}

/// 更新AI模型配置
#[tauri::command]
pub async fn update_ai_model(
    model_id: String,
    updates: serde_json::Value,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    validate_not_empty!(model_id, "common.invalid_params");

    match state.ai_service.update_model(&model_id, updates).await {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("ai.update_model_failed")),
    }
}

/// 测试AI模型连接（基于表单数据）
#[tauri::command]
pub async fn test_ai_connection_with_config(
    config: AIModelConfig,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<String> {
    // 参数验证
    if config.api_url.trim().is_empty() {
        return Ok(api_error!("ai.api_url_empty"));
    }
    if config.api_key.trim().is_empty() {
        return Ok(api_error!("ai.api_key_empty"));
    }
    if config.model.trim().is_empty() {
        return Ok(api_error!("ai.model_name_empty"));
    }

    // 直接使用提供的配置进行连接测试，返回详细的连接结果
    match state.ai_service.test_connection_with_config(&config).await {
        Ok(result) => Ok(api_success!(result)),
        Err(e) => Ok(api_error!("ai.test_connection_error", "error" => e.to_string())),
    }
}

// ===== 用户前置提示词管理命令 =====

/// 获取用户前置提示词
#[tauri::command]
pub async fn get_user_prefix_prompt(
    state: State<'_, AIManagerState>,
) -> TauriApiResult<Option<String>> {
    tracing::debug!("获取用户前置提示词");

    let repositories = state.repositories();

    match repositories.ai_models().get_user_prefix_prompt().await {
        Ok(prompt) => Ok(api_success!(prompt)),
        Err(_) => Ok(api_error!("ai.get_prefix_prompt_failed")),
    }
}

/// 设置用户前置提示词
#[tauri::command]
pub async fn set_user_prefix_prompt(
    prompt: Option<String>,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    let repositories = state.repositories();

    match repositories
        .ai_models()
        .set_user_prefix_prompt(prompt)
        .await
    {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("ai.set_prefix_prompt_failed")),
    }
}
