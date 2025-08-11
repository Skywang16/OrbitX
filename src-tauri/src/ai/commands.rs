/*!
 * AI功能的Tauri命令接口 - 全新重构版本
 *
 * 实现基于会话上下文管理的AI命令接口
 */

use crate::ai::{
    context::handle_truncate_conversation,
    types::{AIModelConfig, Conversation, Message},
    AIService,
};
use crate::storage::cache::UnifiedCache;
use crate::storage::sqlite::SqliteManager;
use chrono::Utc;
use std::sync::Arc;
use tauri::State;
use tracing::{debug, info};

/// AI管理器状态 - 重构版本
pub struct AIManagerState {
    pub ai_service: Arc<AIService>,
    pub sqlite_manager: Option<Arc<SqliteManager>>,
    pub cache: Arc<UnifiedCache>,
}

impl AIManagerState {
    /// 创建新的AI管理器状态
    pub fn new(
        sqlite_manager: Option<Arc<SqliteManager>>,
        cache: Arc<UnifiedCache>,
    ) -> Result<Self, String> {
        let ai_service = Arc::new(AIService::new(
            sqlite_manager.clone().unwrap(),
            cache.clone(),
        ));

        Ok(Self {
            ai_service,
            sqlite_manager,
            cache,
        })
    }

    /// 初始化AI服务
    pub async fn initialize(&self) -> Result<(), String> {
        self.ai_service
            .initialize()
            .await
            .map_err(|e| e.to_string())
    }
}

// ===== AI会话上下文管理命令 - 全新实现 =====

/// 创建新会话
#[tauri::command]
pub async fn create_conversation(
    title: Option<String>,
    state: State<'_, AIManagerState>,
) -> Result<i64, String> {
    debug!("创建新会话: title={:?}", title);

    let sqlite_manager = state
        .sqlite_manager
        .as_ref()
        .ok_or_else(|| "数据库管理器未初始化".to_string())?;

    let conversation = Conversation {
        id: 0, // 数据库自动生成
        title: title.unwrap_or_else(|| "新对话".to_string()),
        message_count: 0,
        last_message_preview: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let conversation_id = sqlite_manager
        .create_conversation(&conversation)
        .await
        .map_err(|e| e.to_string())?;

    info!("成功创建会话: {}", conversation_id);
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

    let sqlite_manager = state
        .sqlite_manager
        .as_ref()
        .ok_or_else(|| "数据库管理器未初始化".to_string())?;

    let conversations = sqlite_manager
        .get_conversations(limit, offset)
        .await
        .map_err(|e| e.to_string())?;

    Ok(conversations)
}

/// 获取会话详情
#[tauri::command]
pub async fn get_conversation(
    conversation_id: i64,
    state: State<'_, AIManagerState>,
) -> Result<Conversation, String> {
    debug!("获取会话详情: {}", conversation_id);

    let sqlite_manager = state
        .sqlite_manager
        .as_ref()
        .ok_or_else(|| "数据库管理器未初始化".to_string())?;

    let conversation = sqlite_manager
        .get_conversation(conversation_id)
        .await
        .map_err(|e| e.to_string())?
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
    debug!("更新会话标题: {} -> {}", conversation_id, title);

    // 参数验证
    if title.trim().is_empty() {
        return Err("会话标题不能为空".to_string());
    }
    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }

    let sqlite_manager = state
        .sqlite_manager
        .as_ref()
        .ok_or_else(|| "数据库管理器未初始化".to_string())?;

    sqlite_manager
        .update_conversation_title(conversation_id, &title)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 删除会话
#[tauri::command]
pub async fn delete_conversation(
    conversation_id: i64,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    debug!("删除会话: {}", conversation_id);

    let sqlite_manager = state
        .sqlite_manager
        .as_ref()
        .ok_or_else(|| "数据库管理器未初始化".to_string())?;

    sqlite_manager
        .delete_conversation(conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    info!("成功删除会话: {}", conversation_id);
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

    // 参数验证
    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }

    let sqlite_manager = state
        .sqlite_manager
        .as_ref()
        .ok_or_else(|| "数据库管理器未初始化".to_string())?;

    // 使用context.rs中的build_context_for_request函数
    let config = crate::ai::types::AIConfig::default();
    let messages = crate::ai::context::build_context_for_request(
        sqlite_manager,
        conversation_id,
        up_to_message_id,
        &config,
    )
    .await
    .map_err(|e| e.to_string())?;

    // TODO: 实现智能压缩功能
    // 当前版本直接返回所有消息，未来将在这里实现：
    // - 基于token限制的智能压缩
    // - 语义相似度分析
    // - 重要性评分
    // - 动态压缩策略选择

    info!(
        "压缩上下文获取完成: conversation_id={}, 消息数量={}",
        conversation_id,
        messages.len()
    );

    Ok(messages)
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

    // 参数验证
    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }
    if truncate_after_message_id <= 0 {
        return Err("无效的消息ID".to_string());
    }

    let sqlite_manager = state
        .sqlite_manager
        .as_ref()
        .ok_or_else(|| "数据库管理器未初始化".to_string())?;

    // 截断会话
    handle_truncate_conversation(sqlite_manager, conversation_id, truncate_after_message_id)
        .await
        .map_err(|e| e.to_string())?;

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

    // 参数验证
    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }
    if content.trim().is_empty() {
        return Err("消息内容不能为空".to_string());
    }
    if !["user", "assistant", "system"].contains(&role.as_str()) {
        return Err("无效的消息角色".to_string());
    }

    let sqlite_manager = state
        .sqlite_manager
        .as_ref()
        .ok_or_else(|| "数据库管理器未初始化".to_string())?;

    // 创建消息对象
    let message = Message {
        id: 0, // 数据库自动生成
        conversation_id,
        role,
        content,
        created_at: Utc::now(),
    };

    // 保存消息
    let message_id = sqlite_manager
        .save_message(&message)
        .await
        .map_err(|e| e.to_string())?;

    info!("消息保存成功: message_id={}", message_id);
    Ok(message_id)
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

    state
        .ai_service
        .add_model(config)
        .await
        .map_err(|e| e.to_string())
}

/// 删除AI模型配置
#[tauri::command]
pub async fn remove_ai_model(
    model_id: String,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    info!("删除AI模型: {}", model_id);

    state
        .ai_service
        .remove_model(&model_id)
        .await
        .map_err(|e| e.to_string())
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
        .map_err(|e| e.to_string())
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
        .map_err(|e| e.to_string())
}

/// 测试AI模型连接
#[tauri::command]
pub async fn test_ai_connection(
    model_id: String,
    state: State<'_, AIManagerState>,
) -> Result<bool, String> {
    info!("测试AI模型连接: {}", model_id);

    // 参数验证
    if model_id.trim().is_empty() {
        return Err("模型ID不能为空".to_string());
    }

    state
        .ai_service
        .test_connection(&model_id)
        .await
        .map_err(|e| e.to_string())
}
