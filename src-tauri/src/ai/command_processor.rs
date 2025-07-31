/*!
 * AI命令处理器模块
 *
 * 提供统一的AI请求处理入口，包括：
 * - 请求路由和适配器选择
 * - 负载均衡和故障转移
 * - 缓存集成和管理
 * - 错误处理和重试机制
 */

use crate::ai::{
    AIAdapterManager, AIConfigManager, AIModelConfig, AIProvider, AIRequest, AIRequestType,
    AIResponse, AIStreamResponse, CacheManager, ContextManager, PromptEngine,
};
use crate::utils::error::AppResult;
use anyhow::anyhow;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{error, info, instrument, warn};

/// 请求处理选项
#[derive(Debug, Clone)]
pub struct ProcessingOptions {
    /// 是否使用缓存
    pub use_cache: bool,
    /// 请求超时时间
    pub timeout: Duration,
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔
    pub retry_interval: Duration,
    /// 是否启用负载均衡
    pub enable_load_balancing: bool,
    /// 指定使用的模型ID
    pub preferred_model_id: Option<String>,
}

impl Default for ProcessingOptions {
    fn default() -> Self {
        Self {
            use_cache: true,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_interval: Duration::from_millis(1000),
            enable_load_balancing: true,
            preferred_model_id: None,
        }
    }
}

/// 请求处理结果
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    /// AI响应
    pub response: AIResponse,
    /// 使用的模型ID
    pub model_id: String,
    /// 是否来自缓存
    pub from_cache: bool,
    /// 处理时间（毫秒）
    pub processing_time: u64,
    /// 重试次数
    pub retry_count: u32,
}

/// AI命令处理器
pub struct AICommandProcessor {
    /// 配置管理器
    config_manager: Arc<Mutex<AIConfigManager>>,
    /// 适配器管理器
    adapter_manager: Arc<Mutex<AIAdapterManager>>,
    /// 提示词引擎
    prompt_engine: Arc<Mutex<PromptEngine>>,
    /// 上下文管理器
    context_manager: Arc<Mutex<ContextManager>>,
    /// 缓存管理器
    cache_manager: Arc<Mutex<CacheManager>>,
}

impl AICommandProcessor {
    /// 创建新的命令处理器
    pub fn new(
        config_manager: Arc<Mutex<AIConfigManager>>,
        adapter_manager: Arc<Mutex<AIAdapterManager>>,
        prompt_engine: Arc<Mutex<PromptEngine>>,
        context_manager: Arc<Mutex<ContextManager>>,
        cache_manager: Arc<Mutex<CacheManager>>,
    ) -> Self {
        Self {
            config_manager,
            adapter_manager,
            prompt_engine,
            context_manager,
            cache_manager,
        }
    }

    /// 处理AI请求
    #[instrument(skip(self, request), fields(request_type = ?request.request_type))]
    pub async fn process_request(
        &self,
        request: AIRequest,
        options: ProcessingOptions,
    ) -> AppResult<ProcessingResult> {
        let start_time = Instant::now();

        info!(
            "处理AI请求: type={:?}, model={:?}",
            request.request_type, options.preferred_model_id
        );

        // 1. 检查缓存
        if options.use_cache {
            let cached_response = {
                let mut cache_manager = self.cache_manager.lock().await;
                let model_id = options.preferred_model_id.as_deref().unwrap_or("default");
                cache_manager.get(&request, model_id)
            };

            if let Some(response) = cached_response {
                let processing_time = start_time.elapsed().as_millis() as u64;
                info!("缓存命中: {}ms", processing_time);
                return Ok(ProcessingResult {
                    response,
                    model_id: "cached".to_string(),
                    from_cache: true,
                    processing_time,
                    retry_count: 0,
                });
            }
        }

        // 2. 增强请求
        let enhanced_request = self.enhance_request(request.clone()).await?;

        // 3. 选择适配器
        let model_id = self.select_adapter(&enhanced_request, &options).await?;

        // 4. 执行请求（带重试）
        let (response, retry_count) = self
            .execute_with_retry(&enhanced_request, &model_id, &options)
            .await?;

        // 5. 异步缓存响应
        if options.use_cache {
            let cache_manager = self.cache_manager.clone();
            let request_clone = request.clone();
            let response_clone = response.clone();
            let model_id_clone = model_id.clone();

            tokio::spawn(async move {
                let mut cache_manager = cache_manager.lock().await;
                let _ = cache_manager.put(&request_clone, &model_id_clone, response_clone);
            });
        }

        let processing_time = start_time.elapsed().as_millis() as u64;
        info!(
            "请求完成: model={}, {}ms, retry={}",
            model_id, processing_time, retry_count
        );

        Ok(ProcessingResult {
            response,
            model_id,
            from_cache: false,
            processing_time,
            retry_count,
        })
    }

    /// 处理流式AI请求
    pub async fn process_stream_request(
        &self,
        request: AIRequest,
        options: ProcessingOptions,
    ) -> AppResult<(AIStreamResponse, String)> {
        info!(
            "命令处理器: 开始处理流式请求, request_type={:?}",
            request.request_type
        );

        // 1. 增强请求（添加上下文和生成提示词）
        info!("命令处理器: 增强请求");
        let enhanced_request = self.enhance_request(request.clone()).await?;
        info!("命令处理器: 请求增强完成");

        // 2. 选择适配器
        info!(
            "命令处理器: 选择适配器, preferred_model={:?}",
            options.preferred_model_id
        );
        let model_id = self.select_adapter(&enhanced_request, &options).await?;
        info!("命令处理器: 选择的模型ID: {}", model_id);

        // 3. 获取适配器
        info!("命令处理器: 获取模型适配器: {}", model_id);
        let adapter = {
            let adapter_manager = self.adapter_manager.lock().await;
            let adapter = adapter_manager.get_adapter(&model_id).ok_or_else(|| {
                error!("命令处理器: 找不到模型适配器: {}", model_id);
                anyhow!("AI配置错误 ({}): 找不到模型适配器", model_id)
            })?;
            info!("命令处理器: 成功获取模型适配器: {}", model_id);
            adapter
        };

        // 4. 执行流式请求
        info!("命令处理器: 开始执行流式请求");
        let stream = adapter.send_stream_request(&enhanced_request).await?;
        info!("命令处理器: 流式请求执行成功");

        Ok((stream, model_id))
    }

    /// 增强请求（添加上下文和生成提示词）
    async fn enhance_request(&self, mut request: AIRequest) -> AppResult<AIRequest> {
        // 获取当前上下文
        {
            let context_manager = self.context_manager.lock().await;
            if request.context.is_none() {
                request.context = Some(context_manager.get_context().clone());
            }
        }

        // 获取用户前置提示词（暂时设为None，因为新配置结构中没有这个字段）
        let user_prefix_prompt: Option<String> = None;

        // 使用提示词引擎生成增强的提示词
        {
            let mut prompt_engine = self.prompt_engine.lock().await;
            let options = crate::ai::PromptOptions {
                user_prefix_prompt,
                ..Default::default()
            };

            let enhanced_content =
                prompt_engine.generate_prompt_with_options(&request, &options)?;
            request.content = enhanced_content;
        }

        Ok(request)
    }

    /// 选择适配器
    async fn select_adapter(
        &self,
        request: &AIRequest,
        options: &ProcessingOptions,
    ) -> AppResult<String> {
        let config_manager = self.config_manager.lock().await;

        info!(
            "适配器选择: 开始选择适配器, request_type={:?}",
            request.request_type
        );

        // 如果指定了首选模型，优先使用
        if let Some(preferred_id) = &options.preferred_model_id {
            info!("适配器选择: 检查首选模型: {}", preferred_id);
            match config_manager.get_model(preferred_id).await {
                Ok(Some(model)) => {
                    info!(
                        "适配器选择: 找到首选模型: {}, provider={:?}",
                        model.name, model.provider
                    );
                    if self.is_model_suitable(&model, request) {
                        info!("适配器选择: 使用首选模型: {}", preferred_id);
                        return Ok(preferred_id.clone());
                    } else {
                        warn!(
                            "适配器选择: 首选模型不适合: suitable={}",
                            self.is_model_suitable(&model, request)
                        );
                    }
                }
                Ok(None) => {
                    warn!("适配器选择: 首选模型不存在: {}", preferred_id);
                }
                Err(e) => {
                    warn!("适配器选择: 获取首选模型失败: {}", e);
                }
            }
        } else {
            info!("适配器选择: 没有指定首选模型");
        }

        // 获取所有可用的模型
        let all_models = config_manager.get_models().await?;
        info!("适配器选择: 总模型数量: {}", all_models.len());

        let available_models: Vec<_> = all_models
            .iter()
            .filter(|model| {
                let suitable = self.is_model_suitable(model, request);
                info!("适配器选择: 检查模型 {} - suitable={}", model.id, suitable);
                suitable
            })
            .collect();

        info!("适配器选择: 可用模型数量: {}", available_models.len());

        if available_models.is_empty() {
            error!("适配器选择: 没有可用的AI模型");
            return Err(anyhow!("AI配置错误: 没有可用的AI模型"));
        }

        // 负载均衡选择
        if options.enable_load_balancing && available_models.len() > 1 {
            info!("适配器选择: 使用负载均衡选择");
            self.select_with_load_balancing(&available_models).await
        } else {
            // 选择默认模型或第一个可用模型
            let selected = available_models
                .iter()
                .find(|model| model.is_default.unwrap_or(false))
                .unwrap_or(&available_models[0]);
            info!(
                "适配器选择: 选择模型: id={}, name={}",
                selected.id, selected.name
            );
            Ok(selected.id.clone())
        }
    }

    /// 检查模型是否适合处理特定请求
    fn is_model_suitable(&self, model: &AIModelConfig, request: &AIRequest) -> bool {
        // 根据请求类型和模型能力判断
        match request.request_type {
            AIRequestType::Chat => {
                // 大部分模型支持聊天
                matches!(
                    model.provider,
                    AIProvider::OpenAI | AIProvider::Claude | AIProvider::Custom
                )
            }
            AIRequestType::Explanation | AIRequestType::ErrorAnalysis => {
                // 需要较强的推理能力
                matches!(model.provider, AIProvider::OpenAI | AIProvider::Claude)
            }
        }
    }

    /// 负载均衡选择模型（优化：基于健康状态和性能）
    async fn select_with_load_balancing(&self, models: &[&AIModelConfig]) -> AppResult<String> {
        // 简化实现：检查适配器是否存在，选择第一个可用的
        let adapter_manager = self.adapter_manager.lock().await;

        for model in models {
            if adapter_manager.has_adapter(&model.id) {
                return Ok(model.id.clone());
            }
        }

        // 如果没有找到可用的适配器，返回第一个模型ID
        Ok(models[0].id.clone())
    }

    /// 执行请求（带重试机制）
    async fn execute_with_retry(
        &self,
        request: &AIRequest,
        model_id: &str,
        options: &ProcessingOptions,
    ) -> AppResult<(AIResponse, u32)> {
        let mut last_error = None;

        for retry_count in 0..=options.max_retries {
            match self
                .execute_single_request(request, model_id, options)
                .await
            {
                Ok(response) => return Ok((response, retry_count)),
                Err(error) => {
                    let error_msg = error.to_string();
                    last_error = Some(anyhow!("{}", error_msg));

                    // 如果是配置错误或认证错误，不重试
                    if error_msg.contains("配置错误") || error_msg.contains("认证失败") {
                        break;
                    }

                    // 如果不是最后一次重试，等待后重试
                    if retry_count < options.max_retries {
                        tokio::time::sleep(options.retry_interval).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("AI未知错误: 请求执行失败")))
    }

    /// 执行单个请求
    async fn execute_single_request(
        &self,
        request: &AIRequest,
        model_id: &str,
        options: &ProcessingOptions,
    ) -> AppResult<AIResponse> {
        // 获取适配器的Arc引用，避免跨await边界持有锁
        let adapter = {
            let adapter_manager = self.adapter_manager.lock().await;

            adapter_manager
                .get_adapter(model_id)
                .ok_or_else(|| anyhow!("AI配置错误 ({}): 找不到模型适配器", model_id))?
        };

        // 带超时的请求执行
        let response_future = adapter.send_request(request);

        match timeout(options.timeout, response_future).await {
            Ok(result) => result,
            Err(_) => Err(anyhow!(
                "AI请求超时 ({}): {}秒",
                model_id,
                options.timeout.as_secs()
            )),
        }
    }

    /// 获取处理器状态
    pub async fn get_status(&self) -> AppResult<ProcessorStatus> {
        let config_manager = self.config_manager.lock().await;
        let cache_manager = self.cache_manager.lock().await;

        let models = config_manager.get_models().await?;
        let available_models = models.len();

        let cache_stats = cache_manager.get_stats()?;

        Ok(ProcessorStatus {
            available_models,
            cache_hit_rate: cache_stats.hit_rate,
            total_cache_entries: cache_stats.total_entries,
        })
    }
}

/// 处理器状态信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorStatus {
    /// 可用模型数量
    pub available_models: usize,
    /// 缓存命中率
    pub cache_hit_rate: f64,
    /// 缓存条目总数
    pub total_cache_entries: usize,
}
