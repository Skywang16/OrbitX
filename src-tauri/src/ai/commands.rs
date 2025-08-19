/*!
 * AI功能的Tauri命令接口 - 全新重构版本
 *
 * 实现基于会话上下文管理的AI命令接口
 */

use crate::ai::types::{AIModelConfig, Conversation, Message};
use crate::ai::{context::handle_truncate_conversation, AIService};
use crate::storage::cache::UnifiedCache;
use crate::storage::repositories::Repository;
use crate::storage::repositories::RepositoryManager;
use crate::utils::error::{ToTauriResult, Validator};

use std::sync::Arc;
use tauri::State;
use tracing::{debug, info};

/// AI管理器状态 - 重构版本
pub struct AIManagerState {
    pub ai_service: Arc<AIService>,
    pub repositories: Arc<RepositoryManager>,
    pub cache: Arc<UnifiedCache>,
}

impl AIManagerState {
    /// 创建新的AI管理器状态
    pub fn new(
        repositories: Arc<RepositoryManager>,
        cache: Arc<UnifiedCache>,
    ) -> Result<Self, String> {
        let ai_service = Arc::new(AIService::new(repositories.clone(), cache.clone()));

        Ok(Self {
            ai_service,
            repositories,
            cache,
        })
    }

    /// 初始化AI服务
    pub async fn initialize(&self) -> Result<(), String> {
        self.ai_service.initialize().await.to_tauri()
    }

    /// 获取Repository管理器的辅助方法
    pub fn repositories(&self) -> &Arc<RepositoryManager> {
        &self.repositories
    }
}

// ===== AI会话上下文管理命令  =====

/// 创建新会话
#[tauri::command]
pub async fn create_conversation(
    title: Option<String>,
    state: State<'_, AIManagerState>,
) -> Result<i64, String> {
    // 验证参数
    if let Some(ref t) = title {
        Validator::validate_not_empty(t, "会话标题")?;
    }

    let repositories = state.repositories();

    let conversation = Conversation::new(title.unwrap_or_else(|| "新对话".to_string()));

    let conversation_id = repositories
        .conversations()
        .save(&conversation)
        .await
        .to_tauri()?;

    info!("创建会话成功, ID: {}", conversation_id);
    Ok(conversation_id)
}

/// 获取会话列表
#[tauri::command]
pub async fn get_conversations(
    limit: Option<i64>,
    offset: Option<i64>,
    state: State<'_, AIManagerState>,
) -> Result<Vec<Conversation>, String> {
    debug!("获取会话列表: limit={:?}, offset={:?}", limit, offset);

    let repositories = state.repositories();

    let conversations = repositories
        .conversations()
        .find_conversations(limit, offset)
        .await
        .to_tauri()?;

    Ok(conversations)
}

/// 获取会话详情
#[tauri::command]
pub async fn get_conversation(
    conversation_id: i64,
    state: State<'_, AIManagerState>,
) -> Result<Conversation, String> {
    debug!("获取会话详情: {}", conversation_id);

    let repositories = state.repositories();

    let conversation = repositories
        .conversations()
        .find_by_id(conversation_id)
        .await
        .to_tauri()?
        .ok_or_else(|| format!("会话不存在: {}", conversation_id))?;

    Ok(conversation)
}

/// 更新会话标题
#[tauri::command]
pub async fn update_conversation_title(
    conversation_id: i64,
    title: String,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    // 参数验证
    Validator::validate_id(conversation_id, "会话ID")?;
    Validator::validate_not_empty(&title, "会话标题")?;

    let repositories = state.repositories();

    repositories
        .conversations()
        .update_title(conversation_id, &title)
        .await
        .to_tauri()?;

    Ok(())
}

/// 删除会话
#[tauri::command]
pub async fn delete_conversation(
    conversation_id: i64,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    // 参数验证
    Validator::validate_id(conversation_id, "会话ID")?;

    let repositories = state.repositories();

    repositories
        .conversations()
        .delete(conversation_id)
        .await
        .to_tauri()?;

    info!("删除会话成功, ID: {}", conversation_id);
    Ok(())
}

/// 获取压缩上下文（供前端eko使用）
#[tauri::command]
pub async fn get_compressed_context(
    conversation_id: i64,
    up_to_message_id: Option<i64>,
    state: State<'_, AIManagerState>,
) -> Result<Vec<Message>, String> {
    info!(
        "获取压缩上下文: conversation_id={}, up_to_message_id={:?}",
        conversation_id, up_to_message_id
    );

    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }

    let repositories = state.repositories();

    // 使用context.rs中的build_context_for_request函数
    let config = crate::ai::types::AIConfig::default();
    let messages = crate::ai::context::build_context_for_request(
        repositories,
        conversation_id,
        up_to_message_id,
        &config,
    )
    .await
    .to_tauri()?;

    info!(
        "压缩上下文获取完成: conversation_id={}, 消息数量={}",
        conversation_id,
        messages.len()
    );

    Ok(messages)
}

/// 构建带智能上下文的prompt（专门用于AI推理）
#[tauri::command]
pub async fn build_prompt_with_context(
    conversation_id: i64,
    current_message: String,
    up_to_message_id: Option<i64>,
    state: State<'_, AIManagerState>,
) -> Result<String, String> {
    info!(
        "构建智能prompt: conversation_id={}, current_message length={}, up_to_message_id={:?}",
        conversation_id,
        current_message.len(),
        up_to_message_id
    );

    // 参数验证
    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }
    if current_message.trim().is_empty() {
        return Err("当前消息不能为空".to_string());
    }

    let repositories = state.repositories();

    // 使用智能上下文管理器构建prompt
    let intelligent_prompt = crate::ai::context::build_intelligent_prompt(
        repositories,
        conversation_id,
        &current_message,
        up_to_message_id,
    )
    .await
    .to_tauri()?;

    info!("智能prompt构建完成: conversation_id={}", conversation_id);
    Ok(intelligent_prompt)
}

/// 截断会话（供前端eko使用）
#[tauri::command]
pub async fn truncate_conversation(
    conversation_id: i64,
    truncate_after_message_id: i64,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    info!(
        "截断会话: conversation_id={}, truncate_after={}",
        conversation_id, truncate_after_message_id
    );

    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }
    if truncate_after_message_id <= 0 {
        return Err("无效的消息ID".to_string());
    }

    let repositories = state.repositories();

    // 截断会话
    handle_truncate_conversation(repositories, conversation_id, truncate_after_message_id)
        .await
        .to_tauri()?;

    info!("会话截断完成: conversation_id={}", conversation_id);
    Ok(())
}

/// 保存单条消息（供前端eko使用）
#[tauri::command]
pub async fn save_message(
    conversation_id: i64,
    role: String,
    content: String,
    state: State<'_, AIManagerState>,
) -> Result<i64, String> {
    info!(
        "保存消息: conversation_id={}, role={}",
        conversation_id, role
    );

    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }
    if content.trim().is_empty() {
        return Err("消息内容不能为空".to_string());
    }
    if !["user", "assistant", "system"].contains(&role.as_str()) {
        return Err("无效的消息角色".to_string());
    }

    let repositories = state.repositories();

    // 创建消息对象
    let message = Message::new(conversation_id, role, content);

    // 保存消息
    let message_id = repositories
        .conversations()
        .save_message(&message)
        .await
        .to_tauri()?;

    info!("消息保存成功: message_id={}", message_id);
    Ok(message_id)
}

/// 更新消息内容
#[tauri::command]
pub async fn update_message_content(
    message_id: i64,
    content: String,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    if message_id <= 0 {
        return Err("无效的消息ID".to_string());
    }

    let repositories = state.repositories();
    repositories
        .conversations()
        .update_message_content(message_id, &content)
        .await
        .to_tauri()?;

    Ok(())
}

/// 更新消息步骤数据
#[tauri::command]
pub async fn update_message_steps(
    message_id: i64,
    steps_json: String,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    if message_id <= 0 {
        return Err("无效的消息ID".to_string());
    }

    let repositories = state.repositories();
    repositories
        .conversations()
        .update_message_steps(message_id, &steps_json)
        .await
        .to_tauri()?;

    Ok(())
}

/// 更新消息状态
#[tauri::command]
pub async fn update_message_status(
    message_id: i64,
    status: Option<String>,
    duration_ms: Option<i64>,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    if message_id <= 0 {
        return Err("无效的消息ID".to_string());
    }

    let repositories = state.repositories();
    repositories
        .conversations()
        .update_message_status(message_id, status.as_deref(), duration_ms)
        .await
        .to_tauri()?;

    Ok(())
}

// ===== AI模型管理命令（保留基础功能） =====

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
) -> Result<(), String> {
    info!("添加AI模型: {}", config.id);

    state.ai_service.add_model(config).await.to_tauri()
}

/// 删除AI模型配置
#[tauri::command]
pub async fn remove_ai_model(
    model_id: String,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    info!("删除AI模型: {}", model_id);

    state.ai_service.remove_model(&model_id).await.to_tauri()
}

/// 更新AI模型配置
#[tauri::command]
pub async fn update_ai_model(
    model_id: String,
    updates: serde_json::Value,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    info!("更新AI模型: {}", model_id);

    state
        .ai_service
        .update_model(&model_id, updates)
        .await
        .to_tauri()
}

/// 设置默认AI模型
#[tauri::command]
pub async fn set_default_ai_model(
    model_id: String,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    info!("设置默认AI模型: {}", model_id);

    state
        .ai_service
        .set_default_model(&model_id)
        .await
        .to_tauri()
}

/// 测试AI模型连接（基于表单数据）
#[tauri::command]
pub async fn test_ai_connection_with_config(
    config: AIModelConfig,
    state: State<'_, AIManagerState>,
) -> Result<bool, String> {
    info!("测试AI模型连接（表单数据）: {}", config.name);

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
    debug!("获取用户前置提示词");

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
    info!("设置用户前置提示词: {:?}", prompt.as_ref().map(|p| p.len()));

    let repositories = state.repositories();

    repositories
        .ai_models()
        .set_user_prefix_prompt(prompt)
        .await
        .to_tauri()
}

// ===== 智能上下文配置管理命令 =====

/// 获取上下文配置
#[tauri::command]
pub async fn get_context_config() -> Result<crate::ai::enhanced_context::ContextConfig, String> {
    Ok(crate::ai::enhanced_context::ContextConfig::default())
}

/// 更新上下文配置
#[tauri::command]
pub async fn update_context_config(
    config: crate::ai::enhanced_context::ContextConfig,
) -> Result<(), String> {
    info!("更新上下文配置: max_tokens={}, compress_threshold={}", 
        config.max_tokens, config.compress_threshold);
    
    // TODO: 实现配置持久化
    // 可以保存到数据库或配置文件
    
    Ok(())
}

/// 获取KV缓存统计
#[tauri::command]
pub async fn get_kv_cache_stats() -> Result<crate::ai::enhanced_context::CacheStats, String> {
    let manager = &*crate::ai::context::CONTEXT_MANAGER;
    Ok(manager.cache_stats())
}

/// 清理过期的KV缓存
#[tauri::command]
pub async fn cleanup_kv_cache() -> Result<(), String> {
    let manager = &*crate::ai::context::CONTEXT_MANAGER;
    manager.cleanup_cache();
    info!("KV缓存清理完成");
    Ok(())
}

/// 手动失效指定对话的缓存
#[tauri::command]
pub async fn invalidate_conversation_cache(conversation_id: i64) -> Result<(), String> {
    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }
    
    let manager = &*crate::ai::context::CONTEXT_MANAGER;
    manager.invalidate_cache(conversation_id);
    Ok(())
}
