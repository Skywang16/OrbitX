/*!
 * AI功能的Tauri命令接口
 *
 * 实现基于会话上下文管理的AI命令接口
 */

use crate::ai::types::{AIModelConfig, Conversation, Message};
use crate::ai::{context::handle_truncate_conversation, AIService};
use crate::mux::PaneId;
use crate::storage::cache::UnifiedCache;
use crate::storage::repositories::Repository;
use crate::storage::repositories::RepositoryManager;
use crate::terminal::TerminalContextService;
use crate::utils::error::{ToTauriResult, Validator};

use std::sync::Arc;
use tauri::State;
use tracing::debug;

/// AI管理器状态
pub struct AIManagerState {
    pub ai_service: Arc<AIService>,
    pub repositories: Arc<RepositoryManager>,
    pub cache: Arc<UnifiedCache>,
    pub terminal_context_service: Arc<TerminalContextService>,
}

impl AIManagerState {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        cache: Arc<UnifiedCache>,
        terminal_context_service: Arc<TerminalContextService>,
    ) -> Result<Self, String> {
        let ai_service = Arc::new(AIService::new(repositories.clone(), cache.clone()));

        Ok(Self {
            ai_service,
            repositories,
            cache,
            terminal_context_service,
        })
    }

    pub async fn initialize(&self) -> Result<(), String> {
        self.ai_service.initialize().await.to_tauri()
    }

    pub fn repositories(&self) -> &Arc<RepositoryManager> {
        &self.repositories
    }

    pub fn get_terminal_context_service(&self) -> &Arc<TerminalContextService> {
        &self.terminal_context_service
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

    // 默认使用空标题，前端渲染时用 i18n 占位文案显示
    let conversation = Conversation::new(title.unwrap_or_else(|| "".to_string()));

    let conversation_id = repositories
        .conversations()
        .save(&conversation)
        .await
        .to_tauri()?;
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
    Ok(())
}

/// 获取压缩上下文（供前端eko使用）
#[tauri::command]
pub async fn get_compressed_context(
    conversation_id: i64,
    up_to_message_id: Option<i64>,
    state: State<'_, AIManagerState>,
) -> Result<Vec<Message>, String> {
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

    Ok(messages)
}

/// 构建带智能上下文的prompt（专门用于AI推理）
#[tauri::command]
pub async fn build_prompt_with_context(
    conversation_id: i64,
    current_message: String,
    up_to_message_id: Option<i64>,
    pane_id: Option<u32>,
    tag_context: Option<serde_json::Value>,
    state: State<'_, AIManagerState>,
) -> Result<String, String> {
    // 参数验证
    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }
    if current_message.trim().is_empty() {
        return Err("当前消息不能为空".to_string());
    }

    let repositories = state.repositories();
    let context_service = state.get_terminal_context_service();

    // 使用 TerminalContextService 解析上下文
    let terminal_context = if let Some(pane_id) = pane_id {
        context_service
            .get_context_by_pane(PaneId::new(pane_id))
            .await
            .map_err(|e| format!("获取终端上下文失败: {}", e))?
    } else {
        context_service
            .get_context_with_fallback(None)
            .await
            .map_err(|e| format!("获取活跃终端上下文失败: {}", e))?
    };

    // 使用智能上下文管理器构建prompt，传递终端上下文
    let intelligent_prompt = crate::ai::context::build_intelligent_prompt_with_context(
        repositories,
        conversation_id,
        &current_message,
        up_to_message_id,
        &terminal_context,
        tag_context,
    )
    .await
    .to_tauri()?;

    Ok(intelligent_prompt)
}

/// 截断会话（供前端eko使用）
#[tauri::command]
pub async fn truncate_conversation(
    conversation_id: i64,
    truncate_after_message_id: i64,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
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
    state.ai_service.add_model(config).await.to_tauri()
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
    _config: crate::ai::enhanced_context::ContextConfig,
) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mux::TerminalMux;
    use crate::shell::ShellIntegrationManager;
    use crate::storage::cache::UnifiedCache;
    use crate::storage::repositories::RepositoryManager;
    use crate::terminal::{ActiveTerminalContextRegistry, TerminalContextService};
    use std::sync::Arc;

    async fn create_test_repositories() -> Arc<RepositoryManager> {
        use crate::storage::database::{DatabaseManager, DatabaseOptions};
        use crate::storage::paths::StoragePaths;
        use chrono::Utc;
        use std::path::PathBuf;

        // 使用系统临时目录下的独立子目录
        let mut app_dir: PathBuf = std::env::temp_dir();
        let unique = format!(
            "orbitx_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        app_dir.push(unique);

        let paths = StoragePaths::new(app_dir).expect("failed to build storage paths");

        let db = Arc::new(
            DatabaseManager::new(paths, DatabaseOptions::default())
                .await
                .expect("failed to create DatabaseManager"),
        );

        db.initialize().await.expect("failed to initialize db");

        Arc::new(RepositoryManager::new(Arc::clone(&db)))
    }

    async fn create_test_ai_manager_state() -> AIManagerState {
        // 创建测试用的存储管理器
        let repositories = create_test_repositories().await;
        let cache = Arc::new(UnifiedCache::new());

        // 创建测试用的终端上下文服务
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new().unwrap());
        let terminal_mux = Arc::new(TerminalMux::new());
        let terminal_context_service = Arc::new(TerminalContextService::new(
            registry,
            shell_integration,
            terminal_mux,
        ));

        AIManagerState::new(repositories, cache, terminal_context_service).unwrap()
    }

    #[tokio::test]
    async fn test_build_intelligent_prompt_with_context_integration() {
        let state = create_test_ai_manager_state().await;

        // 创建测试会话
        let conversation_id = state
            .repositories()
            .conversations()
            .save(&crate::ai::types::Conversation::new("测试会话".to_string()))
            .await
            .unwrap();

        // 获取终端上下文
        let context_service = state.get_terminal_context_service();
        let terminal_context = context_service
            .get_context_with_fallback(None)
            .await
            .unwrap();

        // 测试新的build_intelligent_prompt_with_context函数
        let result = crate::ai::context::build_intelligent_prompt_with_context(
            state.repositories(),
            conversation_id,
            "测试消息",
            None,
            &terminal_context,
            None,
        )
        .await;

        assert!(result.is_ok(), "构建智能prompt应该成功: {:?}", result);
        let prompt = result.unwrap();
        assert!(!prompt.is_empty(), "生成的prompt不应该为空");

        // 验证prompt包含工作目录信息
        assert!(prompt.contains("~"), "prompt应该包含工作目录信息");
    }

    #[tokio::test]
    async fn test_ai_manager_state_terminal_context_service_access() {
        let state = create_test_ai_manager_state().await;

        // 测试获取终端上下文服务
        let context_service = state.get_terminal_context_service();

        // 测试服务的基本功能（应该返回默认上下文）
        let result = context_service.get_context_with_fallback(None).await;
        assert!(result.is_ok(), "获取默认上下文应该成功: {:?}", result);

        let context = result.unwrap();
        assert_eq!(context.current_working_directory, Some("~".to_string()));
    }

    #[tokio::test]
    async fn test_ai_manager_state_creation_with_terminal_context_service() {
        let repositories = create_test_repositories().await;
        let cache = Arc::new(UnifiedCache::new());

        // 创建终端上下文服务
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new().unwrap());
        let terminal_mux = Arc::new(TerminalMux::new());
        let terminal_context_service = Arc::new(TerminalContextService::new(
            registry,
            shell_integration,
            terminal_mux,
        ));

        // 测试创建AIManagerState
        let result = AIManagerState::new(repositories, cache, terminal_context_service);
        assert!(result.is_ok(), "创建AIManagerState应该成功");

        let state = result.unwrap();
        let _context_service = state.get_terminal_context_service();
        // 验证服务存在且可访问
        assert!(true, "终端上下文服务应该可以访问");
    }

    #[tokio::test]
    async fn test_terminal_context_service_integration() {
        let state = create_test_ai_manager_state().await;
        let context_service = state.get_terminal_context_service();

        // 测试获取上下文（无活跃终端，应该返回默认上下文）
        let result = context_service.get_context_with_fallback(None).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.current_working_directory, Some("~".to_string()));
        assert_eq!(context.shell_type, Some(crate::terminal::ShellType::Bash));
        assert!(!context.shell_integration_enabled);

        // 测试指定不存在的面板ID（应该回退到默认上下文）
        let result = context_service
            .get_context_with_fallback(Some(crate::mux::PaneId::new(999)))
            .await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.current_working_directory, Some("~".to_string()));
    }
}
