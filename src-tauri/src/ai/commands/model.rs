/*!
 * AI模型管理命令
 *
 * 负责AI模型的配置、管理和连接测试功能
 */

use super::AIManagerState;
use crate::ai::types::AIModelConfig;
use crate::utils::error::ToTauriResult;

use tauri::State;

// ===== AI模型管理命令 =====

/// 获取所有AI模型配置
#[tauri::command]
pub async fn get_ai_models(state: State<'_, AIManagerState>) -> Result<Vec<AIModelConfig>, String> {
    let models = state.ai_service.get_models().await;
    Ok(models)
}

/// 添加AI模型配置
#[tauri::command]
pub async fn add_ai_model(
    config: AIModelConfig,
    state: State<'_, AIManagerState>,
) -> Result<AIModelConfig, String> {
    // 保存模型配置，如果保存失败会抛出异常
    state
        .ai_service
        .add_model(config.clone())
        .await
        .to_tauri()?;

    // 保存成功，直接返回配置
    // 注意：这里返回传入的配置，因为save操作成功意味着数据已正确保存
    Ok(config)
}

/// 删除AI模型配置
#[tauri::command]
pub async fn remove_ai_model(
    model_id: String,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    state.ai_service.remove_model(&model_id).await.to_tauri()
}

/// 更新AI模型配置
#[tauri::command]
pub async fn update_ai_model(
    model_id: String,
    updates: serde_json::Value,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    state
        .ai_service
        .update_model(&model_id, updates)
        .await
        .to_tauri()
}

/// 测试AI模型连接（基于表单数据）
#[tauri::command]
pub async fn test_ai_connection_with_config(
    config: AIModelConfig,
    state: State<'_, AIManagerState>,
) -> Result<bool, String> {
    // 参数验证
    if config.api_url.trim().is_empty() {
        return Err("API URL不能为空".to_string());
    }
    if config.api_key.trim().is_empty() {
        return Err("API Key不能为空".to_string());
    }
    if config.model.trim().is_empty() {
        return Err("模型名称不能为空".to_string());
    }

    // 直接使用提供的配置进行连接测试
    state
        .ai_service
        .test_connection_with_config(&config)
        .await
        .to_tauri()
}

// ===== 用户前置提示词管理命令 =====

/// 获取用户前置提示词
#[tauri::command]
pub async fn get_user_prefix_prompt(
    state: State<'_, AIManagerState>,
) -> Result<Option<String>, String> {
    tracing::debug!("获取用户前置提示词");

    let repositories = state.repositories();

    repositories
        .ai_models()
        .get_user_prefix_prompt()
        .await
        .to_tauri()
}

/// 设置用户前置提示词
#[tauri::command]
pub async fn set_user_prefix_prompt(
    prompt: Option<String>,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    let repositories = state.repositories();

    repositories
        .ai_models()
        .set_user_prefix_prompt(prompt)
        .await
        .to_tauri()
}
