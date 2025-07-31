/*!
 * AI功能的Tauri命令接口
 *
 * 统一的AI命令处理规范：
 * 1. 参数顺序：state参数始终放在最后
 * 2. 异步处理：所有命令都是async，统一错误转换
 * 3. 日志记录：每个命令都记录调用和结果日志
 * 4. 状态管理：统一使用AIManagerState访问各组件
 */

use crate::ai::{
    AIAdapterManager, AIClient, AICommandProcessor, AIConfigManager, AIModelConfig, AIRequest,
    AIResponse, CacheManager, CommandExplanation, ContextManager, ErrorAnalysis, ProcessingOptions,
    PromptEngine, StreamChunk,
};
use crate::utils::error::{AppResult, ToTauriResult};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

/// AI管理器状态
///
/// 统一状态管理规范：
/// 1. 所有组件使用Arc<Mutex<T>>包装，支持并发访问
/// 2. 提供统一的初始化和访问方法
/// 3. 包含配置验证和错误处理
/// 4. 支持组件间的依赖注入
pub struct AIManagerState {
    pub config_manager: Arc<Mutex<AIConfigManager>>,
    pub adapter_manager: Arc<Mutex<AIAdapterManager>>,
    pub prompt_engine: Arc<Mutex<PromptEngine>>,
    pub context_manager: Arc<Mutex<ContextManager>>,
    pub cache_manager: Arc<Mutex<CacheManager>>,
    pub command_processor: Arc<Mutex<AICommandProcessor>>,
}

impl AIManagerState {
    /// 创建新的AI管理器状态
    ///
    /// 统一初始化规范：
    /// - 按依赖顺序初始化各组件
    /// - 验证配置和依赖关系
    /// - 提供详细的错误信息
    pub fn new(_app_config_manager: Arc<()>) -> Result<Self, String> {
        info!("开始初始化AI管理器状态");

        // 1. 初始化配置管理器
        debug!("初始化配置管理器");
        let config_manager = Arc::new(Mutex::new(AIConfigManager::new()));

        // 2. 初始化适配器管理器
        debug!("初始化适配器管理器");
        let adapter_manager = Arc::new(Mutex::new(AIAdapterManager::new()));

        // 3. 初始化提示词引擎
        debug!("初始化提示词引擎");
        let prompt_engine = Arc::new(Mutex::new(PromptEngine::new()));

        // 4. 初始化上下文管理器
        debug!("初始化上下文管理器");
        let context_manager = Arc::new(Mutex::new(ContextManager::new(10000))); // 大容量历史记录

        // 5. 初始化缓存管理器
        debug!("初始化缓存管理器");
        let cache_manager = Arc::new(Mutex::new(CacheManager::default()));

        // 6. 初始化命令处理器（依赖其他组件）
        debug!("初始化命令处理器");
        let command_processor = Arc::new(Mutex::new(AICommandProcessor::new(
            config_manager.clone(),
            adapter_manager.clone(),
            prompt_engine.clone(),
            context_manager.clone(),
            cache_manager.clone(),
        )));

        let state = Self {
            config_manager: config_manager.clone(),
            adapter_manager: adapter_manager.clone(),
            prompt_engine,
            context_manager,
            cache_manager,
            command_processor,
        };

        // 7. 重新加载已保存的模型并创建适配器
        debug!("重新加载已保存的AI模型");
        let runtime =
            tokio::runtime::Runtime::new().map_err(|e| format!("创建异步运行时失败: {}", e))?;
        runtime.block_on(async {
            let models = {
                let config_manager = config_manager.lock().await;
                config_manager.get_models().await.unwrap_or_default()
            };

            if !models.is_empty() {
                info!("发现 {} 个已保存的AI模型，开始重新创建适配器", models.len());
                let mut adapter_manager = adapter_manager.lock().await;

                for config in models {
                    info!(
                        "重新创建适配器: {}, provider: {:?}",
                        config.id, config.provider
                    );

                    let adapter: Arc<dyn crate::ai::AIAdapter> = match AIClient::new(config.clone())
                    {
                        Ok(client) => Arc::new(client),
                        Err(e) => {
                            error!("创建AI客户端失败，跳过此模型: {}", e);
                            continue; // 跳过这个模型
                        }
                    };

                    adapter_manager.register_adapter(config.id.clone(), adapter);
                    debug!("适配器重新注册完成: {}", config.id);
                }

                info!("所有AI模型适配器重新创建完成");
            } else {
                debug!("没有发现已保存的AI模型");
            }
        });

        info!("AI管理器状态初始化完成");
        Ok(state)
    }

    /// 验证状态完整性
    pub async fn validate(&self) -> Result<(), String> {
        info!("开始验证AI管理器状态");

        // 验证各组件是否可访问
        let _config = self.config_manager.lock().await;
        let _adapter = self.adapter_manager.lock().await;
        let _prompt = self.prompt_engine.lock().await;
        let _context = self.context_manager.lock().await;
        let _cache = self.cache_manager.lock().await;
        let _processor = self.command_processor.lock().await;

        info!("AI管理器状态验证通过");
        Ok(())
    }

    /// 统一的组件访问方法
    ///
    /// 提供带超时和错误处理的组件访问
    pub async fn with_config_manager<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut AIConfigManager) -> AppResult<R>,
    {
        let mut manager = self.config_manager.lock().await;
        f(&mut manager).to_tauri()
    }

    pub async fn with_adapter_manager<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut AIAdapterManager) -> AppResult<R>,
    {
        let mut manager = self.adapter_manager.lock().await;
        f(&mut manager).to_tauri()
    }

    pub async fn with_cache_manager<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut CacheManager) -> AppResult<R>,
    {
        let mut manager = self.cache_manager.lock().await;
        f(&mut manager).to_tauri()
    }

    pub async fn with_context_manager<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut ContextManager) -> AppResult<R>,
    {
        let mut manager = self.context_manager.lock().await;
        f(&mut manager).to_tauri()
    }

    pub async fn with_command_processor<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut AICommandProcessor) -> AppResult<R>,
    {
        let mut processor = self.command_processor.lock().await;
        f(&mut processor).to_tauri()
    }
}

// ===== AI配置管理命令 =====

#[tauri::command]
pub async fn get_ai_models_old(
    state: State<'_, AIManagerState>,
) -> Result<Vec<AIModelConfig>, String> {
    let config_manager = state.config_manager.lock().await;
    match config_manager.get_models().await {
        Ok(models) => Ok(models),
        Err(e) => Err(e.to_string()),
    }
}

/// 添加AI模型配置
#[tauri::command]
pub async fn add_ai_model(
    config: AIModelConfig,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    info!("添加AI模型: {}, provider: {:?}", config.id, config.provider);

    let config_manager = state.config_manager.lock().await;
    let mut adapter_manager = state.adapter_manager.lock().await;

    // 添加配置
    config_manager
        .add_model(config.clone())
        .await
        .map_err(|e| e.to_string())?;

    info!("模型配置已添加，开始创建适配器: {}", config.id);

    // 创建对应的适配器
    let adapter: Arc<dyn crate::ai::AIAdapter> = match AIClient::new(config.clone()) {
        Ok(client) => Arc::new(client),
        Err(e) => {
            error!("创建AI客户端失败: {}", e);
            return Err(format!(
                "AI配置错误 ({}): Failed to create AI client: {e}",
                config.id
            ));
        }
    };

    info!("适配器创建成功，开始注册: {}", config.id);
    adapter_manager.register_adapter(config.id.clone(), adapter);
    info!("适配器注册完成: {}", config.id);

    Ok(())
}

/// 更新AI模型配置
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn update_ai_model(
    model_id: String,
    updates: serde_json::Value,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    info!("开始更新AI模型: id={}", model_id);

    let config_manager = state.config_manager.lock().await;

    // 获取现有配置
    let existing_config = match config_manager.get_model(&model_id).await {
        Ok(Some(config)) => {
            debug!("找到现有AI模型配置: {}", model_id);
            config.clone()
        }
        Ok(None) => {
            error!("未找到AI模型: {}", model_id);
            return Err(format!("Model with ID '{model_id}' not found"));
        }
        Err(e) => {
            error!("获取AI模型配置失败: {}", e);
            return Err(format!("获取模型配置失败: {}", e));
        }
    };

    // 将现有配置序列化为JSON，然后合并更新
    let mut config_json = match serde_json::to_value(&existing_config) {
        Ok(json) => json,
        Err(e) => {
            error!("序列化现有配置失败: {}, 错误: {}", model_id, e);
            return Err(format!("Failed to serialize existing config: {e}"));
        }
    };

    // 合并更新字段
    if let (serde_json::Value::Object(ref mut config_obj), serde_json::Value::Object(updates_obj)) =
        (&mut config_json, updates)
    {
        for (key, value) in updates_obj {
            debug!("更新配置字段: {} = {:?}", key, value);
            config_obj.insert(key, value);
        }
    }

    // 反序列化为AIModelConfig
    let updated_config: AIModelConfig = match serde_json::from_value(config_json) {
        Ok(config) => config,
        Err(e) => {
            error!("反序列化更新配置失败: {}, 错误: {}", model_id, e);
            return Err(format!("Failed to deserialize updated config: {e}"));
        }
    };

    match config_manager.update_model(&model_id, updated_config).await {
        Ok(_) => {
            info!("AI模型更新完成: {}", model_id);
            Ok(())
        }
        Err(e) => {
            error!("更新AI模型失败: {}, 错误: {}", model_id, e);
            Err(e.to_string())
        }
    }
}

/// 删除AI模型配置
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn remove_ai_model(
    model_id: String,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    info!("开始删除AI模型: id={}", model_id);

    let config_manager = state.config_manager.lock().await;
    let mut adapter_manager = state.adapter_manager.lock().await;

    match config_manager.remove_model(&model_id).await {
        Ok(_) => {
            debug!("AI模型配置删除成功: {}", model_id);
            // 同时移除对应的适配器
            adapter_manager.remove_adapter(&model_id);
            info!("AI模型删除完成: {}", model_id);
            Ok(())
        }
        Err(e) => {
            error!("删除AI模型失败: {}, 错误: {}", model_id, e);
            Err(e.to_string())
        }
    }
}

/// 测试AI模型连接
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn test_ai_connection(
    model_id: String,
    state: State<'_, AIManagerState>,
) -> Result<bool, String> {
    info!("开始测试AI模型连接: id={}", model_id);

    // 创建一个简单的测试请求
    let _ai_request = AIRequest {
        request_type: crate::ai::AIRequestType::Chat,
        content: "Hello".to_string(),
        context: None,
        options: Some(crate::ai::AIRequestOptions {
            max_tokens: Some(10),
            temperature: Some(0.1),
            stream: Some(false),
        }),
    };

    let options = ProcessingOptions {
        preferred_model_id: Some(model_id.clone()),
        timeout: std::time::Duration::from_secs(10), // 测试超时
        max_retries: 0,                              // 测试时不重试
        ..Default::default()
    };

    // 首先检查模型是否存在
    let model_exists = {
        let config_manager = state.config_manager.lock().await;
        matches!(config_manager.get_model(&model_id).await, Ok(Some(_)))
    };

    if !model_exists {
        error!("AI模型不存在: {}", model_id);
        return Err(format!("模型 {model_id} 不存在"));
    }

    // 检查适配器是否可用
    let adapter_available = {
        let adapter_manager = state.adapter_manager.lock().await;
        adapter_manager.get_adapter(&model_id).is_some()
    };

    if !adapter_available {
        error!("AI模型适配器不可用: {}", model_id);
        return Err(format!("模型 {model_id} 的适配器不可用"));
    }

    debug!("开始执行AI模型连接测试: {}", model_id);

    // 使用命令处理器测试连接
    let result = {
        let processor = state.command_processor.lock().await;
        processor.process_request(_ai_request, options).await
    };

    match result {
        Ok(_) => {
            info!("AI模型连接测试成功: {}", model_id);
            Ok(true)
        }
        Err(e) => {
            error!("AI模型连接测试失败: {}, 错误: {}", model_id, e);
            // 根据错误类型返回更详细的信息
            let error_msg = e.to_string();
            if error_msg.contains("认证失败") {
                Err("认证失败，请检查API密钥".to_string())
            } else if error_msg.contains("网络连接错误") {
                Err("网络连接失败，请检查网络和API地址".to_string())
            } else if error_msg.contains("配置错误") {
                Err("配置错误，请检查模型配置".to_string())
            } else if error_msg.contains("请求超时") {
                Err("连接超时，请检查网络连接".to_string())
            } else {
                Err(format!("连接测试失败: {e}"))
            }
        }
    }
}

// ===== AI功能命令 =====

/// 发送聊天消息
#[tauri::command]
pub async fn send_chat_message(
    message: String,
    model_id: Option<String>,
    state: State<'_, AIManagerState>,
) -> Result<AIResponse, String> {
    info!("发送聊天消息: model={:?}", model_id);

    // 创建聊天请求
    let ai_request = AIRequest {
        request_type: crate::ai::AIRequestType::Chat,
        content: message.clone(),
        context: None,
        options: None,
    };

    // 配置处理选项
    let options = ProcessingOptions {
        preferred_model_id: model_id.clone(),
        ..Default::default()
    };

    // 使用命令处理器处理请求
    let result = {
        let processor = state.command_processor.lock().await;
        processor.process_request(ai_request, options).await
    };

    match result {
        Ok(result) => Ok(result.response),
        Err(e) => Err(e.to_string()),
    }
}

/// 发送流式聊天消息 (使用Channel实现真正的双向流式通信)
///
/// 统一命令处理规范：
/// - 使用Tauri Channel实现真正的流式通信
/// - 支持前端取消请求
/// - 实时传输AI响应数据块
#[tauri::command]
pub async fn stream_chat_message_with_channel(
    message: String,
    model_id: Option<String>,
    channel: tauri::ipc::Channel<StreamChunk>,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    // 检查模型配置
    let available_models = {
        let config_manager = state.config_manager.lock().await;
        config_manager
            .get_models()
            .await
            .map_err(|e| e.to_string())?
    };

    if available_models.is_empty() {
        return Err("没有配置任何AI模型，请先在设置中添加模型".to_string());
    }

    // 创建聊天请求
    let ai_request = AIRequest {
        request_type: crate::ai::AIRequestType::Chat,
        content: message.clone(),
        context: None,
        options: Some(crate::ai::AIRequestOptions {
            max_tokens: None,
            temperature: None,
            stream: Some(true), // 启用流式响应
        }),
    };

    // 配置处理选项
    let options = ProcessingOptions {
        preferred_model_id: model_id.clone(),
        ..Default::default()
    };

    // 使用命令处理器处理流式请求
    let stream_result = {
        let processor = state.command_processor.lock().await;
        processor.process_stream_request(ai_request, options).await
    };

    match stream_result {
        Ok((mut stream, _selected_model_id)) => {
            use futures::StreamExt;

            // 处理真正的流式响应
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        // 通过Channel实时发送数据块到前端
                        if let Err(_e) = channel.send(chunk.clone()) {
                            break;
                        }

                        // 如果流式响应完成，退出循环
                        if chunk.is_complete {
                            break;
                        }
                    }
                    Err(e) => {
                        // 发送错误信息
                        let error_chunk = StreamChunk {
                            content: format!("Error: {}", e),
                            is_complete: true,
                            metadata: Some({
                                let mut meta = std::collections::HashMap::new();
                                meta.insert("error".to_string(), serde_json::Value::Bool(true));
                                meta
                            }),
                        };
                        let _ = channel.send(error_chunk);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            // 发送错误信息
            let error_chunk = StreamChunk {
                content: format!("Error: {}", e),
                is_complete: true,
                metadata: Some({
                    let mut meta = std::collections::HashMap::new();
                    meta.insert("error".to_string(), serde_json::Value::Bool(true));
                    meta
                }),
            };
            let _ = channel.send(error_chunk);
        }
    }

    Ok(())
}

/// 解释命令
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn explain_command(
    command: String,
    context: Option<serde_json::Value>,
    state: State<'_, AIManagerState>,
) -> Result<CommandExplanation, String> {
    info!("开始解释命令: command={}", command);

    // 创建命令解释请求
    let ai_request = AIRequest {
        request_type: crate::ai::AIRequestType::Explanation,
        content: command.clone(),
        context: context.and_then(|ctx| serde_json::from_value(ctx).ok()),
        options: None,
    };

    // 使用命令处理器处理请求
    let options = ProcessingOptions::default();

    debug!("配置命令解释请求处理选项: {:?}", options);

    let result = {
        let processor = state.command_processor.lock().await;
        processor.process_request(ai_request, options).await
    };

    match result {
        Ok(result) => {
            debug!(
                "命令解释AI响应成功: response_len={}",
                result.response.content.len()
            );
            // 解析AI响应为结构化的命令解释
            let parsed = parse_command_explanation(&result.response.content, &command);
            info!("命令解释完成: command={}", command);
            Ok(parsed)
        }
        Err(e) => {
            error!("命令解释失败: command={}, 错误: {}", command, e);
            Err(e.to_string())
        }
    }
}

/// 分析错误
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn analyze_error(
    error: String,
    command: String,
    context: Option<serde_json::Value>,
    state: State<'_, AIManagerState>,
) -> Result<ErrorAnalysis, String> {
    info!(
        "开始分析错误: command={}, error_len={}",
        command,
        error.len()
    );

    // 创建错误分析请求
    let content = format!("命令: {command}\n错误: {error}");
    let ai_request = AIRequest {
        request_type: crate::ai::AIRequestType::ErrorAnalysis,
        content: content.clone(),
        context: context.and_then(|ctx| serde_json::from_value(ctx).ok()),
        options: None,
    };

    // 使用命令处理器处理请求
    let options = ProcessingOptions::default();

    debug!("配置错误分析请求处理选项: {:?}", options);

    let result = {
        let processor = state.command_processor.lock().await;
        processor.process_request(ai_request, options).await
    };

    match result {
        Ok(result) => {
            debug!(
                "错误分析AI响应成功: response_len={}",
                result.response.content.len()
            );
            // 解析AI响应为结构化的错误分析
            let parsed = parse_error_analysis(&result.response.content, &error, &command);
            info!("错误分析完成: command={}", command);
            Ok(parsed)
        }
        Err(e) => {
            error!("错误分析失败: command={}, 错误: {}", command, e);
            Err(e.to_string())
        }
    }
}

// ===== AI设置管理命令 =====

/// 获取AI模型列表
///
/// 统一命令处理规范：
/// - 参数顺序：state参数在最后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn get_ai_models(state: State<'_, AIManagerState>) -> Result<Vec<AIModelConfig>, String> {
    info!("开始获取AI模型列表");

    let config_manager = state.config_manager.lock().await;
    let models = config_manager
        .get_models()
        .await
        .map_err(|e| e.to_string())?;

    debug!("AI模型获取成功: models_count={}", models.len());
    info!("AI模型获取完成");

    Ok(models)
}

/// 获取用户前置提示词
///
/// 统一命令处理规范：
/// - 参数顺序：state参数在最后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn get_user_prefix_prompt(
    _state: State<'_, AIManagerState>,
) -> Result<Option<String>, String> {
    info!("开始获取用户前置提示词");

    // 暂时返回None，因为新配置结构中没有用户前缀提示词字段
    let prefix_prompt: Option<String> = None;

    info!("用户前置提示词获取完成: {:?}", prefix_prompt.is_some());
    Ok(prefix_prompt)
}

/// 设置用户前置提示词
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn set_user_prefix_prompt(
    prompt: Option<String>,
    _state: State<'_, AIManagerState>,
) -> Result<(), String> {
    info!("开始设置用户前置提示词: {:?}", prompt.is_some());

    // 暂时不做任何操作，因为新配置结构中没有用户前缀提示词字段
    info!("用户前置提示词设置完成（暂时不支持）");
    Ok(())
}

// ===== 上下文管理命令 =====

/// 获取终端上下文
///
/// 统一命令处理规范：
/// - 参数顺序：state参数在最后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn get_terminal_context(
    state: State<'_, AIManagerState>,
) -> Result<serde_json::Value, String> {
    info!("开始获取终端上下文");

    let context_manager = state.context_manager.lock().await;
    let context = context_manager.get_context();

    match serde_json::to_value(context) {
        Ok(value) => {
            debug!("终端上下文序列化成功");
            info!("终端上下文获取完成");
            Ok(value)
        }
        Err(e) => {
            error!("终端上下文序列化失败: 错误: {}", e);
            Err(e.to_string())
        }
    }
}

/// 更新终端上下文
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn update_terminal_context(
    context: serde_json::Value,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    info!("开始更新终端上下文");

    let mut context_manager = state.context_manager.lock().await;

    // 解析并更新上下文
    match serde_json::from_value::<crate::ai::AIContext>(context) {
        Ok(ai_context) => {
            debug!("终端上下文解析成功");

            if let Some(wd) = ai_context.working_directory {
                debug!("更新工作目录: {}", wd);
                context_manager.update_working_directory(wd);
            }
            if let Some(cmd) = ai_context.current_command {
                debug!("更新当前命令: {}", cmd);
                context_manager.set_current_command(Some(cmd));
            }
            if let Some(output) = ai_context.last_output {
                debug!("更新最后输出: len={}", output.len());
                context_manager.set_last_output(Some(output));
            }
            if let Some(env) = ai_context.environment {
                debug!("更新环境变量: count={}", env.len());
                context_manager.update_environment(env);
            }

            info!("终端上下文更新完成");
            Ok(())
        }
        Err(e) => {
            error!("终端上下文解析失败: 错误: {}", e);
            Err(format!("Failed to parse context: {e}"))
        }
    }
}

// ===== 缓存管理命令 =====

#[tauri::command]
pub async fn clear_ai_cache(state: State<'_, AIManagerState>) -> Result<(), String> {
    let mut cache_manager = state.cache_manager.lock().await;
    cache_manager.clear().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_ai_cache_stats(
    state: State<'_, AIManagerState>,
) -> Result<crate::ai::AICacheStats, String> {
    let cache_manager = state.cache_manager.lock().await;
    cache_manager
        .get_stats()
        .map(|stats| stats.into())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cleanup_expired_cache(state: State<'_, AIManagerState>) -> Result<usize, String> {
    let mut cache_manager = state.cache_manager.lock().await;
    cache_manager.cleanup_expired().map_err(|e| e.to_string())
}

// ===== 处理器状态命令 =====

#[tauri::command]
pub async fn get_ai_processor_status(
    state: State<'_, AIManagerState>,
) -> Result<crate::ai::ProcessorStatus, String> {
    let processor = state.command_processor.lock().await;
    processor.get_status().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_ai_cache_monitor_stats(
    state: State<'_, AIManagerState>,
) -> Result<crate::ai::CacheMonitorStats, String> {
    let cache_manager = state.cache_manager.lock().await;
    cache_manager.get_monitor_stats().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reset_ai_cache_monitor(state: State<'_, AIManagerState>) -> Result<(), String> {
    let mut cache_manager = state.cache_manager.lock().await;
    cache_manager.reset_monitor().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn smart_cleanup_ai_cache(state: State<'_, AIManagerState>) -> Result<usize, String> {
    let mut cache_manager = state.cache_manager.lock().await;
    cache_manager.smart_cleanup().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_ai_cache_strategy(
    strategy: String,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    let cache_strategy = match strategy.as_str() {
        "time-based" => crate::ai::CacheStrategy::TimeBasedTTL,
        "frequency-based" => crate::ai::CacheStrategy::FrequencyBased,
        "content-similarity" => crate::ai::CacheStrategy::ContentSimilarity,
        "hybrid" => crate::ai::CacheStrategy::Hybrid,
        _ => return Err("无效的缓存策略".to_string()),
    };

    let mut cache_manager = state.cache_manager.lock().await;
    cache_manager.set_strategy(cache_strategy);
    Ok(())
}

#[tauri::command]
pub async fn get_ai_cache_strategy(state: State<'_, AIManagerState>) -> Result<String, String> {
    let cache_manager = state.cache_manager.lock().await;
    let strategy = match cache_manager.get_strategy() {
        crate::ai::CacheStrategy::TimeBasedTTL => "time-based",
        crate::ai::CacheStrategy::FrequencyBased => "frequency-based",
        crate::ai::CacheStrategy::ContentSimilarity => "content-similarity",
        crate::ai::CacheStrategy::Hybrid => "hybrid",
    };
    Ok(strategy.to_string())
}

#[tauri::command]
pub async fn get_ai_cache_performance_metrics(
    state: State<'_, AIManagerState>,
) -> Result<crate::ai::CachePerformanceMetrics, String> {
    let cache_manager = state.cache_manager.lock().await;
    cache_manager
        .get_performance_metrics()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_ai_cache_optimization_suggestions(
    state: State<'_, AIManagerState>,
) -> Result<Vec<String>, String> {
    let cache_manager = state.cache_manager.lock().await;
    Ok(cache_manager.get_optimization_suggestions())
}

#[tauri::command]
pub async fn configure_ai_cache(
    default_ttl: Option<u64>,
    max_entries: Option<usize>,
    state: State<'_, AIManagerState>,
) -> Result<(), String> {
    let mut cache_manager = state.cache_manager.lock().await;

    if let Some(ttl) = default_ttl {
        cache_manager.set_default_ttl(ttl);
    }

    if let Some(max) = max_entries {
        cache_manager.set_max_entries(max);
    }

    Ok(())
}

// ===== 响应解析函数 =====

/// 解析命令解释响应
fn parse_command_explanation(content: &str, command: &str) -> CommandExplanation {
    let mut explanation = content.to_string();
    let breakdown;
    let risks;
    let alternatives;

    // 尝试解析结构化内容
    if let Some(parsed) = try_parse_structured_explanation(content) {
        explanation = parsed.explanation;
        breakdown = parsed.breakdown;
        risks = parsed.risks;
        alternatives = parsed.alternatives;
    } else {
        // 如果无法解析结构化内容，尝试从文本中提取信息
        breakdown = extract_command_breakdown(content, command);
        risks = extract_risk_warnings(content);
        alternatives = extract_command_alternatives(content);
    }

    CommandExplanation {
        command: command.to_string(),
        explanation,
        breakdown,
        risks,
        alternatives,
    }
}

/// 解析错误分析响应
fn parse_error_analysis(content: &str, error: &str, command: &str) -> ErrorAnalysis {
    let mut analysis = content.to_string();
    let possible_causes;
    let solutions;
    let related_docs;

    // 尝试解析结构化内容
    if let Some(parsed) = try_parse_structured_error_analysis(content) {
        analysis = parsed.analysis;
        possible_causes = parsed.possible_causes;
        solutions = parsed.solutions;
        related_docs = parsed.related_docs;
    } else {
        // 如果无法解析结构化内容，尝试从文本中提取信息
        possible_causes = extract_possible_causes(content);
        solutions = extract_error_solutions(content);
        related_docs = extract_related_docs(content);
    }

    ErrorAnalysis {
        error: error.to_string(),
        command: command.to_string(),
        analysis,
        possible_causes,
        solutions,
        related_docs,
    }
}

// ===== 结构化解析辅助函数 =====

/// 尝试解析结构化的命令解释（JSON格式）
fn try_parse_structured_explanation(content: &str) -> Option<CommandExplanation> {
    // 尝试查找JSON块
    if let Some(json_start) = content.find("```json") {
        if let Some(json_end) = content[json_start..].find("```") {
            let json_content = &content[json_start + 7..json_start + json_end];
            if let Ok(parsed) = serde_json::from_str::<CommandExplanation>(json_content) {
                return Some(parsed);
            }
        }
    }
    None
}

/// 尝试解析结构化的错误分析（JSON格式）
fn try_parse_structured_error_analysis(content: &str) -> Option<ErrorAnalysis> {
    // 尝试查找JSON块
    if let Some(json_start) = content.find("```json") {
        if let Some(json_end) = content[json_start..].find("```") {
            let json_content = &content[json_start + 7..json_start + json_end];
            if let Ok(parsed) = serde_json::from_str::<ErrorAnalysis>(json_content) {
                return Some(parsed);
            }
        }
    }
    None
}

// ===== 文本提取辅助函数 =====

/// 从文本中提取命令分解信息
fn extract_command_breakdown(content: &str, command: &str) -> Option<Vec<crate::ai::CommandPart>> {
    let mut parts = Vec::new();

    // 简单的启发式方法：查找包含命令部分的行
    let lines: Vec<&str> = content.lines().collect();
    for line in lines {
        if line.contains("- ") || line.contains("* ") {
            // 尝试解析列表项
            if let Some(part_info) = parse_command_part_line(line, command) {
                parts.push(part_info);
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts)
    }
}

/// 从文本中提取风险警告
fn extract_risk_warnings(content: &str) -> Option<Vec<crate::ai::RiskWarning>> {
    let mut warnings = Vec::new();

    // 查找包含风险关键词的行
    let risk_keywords = [
        "危险", "警告", "注意", "风险", "warning", "danger", "caution", "risk",
    ];
    let lines: Vec<&str> = content.lines().collect();

    for line in lines {
        let lower_line = line.to_lowercase();
        if risk_keywords
            .iter()
            .any(|&keyword| lower_line.contains(keyword))
        {
            let level = determine_risk_level(line);
            warnings.push(crate::ai::RiskWarning {
                level,
                description: line.trim().to_string(),
            });
        }
    }

    if warnings.is_empty() {
        None
    } else {
        Some(warnings)
    }
}

/// 从文本中提取命令替代方案
fn extract_command_alternatives(content: &str) -> Option<Vec<crate::ai::CommandAlternative>> {
    let mut alternatives = Vec::new();

    // 查找包含替代方案关键词的行
    let alt_keywords = [
        "替代",
        "代替",
        "alternative",
        "instead",
        "better",
        "recommend",
    ];
    let lines: Vec<&str> = content.lines().collect();

    for line in lines {
        let lower_line = line.to_lowercase();
        if alt_keywords
            .iter()
            .any(|&keyword| lower_line.contains(keyword))
        {
            if let Some(alt) = parse_alternative_line(line) {
                alternatives.push(alt);
            }
        }
    }

    if alternatives.is_empty() {
        None
    } else {
        Some(alternatives)
    }
}

/// 从文本中提取可能的错误原因
fn extract_possible_causes(content: &str) -> Vec<String> {
    let mut causes = Vec::new();

    // 查找包含原因关键词的行
    let cause_keywords = [
        "原因", "因为", "由于", "cause", "because", "due to", "reason",
    ];
    let lines: Vec<&str> = content.lines().collect();

    for line in lines {
        let lower_line = line.to_lowercase();
        if cause_keywords
            .iter()
            .any(|&keyword| lower_line.contains(keyword))
        {
            causes.push(line.trim().to_string());
        }
    }

    causes
}

/// 从文本中提取错误解决方案
fn extract_error_solutions(content: &str) -> Vec<crate::ai::ErrorSolution> {
    let mut solutions = Vec::new();

    // 查找包含解决方案关键词的行
    let solution_keywords = [
        "解决",
        "修复",
        "解决方案",
        "solution",
        "fix",
        "resolve",
        "try",
    ];
    let lines: Vec<&str> = content.lines().collect();

    for line in lines {
        let lower_line = line.to_lowercase();
        if solution_keywords
            .iter()
            .any(|&keyword| lower_line.contains(keyword))
        {
            let priority = determine_solution_priority(line);
            let command = extract_command_from_line(line);

            solutions.push(crate::ai::ErrorSolution {
                description: line.trim().to_string(),
                command,
                priority,
            });
        }
    }

    solutions
}

/// 从文本中提取相关文档链接
fn extract_related_docs(content: &str) -> Option<Vec<crate::ai::DocumentLink>> {
    let mut docs = Vec::new();

    // 简单的URL查找（不使用regex）
    let lines: Vec<&str> = content.lines().collect();
    for line in lines {
        if line.contains("http://") || line.contains("https://") {
            // 查找URL的开始和结束
            if let Some(start) = line.find("http") {
                let url_part = &line[start..];
                let end = url_part.find(' ').unwrap_or(url_part.len());
                let url = &url_part[..end];

                docs.push(crate::ai::DocumentLink {
                    title: "相关文档".to_string(), // 简单的标题
                    url: url.to_string(),
                });
            }
        }
    }

    if docs.is_empty() {
        None
    } else {
        Some(docs)
    }
}

// ===== 详细解析辅助函数 =====

/// 解析命令部分行
fn parse_command_part_line(line: &str, command: &str) -> Option<crate::ai::CommandPart> {
    // 移除列表标记
    let cleaned = line
        .trim_start_matches("- ")
        .trim_start_matches("* ")
        .trim();

    // 尝试分割为部分和描述
    if let Some(colon_pos) = cleaned.find(':') {
        let part = cleaned[..colon_pos].trim();
        let description = cleaned[colon_pos + 1..].trim();

        // 检查这个部分是否真的是命令的一部分
        if command.contains(part) {
            return Some(crate::ai::CommandPart {
                part: part.to_string(),
                description: description.to_string(),
            });
        }
    }

    None
}

/// 确定风险级别
fn determine_risk_level(line: &str) -> crate::ai::RiskLevel {
    let lower_line = line.to_lowercase();

    if lower_line.contains("高")
        || lower_line.contains("严重")
        || lower_line.contains("danger")
        || lower_line.contains("critical")
    {
        crate::ai::RiskLevel::High
    } else if lower_line.contains("中")
        || lower_line.contains("warning")
        || lower_line.contains("caution")
    {
        crate::ai::RiskLevel::Medium
    } else {
        crate::ai::RiskLevel::Low
    }
}

/// 解析替代方案行
fn parse_alternative_line(line: &str) -> Option<crate::ai::CommandAlternative> {
    // 简单的启发式方法：查找命令模式
    let cleaned = line.trim();

    // 查找可能的命令（以$开头或包含常见命令）
    if let Some(command) = extract_command_from_line(line) {
        return Some(crate::ai::CommandAlternative {
            command,
            description: cleaned.to_string(),
            reason: "AI推荐的替代方案".to_string(),
        });
    }

    None
}

/// 确定解决方案优先级
fn determine_solution_priority(line: &str) -> crate::ai::SolutionPriority {
    let lower_line = line.to_lowercase();

    if lower_line.contains("首先")
        || lower_line.contains("重要")
        || lower_line.contains("urgent")
        || lower_line.contains("first")
    {
        crate::ai::SolutionPriority::High
    } else if lower_line.contains("其次")
        || lower_line.contains("然后")
        || lower_line.contains("also")
        || lower_line.contains("alternatively")
    {
        crate::ai::SolutionPriority::Medium
    } else {
        crate::ai::SolutionPriority::Low
    }
}

/// 从行中提取命令
fn extract_command_from_line(line: &str) -> Option<String> {
    // 查找代码块中的命令
    if let Some(start) = line.find('`') {
        if let Some(end) = line[start + 1..].find('`') {
            let command = &line[start + 1..start + 1 + end];
            if !command.is_empty() {
                return Some(command.to_string());
            }
        }
    }

    // 查找以$开头的命令
    if let Some(dollar_pos) = line.find('$') {
        let after_dollar = &line[dollar_pos + 1..];
        if let Some(space_pos) = after_dollar.find(' ') {
            let command = after_dollar[..space_pos].trim();
            if !command.is_empty() {
                return Some(format!("${command}"));
            }
        } else {
            let command = after_dollar.trim();
            if !command.is_empty() {
                return Some(format!("${command}"));
            }
        }
    }

    None
}
