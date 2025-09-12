//! 补全引擎核心模块
//!
//! 协调各种补全提供者，提供统一的补全接口

use crate::completion::providers::{
    CompletionProvider, ContextAwareProviderWrapper, FilesystemProvider, GitCompletionProvider,
    HistoryProvider, NpmCompletionProvider, SystemCommandsProvider,
};
use crate::completion::smart_provider::SmartCompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionResponse};
use crate::storage::cache::UnifiedCache;
use crate::utils::error::AppResult;
use anyhow::anyhow;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::info;

/// 补全引擎配置
#[derive(Debug, Clone)]
pub struct CompletionEngineConfig {
    /// 最大返回结果数
    pub max_results: usize,
    /// 单个提供者的超时时间
    pub provider_timeout: Duration,

    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔
    pub retry_interval: Duration,
}

impl Default for CompletionEngineConfig {
    fn default() -> Self {
        Self {
            max_results: 50,
            provider_timeout: Duration::from_millis(500),

            max_retries: 2,
            retry_interval: Duration::from_millis(100),
        }
    }
}

/// 补全引擎
pub struct CompletionEngine {
    /// 补全提供者列表
    providers: Vec<Arc<dyn CompletionProvider>>,
    /// 配置
    config: CompletionEngineConfig,
}

impl CompletionEngine {
    /// 创建新的补全引擎
    pub fn new(config: CompletionEngineConfig) -> AppResult<Self> {
        Ok(Self {
            providers: Vec::new(),
            config,
        })
    }

    /// 添加补全提供者
    pub fn add_provider(&mut self, provider: Arc<dyn CompletionProvider>) {
        self.providers.push(provider);

        // 按优先级排序
        self.providers
            .sort_by_key(|b| std::cmp::Reverse(b.priority()));
    }

    /// 创建默认的补全引擎（包含所有标准提供者）
    pub async fn with_default_providers(
        config: CompletionEngineConfig,
        cache: Arc<UnifiedCache>,
    ) -> AppResult<Self> {
        let mut engine = Self::new(config)?;

        // 创建基础提供者
        let filesystem_provider = Arc::new(FilesystemProvider::default());
        let system_commands_provider = Arc::new(SystemCommandsProvider::default());
        let history_provider = Arc::new(HistoryProvider::new(cache));

        // 创建增强提供者
        let git_provider = Arc::new(GitCompletionProvider::default());
        let npm_provider = Arc::new(NpmCompletionProvider::default());

        // 获取全局的上下文感知提供者
        let context_aware_provider = {
            use crate::completion::output_analyzer::OutputAnalyzer;
            let analyzer = OutputAnalyzer::global();
            let provider_mutex = analyzer.get_context_provider();
            // 这里我们需要创建一个包装器来适配Arc<dyn CompletionProvider>
            Arc::new(ContextAwareProviderWrapper::new(provider_mutex))
        };

        // 创建智能提供者（组合所有基础提供者）
        let smart_provider = Arc::new(SmartCompletionProvider::new(
            filesystem_provider.clone(),
            system_commands_provider.clone(),
            history_provider.clone(),
        ));

        // 添加增强提供者（最高优先级）
        engine.add_provider(context_aware_provider); // 上下文感知提供者优先级最高
        engine.add_provider(git_provider);
        engine.add_provider(npm_provider);

        // 添加智能提供者
        engine.add_provider(smart_provider);

        // 添加基础提供者作为后备
        engine.add_provider(system_commands_provider);
        engine.add_provider(history_provider);
        engine.add_provider(filesystem_provider);

        Ok(engine)
    }

    /// 获取补全建议
    pub async fn completion_get(
        &self,
        context: &CompletionContext,
    ) -> AppResult<CompletionResponse> {
        let start_time = std::time::Instant::now();

        let mut all_items = Vec::new();
        let mut provider_stats = Vec::new();

        // 并行执行所有适用的提供者
        for provider in &self.providers {
            let provider_start_time = std::time::Instant::now();

            if !provider.should_provide(context) {
                provider_stats.push((provider.name(), 0, 0, "skipped"));
                continue;
            }

            // 执行提供者（带重试和超时）
            let result = self.execute_provider_with_retry(provider, context).await;

            let provider_time = provider_start_time.elapsed().as_millis();

            match result {
                Ok(Ok(items)) => {
                    provider_stats.push((provider.name(), items.len(), provider_time, "success"));
                    all_items.extend(items);
                }
                Ok(Err(_)) => {
                    provider_stats.push((provider.name(), 0, provider_time, "error"));
                }
                Err(_) => {
                    provider_stats.push((provider.name(), 0, provider_time, "timeout"));
                }
            }
        }

        // 去重和排序
        all_items = self.deduplicate_and_sort(all_items);

        // 限制结果数量
        let has_more = all_items.len() > self.config.max_results;
        all_items.truncate(self.config.max_results);

        let response = CompletionResponse {
            items: all_items,
            replace_start: context.word_start,
            replace_end: context.cursor_position,
            has_more,
        };

        let total_time = start_time.elapsed().as_millis();

        // 生成最终汇总日志
        let provider_results: Vec<String> = provider_stats
            .iter()
            .filter(|(_, count, _, _)| *count > 0) // 只显示有结果的提供者
            .map(|(name, count, time, status)| {
                format!("{}({} 项, {}ms, {})", name, count, time, status)
            })
            .collect();

        if !provider_results.is_empty() {
            info!(
                "补全汇总: input='{}', providers=[{}], final_items={}, total_time={}ms",
                context.input,
                provider_results.join(", "),
                response.items.len(),
                total_time
            );
        }
        Ok(response)
    }

    /// 去重和排序补全项
    fn deduplicate_and_sort(&self, items: Vec<CompletionItem>) -> Vec<CompletionItem> {
        // 按文本去重，保留分数最高的
        let mut seen = std::collections::HashMap::new();
        let mut deduplicated = Vec::new();

        for item in items {
            match seen.get(&item.text) {
                Some(existing_score) => {
                    if item.score > *existing_score {
                        // 找到并替换现有项
                        if let Some(pos) = deduplicated
                            .iter()
                            .position(|i: &CompletionItem| i.text == item.text)
                        {
                            deduplicated[pos] = item.clone();
                            seen.insert(item.text, item.score);
                        }
                    }
                }
                None => {
                    seen.insert(item.text.clone(), item.score);
                    deduplicated.push(item);
                }
            }
        }

        // 按分数排序
        deduplicated.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.text.cmp(&b.text))
        });

        deduplicated
    }

    /// 执行提供者（带重试和超时）
    async fn execute_provider_with_retry(
        &self,
        provider: &Arc<dyn CompletionProvider>,
        context: &CompletionContext,
    ) -> Result<AppResult<Vec<CompletionItem>>, tokio::time::error::Elapsed> {
        let mut last_error = None;

        for retry_count in 0..=self.config.max_retries {
            let provider_clone = Arc::clone(provider);
            let context_clone = context.clone();

            let result = timeout(self.config.provider_timeout, async move {
                provider_clone.provide_completions(&context_clone).await
            })
            .await;

            match result {
                Ok(Ok(items)) => {
                    return Ok(Ok(items));
                }
                Ok(Err(e)) => {
                    last_error = Some(e);
                    if retry_count < self.config.max_retries {
                        sleep(self.config.retry_interval).await;
                    }
                }
                Err(timeout_error) => {
                    if retry_count < self.config.max_retries {
                        sleep(self.config.retry_interval).await;
                    } else {
                        return Err(timeout_error);
                    }
                }
            }
        }

        // 如果所有重试都失败了，返回最后一个错误
        Ok(Err(last_error.unwrap_or_else(|| {
            anyhow!("提供者错误: 所有重试都失败了")
        })))
    }

    /// 获取引擎统计信息
    pub fn get_stats(&self) -> AppResult<EngineStats> {
        Ok(EngineStats {
            provider_count: self.providers.len(),
        })
    }
}

/// 引擎统计信息
#[derive(Debug)]
pub struct EngineStats {
    /// 提供者数量
    pub provider_count: usize,
}

impl Default for CompletionEngine {
    fn default() -> Self {
        Self::new(CompletionEngineConfig::default()).expect("创建默认补全引擎失败")
    }
}

// 为了支持downcast，需要为CompletionProvider添加as_any方法
// 这需要修改trait定义，但为了保持简单，我们先用这个版本
