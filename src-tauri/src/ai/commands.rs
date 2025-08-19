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

/// 构建带上下文的prompt（专门用于AI推理）
#[tauri::command]
pub async fn build_prompt_with_context(
    conversation_id: i64,
    current_message: String,
    up_to_message_id: Option<i64>,
    state: State<'_, AIManagerState>,
) -> Result<String, String> {
    info!(
        "构建prompt: conversation_id={}, current_message length={}, up_to_message_id={:?}",
        conversation_id,
        current_message.len(),
        up_to_message_id
    );

    if conversation_id <= 0 {
        return Err("无效的会话ID".to_string());
    }

    if current_message.trim().is_empty() {
        return Err("当前消息不能为空".to_string());
    }

    let repositories = state.repositories();

    // 获取用户前置提示词
    let user_prefix_prompt = repositories
        .ai_models()
        .get_user_prefix_prompt()
        .await
        .to_tauri()?;

    // 获取历史消息
    let config = crate::ai::types::AIConfig::default();
    let messages = crate::ai::context::build_context_for_request(
        repositories,
        conversation_id,
        up_to_message_id,
        &config,
    )
    .await
    .to_tauri()?;

    // 构建prompt
    let mut prompt_parts = Vec::new();

    // 添加用户前置提示词（如果存在）
    if let Some(prefix_prompt) = user_prefix_prompt {
        if !prefix_prompt.trim().is_empty() {
            prompt_parts.push(format!("【用户前置提示词】\n{}\n", prefix_prompt));
        }
    }

    let prompt = if messages.len() > 0 {
        // 有历史对话的情况
        let history_context = messages
            .iter()
            .map(|msg| {
                if msg.role == "assistant" && msg.steps_json.is_some() {
                    // 对于AI消息，如果有steps数据，需要包含完整的工具调用过程
                    if let Some(steps_json) = &msg.steps_json {
                        // 尝试解析steps数据以提取完整上下文
                        match serde_json::from_str::<serde_json::Value>(steps_json) {
                            Ok(steps) => {
                                let mut full_content = Vec::new();

                                // 如果steps是数组，遍历每个步骤
                                if let Some(steps_array) = steps.as_array() {
                                    for step in steps_array {
                                        if let Some(step_type) =
                                            step.get("type").and_then(|t| t.as_str())
                                        {
                                            match step_type {
                                                "thinking" => {
                                                    if let Some(content) =
                                                        step.get("content").and_then(|c| c.as_str())
                                                    {
                                                        full_content
                                                            .push(format!("[思考] {}", content));
                                                    }
                                                }
                                                "tool_use" => {
                                                    if let Some(tool_execution) =
                                                        step.get("toolExecution")
                                                    {
                                                        if let (Some(name), Some(params)) = (
                                                            tool_execution
                                                                .get("name")
                                                                .and_then(|n| n.as_str()),
                                                            tool_execution.get("params"),
                                                        ) {
                                                            full_content.push(format!(
                                                                "[工具调用] {} 参数: {}",
                                                                name, params
                                                            ));

                                                            // 如果有工具结果，也添加
                                                            if let Some(result) =
                                                                tool_execution.get("result")
                                                            {
                                                                let result_str =
                                                                    if result.is_string() {
                                                                        result
                                                                            .as_str()
                                                                            .unwrap_or("")
                                                                            .to_string()
                                                                    } else {
                                                                        result.to_string()
                                                                    };
                                                                // 输出完整的工具结果，不进行截断
                                                                full_content.push(format!(
                                                                    "[工具结果] {}",
                                                                    result_str
                                                                ));
                                                            }
                                                        }
                                                    }
                                                }
                                                "text" => {
                                                    if let Some(content) =
                                                        step.get("content").and_then(|c| c.as_str())
                                                    {
                                                        full_content
                                                            .push(format!("[回复] {}", content));
                                                    }
                                                }
                                                _ => {
                                                    // 其他类型的步骤
                                                    if let Some(content) =
                                                        step.get("content").and_then(|c| c.as_str())
                                                    {
                                                        full_content.push(format!(
                                                            "[{}] {}",
                                                            step_type, content
                                                        ));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // 如果成功解析了steps，使用完整内容；否则回退到基础content
                                if !full_content.is_empty() {
                                    format!("{}: {}", msg.role, full_content.join("\n"))
                                } else {
                                    format!("{}: {}", msg.role, msg.content)
                                }
                            }
                            Err(_) => {
                                // 解析失败，使用基础content
                                format!("{}: {}", msg.role, msg.content)
                            }
                        }
                    } else {
                        format!("{}: {}", msg.role, msg.content)
                    }
                } else {
                    // 用户消息或没有steps的消息，直接使用content
                    format!("{}: {}", msg.role, msg.content)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        prompt_parts.push(format!(
            "以下是我们之前的对话历史，请参考这些上下文来回答我的新问题：\n\n【对话历史】\n{}\n\n【当前问题】\n{}\n\n你的首要任务是：精确理解用户当前的意图，查看最近的上下文。严格遵循用户的要求，不要自己想当然的执行操作",
            history_context,
            current_message
        ));

        prompt_parts.join("\n")
    } else {
        // 没有历史对话的情况
        prompt_parts.push(current_message);
        prompt_parts.join("\n")
    };

    info!(
        "prompt构建完成: conversation_id={}, 历史消息数量={}",
        conversation_id,
        messages.len()
    );

    Ok(prompt)
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
