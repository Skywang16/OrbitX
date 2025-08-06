/*!
 * AI服务 - 统一管理所有AI功能
 */

use crate::ai::{AIModelConfig, AIProvider, AIRequest, AIResponse, AIStreamResponse, StreamChunk};
use crate::storage::sqlite::SqliteManager;
use crate::utils::error::AppResult;
use anyhow::{anyhow, Context};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

// 重新导入必要的HTTP客户端
use async_openai::{
    config::OpenAIConfig,
    types::{ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs},
    Client as OpenAIClient,
};

/// AI客户端
#[derive(Debug)]
pub struct AIClient {
    config: AIModelConfig,
    http_client: reqwest::Client,
    openai_client: Option<OpenAIClient<OpenAIConfig>>,
}

impl AIClient {
    /// 创建新的AI客户端
    pub fn new(config: AIModelConfig) -> AppResult<Self> {
        info!("创建AI客户端: {}", config.id);

        // 验证配置
        Self::validate_config(&config)?;

        // 创建HTTP客户端
        let mut client_builder = reqwest::Client::builder();

        // 如果超时时间大于0，则设置超时；否则不设置超时（无限制）
        if config.timeout() > 0 {
            client_builder = client_builder.timeout(Duration::from_secs(config.timeout()));
        }

        let http_client = client_builder.build().context("创建HTTP客户端失败")?;

        // 根据提供商创建对应的客户端
        let openai_client = match config.provider {
            AIProvider::OpenAI | AIProvider::Claude => {
                let openai_config = OpenAIConfig::new()
                    .with_api_key(&config.api_key)
                    .with_api_base(&config.api_url);
                Some(OpenAIClient::with_config(openai_config))
            }
            AIProvider::Custom => None,
        };

        Ok(Self {
            config,
            http_client,
            openai_client,
        })
    }

    /// 验证配置
    fn validate_config(config: &AIModelConfig) -> AppResult<()> {
        if config.api_key.is_empty() {
            return Err(anyhow!("API密钥不能为空"));
        }

        if config.api_url.is_empty() {
            return Err(anyhow!("API URL不能为空"));
        }

        if !config.api_url.starts_with("https://") && !config.api_url.starts_with("http://") {
            return Err(anyhow!("API URL格式无效"));
        }

        Ok(())
    }

    /// 发送请求
    pub async fn send_request(&self, request: &AIRequest) -> AppResult<AIResponse> {
        match self.config.provider {
            AIProvider::Custom => self.send_custom_request(request).await,
            _ => self.send_openai_request(request).await,
        }
    }

    /// 发送流式请求
    pub async fn send_stream_request(&self, request: &AIRequest) -> AppResult<AIStreamResponse> {
        match self.config.provider {
            AIProvider::Custom => self.send_custom_stream_request(request).await,
            _ => self.send_openai_stream_request(request).await,
        }
    }

    /// 发送OpenAI兼容请求
    async fn send_openai_request(&self, request: &AIRequest) -> AppResult<AIResponse> {
        let client = self
            .openai_client
            .as_ref()
            .ok_or_else(|| anyhow!("OpenAI客户端未初始化"))?;

        let chat_request = CreateChatCompletionRequestArgs::default()
            .model(&self.config.model)
            .messages(vec![ChatCompletionRequestUserMessageArgs::default()
                .content(request.content.clone())
                .build()?
                .into()])
            .max_tokens(self.config.max_tokens())
            .temperature(self.config.temperature())
            .build()?;

        let response = client
            .chat()
            .create(chat_request)
            .await
            .context("发送OpenAI请求失败")?;

        let content = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.as_ref())
            .ok_or_else(|| anyhow!("AI响应为空"))?;

        Ok(AIResponse {
            content: content.clone(),
            response_type: crate::ai::AIResponseType::Chat,
            suggestions: None,
            metadata: Some(crate::ai::AIResponseMetadata {
                model: Some(self.config.id.clone()),
                tokens_used: None,
                response_time: None,
            }),
            error: None,
        })
    }

    /// 发送自定义请求
    async fn send_custom_request(&self, request: &AIRequest) -> AppResult<AIResponse> {
        let request_body = serde_json::json!({
            "model": self.config.model,
            "messages": [{"role": "user", "content": request.content}],
            "stream": true,
        });

        let response = self
            .http_client
            .post(&self.config.api_url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&request_body)
            .send()
            .await
            .context("发送自定义请求失败")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // 尝试解析错误响应为JSON
            let error_json: Option<serde_json::Value> = serde_json::from_str(&error_text).ok();

            // 提取错误代码和消息
            let (error_code, error_message) = if let Some(json) = &error_json {
                let code = json["error"]["code"].as_str().map(|s| s.to_string());
                let message = json["error"]["message"]
                    .as_str()
                    .unwrap_or("未知错误")
                    .to_string();
                (code, message)
            } else {
                (None, error_text.clone())
            };

            // 返回包含错误信息的响应，而不是抛出错误
            return Ok(AIResponse {
                content: String::new(),
                response_type: crate::ai::AIResponseType::Chat,
                suggestions: None,
                metadata: Some(crate::ai::AIResponseMetadata {
                    model: Some(self.config.id.clone()),
                    tokens_used: None,
                    response_time: None,
                }),
                error: Some(crate::ai::AIErrorInfo {
                    message: error_message,
                    code: error_code,
                    details: None,
                    provider_response: error_json,
                }),
            });
        }

        // 获取原始文本并解析SSE格式
        let response_text = response.text().await.context("获取响应文本失败")?;

        // 解析SSE格式，提取所有content内容
        let mut full_content = String::new();

        for line in response_text.lines() {
            if line.starts_with("data: ") {
                let json_str = &line[6..]; // 去掉 "data: " 前缀

                // 跳过 [DONE] 标志
                if json_str == "[DONE]" {
                    break;
                }

                // 解析JSON
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                    // 提取delta.content内容
                    if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                        if !content.is_empty() {
                            full_content.push_str(content);
                        }
                    }
                }
            }
        }

        let content = if full_content.is_empty() {
            return Err(anyhow!("未找到有效的content内容"));
        } else {
            full_content
        };

        Ok(AIResponse {
            content: content.to_string(),
            response_type: crate::ai::AIResponseType::Chat,
            suggestions: None,
            metadata: Some(crate::ai::AIResponseMetadata {
                model: Some(self.config.id.clone()),
                tokens_used: None,
                response_time: None,
            }),
            error: None,
        })
    }

    /// 发送OpenAI流式请求（简化实现）
    async fn send_openai_stream_request(
        &self,
        _request: &AIRequest,
    ) -> AppResult<AIStreamResponse> {
        // 简化实现：暂时返回错误，后续可以实现
        Err(anyhow!("流式请求暂未实现"))
    }

    /// 发送自定义流式请求
    async fn send_custom_stream_request(&self, request: &AIRequest) -> AppResult<AIStreamResponse> {
        use futures::stream::StreamExt;
        use reqwest_eventsource::{Event, EventSource};

        let request_body = serde_json::json!({
            "model": self.config.model,
            "messages": [{"role": "user", "content": request.content}],
            "stream": true,
        });

        let client = reqwest::Client::new();
        let req_builder = client
            .post(&self.config.api_url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&request_body);

        let mut event_source = EventSource::new(req_builder).context("创建EventSource失败")?;

        let stream = async_stream::stream! {
            while let Some(event) = event_source.next().await {
                match event {
                    Ok(Event::Open) => {
                        // SSE连接已建立
                    }
                    Ok(Event::Message(message)) => {
                        let data = message.data;

                        // 跳过 [DONE] 标志
                        if data == "[DONE]" {
                            yield Ok(StreamChunk {
                                content: String::new(),
                                is_complete: true,
                                metadata: None,
                            });
                            break;
                        }

                        // 解析JSON并提取content
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
                            if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                if !content.is_empty() {
                                    yield Ok(StreamChunk {
                                        content: content.to_string(),
                                        is_complete: false,
                                        metadata: None,
                                    });
                                }
                            }
                        }
                    }
                    Err(err) => {
                        error!("SSE错误: {}", err);
                        yield Err(crate::utils::error::AppError::from(anyhow!("SSE错误: {}", err)));
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

/// AI服务 - 统一管理所有AI功能
pub struct AIService {
    /// 模型配置
    models: RwLock<HashMap<String, AIModelConfig>>,
    /// AI客户端
    clients: RwLock<HashMap<String, Arc<AIClient>>>,
    /// 存储管理器
    storage: Option<Arc<SqliteManager>>,
}

impl AIService {
    /// 创建新的AI服务
    pub fn new(storage: Option<Arc<SqliteManager>>) -> Self {
        Self {
            models: RwLock::new(HashMap::new()),
            clients: RwLock::new(HashMap::new()),
            storage,
        }
    }

    /// 初始化服务，从存储加载模型配置
    pub async fn initialize(&self) -> AppResult<()> {
        if let Some(storage) = &self.storage {
            let models = storage
                .get_ai_models()
                .await
                .context("从存储加载AI模型失败")?;

            let mut models_map = self.models.write().await;
            let mut clients_map = self.clients.write().await;

            for model in models {
                let model_id = model.id.clone();

                // 创建客户端
                match AIClient::new(model.clone()) {
                    Ok(client) => {
                        clients_map.insert(model_id.clone(), Arc::new(client));
                        models_map.insert(model_id.clone(), model);
                        info!("成功加载AI模型: {}", model_id);
                    }
                    Err(e) => {
                        warn!("加载AI模型失败 {}: {}", model_id, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// 获取所有模型配置
    pub async fn get_models(&self) -> Vec<AIModelConfig> {
        let models = self.models.read().await;
        models.values().cloned().collect()
    }

    /// 添加模型配置
    pub async fn add_model(&self, config: AIModelConfig) -> AppResult<()> {
        let model_id = config.id.clone();

        // 创建客户端
        let client = AIClient::new(config.clone()).context("创建AI客户端失败")?;

        // 保存到存储
        if let Some(storage) = &self.storage {
            let config_value = serde_json::to_value(&config).context("序列化模型配置失败")?;

            let save_options = crate::storage::types::SaveOptions {
                table: Some("ai_models".to_string()),
                overwrite: false,
                ..Default::default()
            };

            storage
                .save_data(&config_value, &save_options)
                .await
                .context("保存模型配置失败")?;
        }

        // 更新内存
        {
            let mut models = self.models.write().await;
            let mut clients = self.clients.write().await;

            models.insert(model_id.clone(), config);
            clients.insert(model_id.clone(), Arc::new(client));
        }

        info!("成功添加AI模型: {}", model_id);
        Ok(())
    }

    /// 更新模型配置（支持部分更新）
    pub async fn update_model(&self, model_id: &str, updates: serde_json::Value) -> AppResult<()> {
        // 获取现有配置
        let updated_config = {
            let models = self.models.read().await;
            let existing_config = models
                .get(model_id)
                .ok_or_else(|| anyhow!("模型不存在: {}", model_id))?;
            existing_config.clone()
        };

        // 应用部分更新
        let mut config_value =
            serde_json::to_value(&updated_config).context("序列化现有配置失败")?;

        // 合并更新
        if let serde_json::Value::Object(ref mut config_obj) = config_value {
            if let serde_json::Value::Object(updates_obj) = updates {
                for (key, value) in updates_obj {
                    config_obj.insert(key, value);
                }
            }
        }

        // 反序列化为完整配置
        let final_config: AIModelConfig =
            serde_json::from_value(config_value).context("反序列化更新后的配置失败")?;

        // 创建新客户端
        let client = AIClient::new(final_config.clone()).context("创建AI客户端失败")?;

        // 保存到存储
        if let Some(storage) = &self.storage {
            let config_value = serde_json::to_value(&final_config).context("序列化模型配置失败")?;

            let save_options = crate::storage::types::SaveOptions {
                table: Some("ai_models".to_string()),
                overwrite: true,
                ..Default::default()
            };

            storage
                .save_data(&config_value, &save_options)
                .await
                .context("更新模型配置失败")?;
        }

        // 更新内存
        {
            let mut models = self.models.write().await;
            let mut clients = self.clients.write().await;

            models.insert(model_id.to_string(), final_config);
            clients.insert(model_id.to_string(), Arc::new(client));
        }

        info!("成功更新AI模型: {}", model_id);
        Ok(())
    }

    /// 删除模型配置
    pub async fn remove_model(&self, model_id: &str) -> AppResult<()> {
        // 从内存删除
        {
            let mut models = self.models.write().await;
            let mut clients = self.clients.write().await;

            models.remove(model_id);
            clients.remove(model_id);
        }

        info!("成功删除AI模型: {}", model_id);
        Ok(())
    }

    /// 发送AI请求
    pub async fn send_request(
        &self,
        request: &AIRequest,
        model_id: Option<&str>,
    ) -> AppResult<AIResponse> {
        // 选择模型
        let selected_model_id = self.select_model(model_id).await?;

        // 获取客户端
        let client = {
            let clients = self.clients.read().await;
            clients
                .get(&selected_model_id)
                .ok_or_else(|| anyhow!("客户端不存在: {}", selected_model_id))?
                .clone()
        };

        // 发送请求（不使用缓存）
        let response = client.send_request(request).await?;

        Ok(response)
    }

    /// 发送流式AI请求
    pub async fn send_stream_request(
        &self,
        request: &AIRequest,
        model_id: Option<&str>,
    ) -> AppResult<AIStreamResponse> {
        // 选择模型
        let selected_model_id = self.select_model(model_id).await?;

        // 获取客户端
        let client = {
            let clients = self.clients.read().await;
            clients
                .get(&selected_model_id)
                .ok_or_else(|| anyhow!("客户端不存在: {}", selected_model_id))?
                .clone()
        };

        // 发送流式请求
        client.send_stream_request(request).await
    }

    /// 测试模型连接
    pub async fn test_connection(&self, model_id: &str) -> AppResult<bool> {
        let client = {
            let clients = self.clients.read().await;
            clients
                .get(model_id)
                .ok_or_else(|| anyhow!("客户端不存在: {}", model_id))?
                .clone()
        };

        let test_request = AIRequest {
            request_type: crate::ai::AIRequestType::Chat,
            content: "Hello".to_string(),
            context: None,
            options: Some(crate::ai::AIRequestOptions {
                max_tokens: Some(10),
                temperature: Some(0.1),
                stream: Some(false),
            }),
        };

        match client.send_request(&test_request).await {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("模型连接测试失败 {}: {}", model_id, e);
                Ok(false)
            }
        }
    }

    /// 清空缓存（已移除缓存功能）
    pub async fn clear_cache(&self) -> AppResult<()> {
        // 缓存功能已移除，直接返回成功
        Ok(())
    }

    /// 设置默认模型
    pub async fn set_default_model(&self, model_id: &str) -> AppResult<()> {
        let mut models = self.models.write().await;

        // 检查模型是否存在
        if !models.contains_key(model_id) {
            return Err(anyhow!("模型不存在: {}", model_id));
        }

        // 将所有模型的is_default设为false
        for model in models.values_mut() {
            model.is_default = Some(false);
        }

        // 设置指定模型为默认
        if let Some(model) = models.get_mut(model_id) {
            model.is_default = Some(true);
        }

        // 保存到存储
        if let Some(storage) = &self.storage {
            for model in models.values() {
                let config_value = serde_json::to_value(model).context("序列化模型配置失败")?;

                let save_options = crate::storage::types::SaveOptions {
                    table: Some("ai_models".to_string()),
                    overwrite: true,
                    ..Default::default()
                };

                storage
                    .save_data(&config_value, &save_options)
                    .await
                    .context("保存模型配置失败")?;
            }
        }

        info!("成功设置默认AI模型: {}", model_id);
        Ok(())
    }

    /// 选择模型
    async fn select_model(&self, preferred_model_id: Option<&str>) -> AppResult<String> {
        let models = self.models.read().await;

        // 如果指定了模型ID，使用指定的模型
        if let Some(model_id) = preferred_model_id {
            if models.contains_key(model_id) {
                return Ok(model_id.to_string());
            }
        }

        // 查找默认模型
        for model in models.values() {
            if model.is_default.unwrap_or(false) {
                return Ok(model.id.clone());
            }
        }

        // 如果没有默认模型，使用第一个可用的模型
        if let Some(model) = models.values().next() {
            return Ok(model.id.clone());
        }

        Err(anyhow!("没有可用的AI模型"))
    }
}
