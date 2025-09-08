/*!
 * 向量索引启动初始化模块
 *
 * 在应用启动时检查向量索引全局开关，如果启用则自动初始化向量索引服务
 */

use crate::ai::tool::storage::StorageCoordinatorState;
use crate::llm::commands::LLMManagerState;
use crate::vector_index::{
    commands::VectorIndexState, service::VectorIndexService, types::VectorIndexConfig,
};
use anyhow::{Context, Result};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tracing::{debug, info, warn};

/// 在应用启动时初始化向量索引服务
///
/// 检查存储在mp层的全局开关，如果启用则自动初始化向量索引服务
pub async fn initialize_vector_index_on_startup<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    info!("检查向量索引启动配置");

    // 获取存储状态
    let storage_state = app.state::<StorageCoordinatorState>();
    // 获取当前会话状态
    let session_state = storage_state
        .coordinator
        .load_session_state()
        .await
        .context("获取会话状态失败")?
        .unwrap_or_default();

    // 检查向量索引是否启用
    if !session_state.ai.vector_index_enabled {
        info!("向量索引功能未启用，跳过初始化");
        return Ok(());
    }

    info!("向量索引功能已启用，开始自动初始化");

    // 获取向量索引配置
    let config_service = crate::vector_index::VectorIndexConfigService::new(
        storage_state.coordinator.repositories(),
    );

    let vector_config = config_service
        .get_config_or_default()
        .await
        .context("获取向量索引配置失败")?;

    // 验证配置完整性
    if vector_config.qdrant_url.trim().is_empty() {
        warn!("向量索引配置不完整（缺少Qdrant URL），跳过初始化");
        return Ok(());
    }

    // 获取所需的状态管理器
    let vector_index_state = app.state::<VectorIndexState>();
    let llm_state = app.state::<LLMManagerState>();
    let ai_state = app.state::<crate::ai::AIManagerState>();

    // 检查是否已经初始化
    if vector_index_state.is_initialized().await {
        info!("向量索引服务已初始化，跳过");
        return Ok(());
    }

    // 执行初始化
    match try_initialize_vector_service(&vector_config, &vector_index_state, &llm_state, &ai_state)
        .await
    {
        Ok(()) => {
            info!("向量索引服务启动初始化成功");

            // 发送初始化成功事件
            if let Err(e) = app.emit(
                "vector-index-startup",
                serde_json::json!({
                    "status": "initialized",
                    "message": "向量索引服务启动初始化成功"
                }),
            ) {
                warn!("发送向量索引启动事件失败: {}", e);
            }
        }
        Err(e) => {
            warn!("向量索引服务启动初始化失败: {}", e);

            // 发送初始化失败事件
            if let Err(e) = app.emit(
                "vector-index-startup",
                serde_json::json!({
                    "status": "failed",
                    "message": format!("向量索引服务启动初始化失败: {}", e)
                }),
            ) {
                warn!("发送向量索引启动事件失败: {}", e);
            }
        }
    }

    Ok(())
}

/// 尝试初始化向量索引服务
async fn try_initialize_vector_service(
    config: &VectorIndexConfig,
    vector_index_state: &VectorIndexState,
    llm_state: &LLMManagerState,
    ai_state: &crate::ai::AIManagerState,
) -> Result<()> {
    debug!("开始初始化向量索引服务");

    // 获取embedding模型配置
    let models = ai_state.ai_service.get_models().await;
    let _embedding_model_config = models
        .iter()
        .find(|m| m.id == config.embedding_model_id)
        .ok_or_else(|| {
            anyhow::anyhow!("找不到配置的embedding模型: {}", config.embedding_model_id)
        })?;

    // 创建LLM服务实例
    let llm_service = llm_state.service.clone();

    // 创建向量索引服务实例
    let service = VectorIndexService::new(
        config.clone(),
        llm_service,
        _embedding_model_config.id.clone(),
    )
    .await
    .context("创建向量索引服务失败")?;

    // 存储服务实例
    vector_index_state.set_service(Arc::new(service)).await;

    debug!("向量索引服务初始化成功");
    Ok(())
}

/// 检查向量索引功能是否启用（供其他模块调用）
pub async fn is_vector_index_enabled(app: &tauri::AppHandle) -> Result<bool> {
    let storage_state = app.state::<StorageCoordinatorState>();
    let session_state = storage_state
        .coordinator
        .load_session_state()
        .await
        .context("获取会话状态失败")?
        .unwrap_or_default();

    Ok(session_state.ai.vector_index_enabled)
}

/// 获取已配置的向量索引工作目录列表
pub async fn get_vector_index_workspaces(app: &tauri::AppHandle) -> Result<Vec<String>> {
    let storage_state = app.state::<StorageCoordinatorState>();
    let session_state = storage_state
        .coordinator
        .load_session_state()
        .await
        .context("获取会话状态失败")?
        .unwrap_or_default();

    Ok(session_state.ai.vector_index_workspaces)
}
