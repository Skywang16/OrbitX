/*!
 * AI功能的Tauri命令接口
 */

use crate::ai::{
    AIModelConfig, AIRequest, AIRequestType, AIResponse, AIService, ChatMessage, ChatMessageType,
    StreamChunk,
};
use crate::storage::sqlite::{AIChatHistoryEntry, ChatHistoryQuery, SqliteManager};
use chrono::Utc;
use std::sync::Arc;
use tauri::{ipc::Channel, State};
use tracing::{error, info};

/// AI管理器状态
pub struct AIManagerState {
    pub ai_service: Arc<AIService>,
    pub sqlite_manager: Option<Arc<SqliteManager>>,
}

impl AIManagerState {
    /// 创建新的AI管理器状态
    pub fn new(sqlite_manager: Option<Arc<SqliteManager>>) -> Result<Self, String> {
        let ai_service = Arc::new(AIService::new(sqlite_manager.clone()));

        Ok(Self {
            ai_service,
            sqlite_manager,
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
) -> Result<(), String> {
    info!("添加AI模型: {}", config.id);

    state
        .ai_service
        .add_model(config)
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

/// 测试AI模型连接
#[tauri::command]
pub async fn test_ai_connection(
    model_id: String,
    state: State<'_, AIManagerState>,
) -> Result<bool, String> {
    info!("测试AI模型连接: {}", model_id);

    state
        .ai_service
        .test_connection(&model_id)
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

// ===== AI功能命令 =====

/// 发送聊天消息
#[tauri::command]
pub async fn send_chat_message(
    message: String,
    model_id: Option<String>,
    state: State<'_, AIManagerState>,
) -> Result<AIResponse, String> {
    let request = AIRequest {
        request_type: AIRequestType::Chat,
        content: message,
        context: None,
        options: None,
    };

    state
        .ai_service
        .send_request(&request, model_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 发送流式聊天消息
#[tauri::command]
pub async fn stream_chat_message_with_channel(
    message: String,
    model_id: Option<String>,
    channel: Channel<StreamChunk>,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    let request = AIRequest {
        request_type: AIRequestType::Chat,
        content: message,
        context: None,
        options: None,
    };

    // 发送开始信号
    let start_chunk = StreamChunk {
        content: String::new(),
        is_complete: false,
        metadata: Some({
            let mut metadata = std::collections::HashMap::new();
            metadata.insert("stream_started".to_string(), serde_json::json!(true));
            metadata
        }),
    };

    if let Err(e) = channel.send(start_chunk) {
        error!("发送开始信号失败: {}", e);
        return Err("发送开始信号失败".to_string());
    }

    // 使用真正的流式请求
    match state
        .ai_service
        .send_stream_request(&request, model_id.as_deref())
        .await
    {
        Ok(mut stream) => {
            use futures::StreamExt;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        if let Err(e) = channel.send(chunk.clone()) {
                            error!("发送流式数据失败: {}", e);
                            return Err("发送流式数据失败".to_string());
                        }

                        if chunk.is_complete {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("流式响应错误: {}", e);

                        let error_chunk = StreamChunk {
                            content: String::new(),
                            is_complete: true,
                            metadata: Some({
                                let mut metadata = std::collections::HashMap::new();
                                metadata.insert(
                                    "error".to_string(),
                                    serde_json::json!({
                                        "message": e.to_string(),
                                        "code": "stream_error"
                                    }),
                                );
                                metadata
                            }),
                        };

                        let _ = channel.send(error_chunk);
                        return Err(e.to_string());
                    }
                }
            }

            Ok(())
        }
        Err(e) => {
            error!("创建流式请求失败: {}", e);

            // 发送错误chunk
            let error_chunk = StreamChunk {
                content: String::new(),
                is_complete: true,
                metadata: Some({
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert(
                        "error".to_string(),
                        serde_json::json!({
                            "message": e.to_string(),
                            "code": "stream_init_error"
                        }),
                    );
                    metadata
                }),
            };

            let _ = channel.send(error_chunk);
            Err(e.to_string())
        }
    }
}

/// 解释命令
#[tauri::command]
pub async fn explain_command(
    command: String,
    _context: Option<serde_json::Value>,
    state: State<'_, AIManagerState>,
) -> Result<AIResponse, String> {
    let prompt = format!("请解释以下命令的作用和用法：{}", command);

    let request = AIRequest {
        request_type: AIRequestType::Explanation,
        content: prompt,
        context: None,
        options: None,
    };

    state
        .ai_service
        .send_request(&request, None)
        .await
        .map_err(|e| e.to_string())
}

/// 分析错误
#[tauri::command]
pub async fn analyze_error(
    error: String,
    command: String,
    _context: Option<serde_json::Value>,
    state: State<'_, AIManagerState>,
) -> Result<AIResponse, String> {
    let prompt = format!(
        "命令 '{}' 执行时出现错误：{}\n请分析错误原因并提供解决方案。",
        command, error
    );

    let request = AIRequest {
        request_type: AIRequestType::ErrorAnalysis,
        content: prompt,
        context: None,
        options: None,
    };

    state
        .ai_service
        .send_request(&request, None)
        .await
        .map_err(|e| e.to_string())
}

// ===== 缓存管理命令 =====

/// 清空AI缓存
#[tauri::command]
pub async fn clear_ai_cache(state: State<'_, AIManagerState>) -> Result<(), String> {
    info!("清空AI缓存");

    state
        .ai_service
        .clear_cache()
        .await
        .map_err(|e| e.to_string())
}

// ===== 聊天历史管理命令 =====

/// 获取聊天历史记录
#[tauri::command]
pub async fn get_chat_history(
    session_id: Option<String>,
    state: State<'_, AIManagerState>,
) -> Result<Vec<ChatMessage>, String> {
    info!("获取聊天历史: session_id={:?}", session_id);

    let sqlite_manager = state
        .sqlite_manager
        .as_ref()
        .ok_or_else(|| "数据库管理器未初始化".to_string())?;

    let query = ChatHistoryQuery {
        session_id,
        model_id: None,
        role: None,
        date_from: None,
        date_to: None,
        limit: Some(1000), // 限制最多返回1000条记录
        offset: None,
    };

    let entries = sqlite_manager
        .get_chat_history(&query)
        .await
        .map_err(|e| e.to_string())?;

    // 转换为前端期望的ChatMessage格式
    let messages: Vec<ChatMessage> = entries
        .into_iter()
        .map(|entry| ChatMessage {
            id: entry.id.unwrap_or(0).to_string(),
            message_type: match entry.role.as_str() {
                "user" => ChatMessageType::User,
                "assistant" => ChatMessageType::Assistant,
                "system" => ChatMessageType::System,
                _ => ChatMessageType::User,
            },
            content: entry.content,
            timestamp: entry.created_at,
            metadata: entry
                .metadata_json
                .and_then(|json| serde_json::from_str(&json).ok()),
        })
        .collect();

    info!("成功获取 {} 条聊天历史记录", messages.len());
    Ok(messages)
}

/// 获取所有会话列表
#[tauri::command]
pub async fn get_chat_sessions(state: State<'_, AIManagerState>) -> Result<Vec<String>, String> {
    let sqlite_manager = state
        .sqlite_manager
        .as_ref()
        .ok_or_else(|| "数据库管理器未初始化".to_string())?;

    sqlite_manager
        .get_chat_sessions()
        .await
        .map_err(|e| e.to_string())
}

/// 保存聊天历史记录
#[tauri::command]
pub async fn save_chat_history(
    messages: Vec<ChatMessage>,
    session_id: Option<String>,
    state: State<'_, AIManagerState>,
) -> Result<String, String> {
    info!(
        "保存聊天历史: {} 条消息, session_id={:?}",
        messages.len(),
        session_id
    );

    let sqlite_manager = state
        .sqlite_manager
        .as_ref()
        .ok_or_else(|| "数据库管理器未初始化".to_string())?;

    // 使用提供的session_id或生成新的
    let final_session_id =
        session_id.unwrap_or_else(|| format!("session_{}", Utc::now().timestamp()));

    // 获取默认模型ID
    let default_model = sqlite_manager
        .get_default_ai_model()
        .await
        .map_err(|e| e.to_string())?;

    let model_id = default_model
        .map(|m| m.id)
        .unwrap_or_else(|| "default".to_string());

    // 保存每条消息
    for message in messages {
        let entry = AIChatHistoryEntry {
            id: None,
            session_id: final_session_id.clone(),
            model_id: model_id.clone(),
            role: match message.message_type {
                ChatMessageType::User => "user".to_string(),
                ChatMessageType::Assistant => "assistant".to_string(),
                ChatMessageType::System => "system".to_string(),
            },
            content: message.content,
            token_count: message
                .metadata
                .as_ref()
                .and_then(|m| m.tokens_used)
                .map(|t| t as i32),
            metadata_json: message
                .metadata
                .map(|m| serde_json::to_string(&m).unwrap_or_default()),
            created_at: message.timestamp,
        };

        sqlite_manager
            .save_chat_message(&entry)
            .await
            .map_err(|e| e.to_string())?;
    }

    info!("成功保存聊天历史到会话: {}", final_session_id);
    Ok(final_session_id)
}

/// 清除聊天历史记录
#[tauri::command]
pub async fn clear_chat_history(
    session_id: Option<String>,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    info!("清除聊天历史: session_id={:?}", session_id);

    let sqlite_manager = state
        .sqlite_manager
        .as_ref()
        .ok_or_else(|| "数据库管理器未初始化".to_string())?;

    let affected_rows = sqlite_manager
        .clear_chat_history(session_id.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    info!("成功清除 {} 条聊天历史记录", affected_rows);
    Ok(())
}

// ===== 占位符命令（保持API兼容性） =====

/// 获取用户前置提示词（占位符）
#[tauri::command]
pub async fn get_user_prefix_prompt() -> Result<Option<String>, String> {
    Ok(None)
}

/// 设置用户前置提示词（占位符）
#[tauri::command]
pub async fn set_user_prefix_prompt(_prompt: Option<String>) -> Result<(), String> {
    Ok(())
}

/// 获取终端上下文（占位符）
#[tauri::command]
pub async fn get_terminal_context() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({}))
}

/// 更新终端上下文（占位符）
#[tauri::command]
pub async fn update_terminal_context(_context: serde_json::Value) -> Result<(), String> {
    Ok(())
}
