//! 智能补全引擎

use crate::completion::error::{CompletionEngineResult, CompletionProviderError};
use crate::completion::providers::{
    CompletionProvider, FilesystemProvider, GitCompletionProvider, HistoryProvider,
    NpmCompletionProvider, SystemCommandsProvider,
};
use crate::completion::scoring::MIN_SCORE;
use crate::completion::smart_provider::SmartCompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionResponse};
use crate::storage::DatabaseManager;
use crate::storage::{CacheNamespace, UnifiedCache};
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, timeout};
use tracing::warn;

#[derive(Debug, Clone, Copy)]
pub struct CompletionEngineConfig {
    pub max_results: usize,
    pub provider_timeout: Duration,
    pub max_retries: u32,
    pub retry_interval: Duration,
    pub max_concurrency: usize,
    pub provider_cache_ttl: Duration,
    pub result_cache_ttl: Duration,
    pub score_floor: f64,
}

impl Default for CompletionEngineConfig {
    fn default() -> Self {
        Self {
            max_results: 50,
            provider_timeout: Duration::from_millis(300),
            max_retries: 1,
            retry_interval: Duration::from_millis(75),
            max_concurrency: 4,
            provider_cache_ttl: Duration::from_secs(30),
            result_cache_ttl: Duration::from_millis(800),
            score_floor: f64::MIN,
        }
    }
}

#[derive(Clone)]
struct ProviderHandle {
    provider: Arc<dyn CompletionProvider>,
}

impl ProviderHandle {
    fn name(&self) -> &'static str {
        self.provider.name()
    }

    fn priority(&self) -> i32 {
        self.provider.priority()
    }

    fn should_provide(&self, context: &CompletionContext) -> bool {
        self.provider.should_provide(context)
    }
}

pub struct CompletionEngine {
    providers: Vec<ProviderHandle>,
    config: CompletionEngineConfig, // 直接内嵌，零成本
    cache: Arc<UnifiedCache>,
}

impl CompletionEngine {
    pub fn new(
        config: CompletionEngineConfig,
        cache: Arc<UnifiedCache>,
    ) -> CompletionEngineResult<Self> {
        Ok(Self {
            providers: Vec::new(),
            config,
            cache,
        })
    }

    pub fn add_provider(&mut self, provider: Arc<dyn CompletionProvider>) {
        self.providers.push(ProviderHandle { provider });
        self.providers
            .sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    pub async fn with_default_providers(
        config: CompletionEngineConfig,
        cache: Arc<UnifiedCache>,
        database: Arc<DatabaseManager>,
    ) -> CompletionEngineResult<Self> {
        let mut engine = Self::new(config, Arc::clone(&cache))?;

        let filesystem_provider = Arc::new(FilesystemProvider::default());
        let system_commands_provider = Arc::new(SystemCommandsProvider::default());
        let history_provider = Arc::new(HistoryProvider::new(Arc::clone(&cache)));
        let git_provider = Arc::new(GitCompletionProvider::new(Arc::clone(&cache)));
        let npm_provider = Arc::new(NpmCompletionProvider::new(Arc::clone(&cache)));

        let context_aware_provider = {
            use crate::completion::output_analyzer::OutputAnalyzer;
            let analyzer = OutputAnalyzer::global();
            let provider = analyzer.context_provider();
            provider as Arc<dyn CompletionProvider>
        };

        let smart_provider = Arc::new(SmartCompletionProvider::new(
            filesystem_provider.clone(),
            system_commands_provider.clone(),
            history_provider.clone(),
            database,
        ));

        engine.add_provider(context_aware_provider);
        engine.add_provider(git_provider);
        engine.add_provider(npm_provider);
        engine.add_provider(smart_provider);
        engine.add_provider(system_commands_provider);
        engine.add_provider(history_provider);
        engine.add_provider(filesystem_provider);

        Ok(engine)
    }

    pub async fn completion_get(
        &self,
        context: &CompletionContext,
    ) -> CompletionEngineResult<CompletionResponse> {
        let _start = Instant::now();
        let fingerprint = Self::context_fingerprint(context);
        let result_cache_key = Self::result_cache_key(fingerprint);

        if let Some(cached) = self
            .cache
            .get_deserialized_ns::<CompletionResponse>(
                CacheNamespace::Completion,
                &result_cache_key,
            )
            .await?
        {
            return Ok(cached);
        }

        let mut aggregated_items = Vec::new();
        let mut provider_logs = Vec::new();
        let mut pending = Vec::new();

        for handle in self
            .providers
            .iter()
            .cloned()
            .filter(|handle| handle.should_provide(context))
        {
            let provider_cache_key = Self::provider_cache_key(handle.name(), fingerprint);
            if let Some(entry) = self
                .cache
                .get_deserialized_ns::<ProviderCacheEntry>(
                    CacheNamespace::Completion,
                    &provider_cache_key,
                )
                .await?
            {
                if !entry.items.is_empty() {
                    aggregated_items.extend_from_slice(&entry.items);
                }
                provider_logs.push(format!(
                    "{}(cache, {} items)",
                    handle.name(),
                    entry.items.len()
                ));
            } else {
                pending.push((handle, provider_cache_key));
            }
        }

        let config = &self.config;
        let cache = Arc::clone(&self.cache);
        let context_arc = Arc::new(context.clone());

        let mut task_stream = stream::iter(pending.into_iter().map(|(handle, cache_key)| {
            let context = Arc::clone(&context_arc);
            let cache = Arc::clone(&cache);
            let config = *config; // Copy, 零成本
            async move { Self::run_provider(handle, context, cache, cache_key, config).await }
        }))
        .buffer_unordered(self.config.max_concurrency);

        while let Some(outcome) = task_stream.next().await {
            let ProviderOutcome {
                name,
                items,
                status,
                elapsed,
                attempts,
            } = outcome;

            let item_count = items.len();
            match &status {
                ProviderStatus::Success => {
                    if !items.is_empty() {
                        aggregated_items.extend(items);
                    }
                    provider_logs.push(format!(
                        "{}(live, {} items, {}ms, {} attempts)",
                        name,
                        item_count,
                        elapsed.as_millis(),
                        attempts
                    ));
                }
                ProviderStatus::Timeout => {
                    warn!(
                        provider = name,
                        elapsed_ms = elapsed.as_millis(),
                        attempts = attempts,
                        "completion.provider_timeout: provider timed out"
                    );
                }
                ProviderStatus::Error(error) => {
                    warn!(
                        provider = name,
                        elapsed_ms = elapsed.as_millis(),
                        attempts = attempts,
                        error = %error,
                        "completion.provider_error"
                    );
                    provider_logs.push(format!(
                        "{}(error: {}, {}ms, {} attempts)",
                        name,
                        error,
                        elapsed.as_millis(),
                        attempts
                    ));
                }
            }
        }

        let mut items = self.finalize_items(aggregated_items);
        let has_more = items.len() > self.config.max_results;
        if has_more {
            items.truncate(self.config.max_results);
        }

        let response = CompletionResponse {
            items,
            replace_start: {
                // 特例：`cd` 不带空格时也允许补全目录（会插入前导空格）。
                // 这不影响 shell 的 Tab 补全，只影响 OrbitX 内联/列表建议。
                if context.input.trim() == "cd"
                    && context.cursor_position == context.input.len()
                    && !context.input.chars().any(|c| c.is_whitespace())
                {
                    context.cursor_position
                } else {
                    context.word_start
                }
            },
            replace_end: context.cursor_position,
            has_more,
        };

        if self.config.result_cache_ttl > Duration::from_millis(0) {
            if let Err(error) = self
                .cache
                .set_serialized_ns_with_ttl(
                    CacheNamespace::Completion,
                    &result_cache_key,
                    &response,
                    self.config.result_cache_ttl,
                )
                .await
            {
                warn!(error = %error, "completion.cache_store_failed");
            }
        }

        Ok(response)
    }

    pub fn get_stats(&self) -> CompletionEngineResult<EngineStats> {
        Ok(EngineStats {
            provider_count: self.providers.len(),
        })
    }

    pub async fn clear_cached_results(&self) -> CompletionEngineResult<()> {
        self.cache.clear_namespace(CacheNamespace::Completion).await;
        Ok(())
    }

    /// 完成补全项处理：过滤、去重、排序
    ///
    /// 使用原地操作减少内存分配
    fn finalize_items(&self, mut items: Vec<CompletionItem>) -> Vec<CompletionItem> {
        // 1. 过滤低分项（原地操作）
        items.retain(|item| item.score >= MIN_SCORE);

        // 2. 排序（使用 CompletionItem 的 Ord 实现）
        items.sort_unstable();

        // 3. 去重：保留每个文本的第一个（因已按分数排序，第一个即最高分）
        items.dedup_by(|a, b| a.text == b.text);

        items
    }

    async fn run_provider(
        handle: ProviderHandle,
        context: Arc<CompletionContext>,
        cache: Arc<UnifiedCache>,
        cache_key: String,
        config: CompletionEngineConfig, // 直接传递，零成本Copy
    ) -> ProviderOutcome {
        let start = Instant::now();
        let mut attempts = 0;
        let mut last_status = ProviderStatus::Timeout;

        while attempts <= config.max_retries {
            attempts += 1;
            let provider = Arc::clone(&handle.provider);
            let ctx = Arc::clone(&context);

            match timeout(config.provider_timeout, async move {
                provider.provide_completions(&*ctx).await
            })
            .await
            {
                Ok(Ok(items)) => {
                    if !items.is_empty() {
                        let entry = ProviderCacheEntry::new(items.clone());
                        if let Err(error) = cache
                            .set_serialized_ns_with_ttl(
                                CacheNamespace::Completion,
                                &cache_key,
                                &entry,
                                config.provider_cache_ttl,
                            )
                            .await
                        {
                            warn!(
                                provider = handle.name(),
                                error = %error,
                                "completion.provider_cache_failed"
                            );
                        }
                    }

                    return ProviderOutcome {
                        name: handle.name(),
                        items,
                        status: ProviderStatus::Success,
                        elapsed: start.elapsed(),
                        attempts,
                    };
                }
                Ok(Err(error)) => {
                    last_status = ProviderStatus::Error(error);
                }
                Err(_) => {
                    last_status = ProviderStatus::Timeout;
                }
            }

            if attempts > config.max_retries {
                break;
            }

            sleep(config.retry_interval).await;
        }

        ProviderOutcome {
            name: handle.name(),
            items: Vec::new(),
            status: last_status,
            elapsed: start.elapsed(),
            attempts,
        }
    }

    fn context_fingerprint(context: &CompletionContext) -> u64 {
        let mut hasher = DefaultHasher::new();
        context.input.hash(&mut hasher);
        context.cursor_position.hash(&mut hasher);
        context.working_directory.hash(&mut hasher);
        context.current_word.hash(&mut hasher);
        hasher.finish()
    }

    fn result_cache_key(fingerprint: u64) -> String {
        format!("result:{}", fingerprint)
    }

    fn provider_cache_key(provider: &str, fingerprint: u64) -> String {
        format!("provider:{}:{}", provider, fingerprint)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderCacheEntry {
    items: Arc<[CompletionItem]>,
    cached_at: u64,
}

impl ProviderCacheEntry {
    fn new(items: Vec<CompletionItem>) -> Self {
        Self {
            items: items.into(),
            cached_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

#[derive(Debug)]
struct ProviderOutcome {
    name: &'static str,
    items: Vec<CompletionItem>,
    status: ProviderStatus,
    elapsed: Duration,
    attempts: u32,
}

#[derive(Debug)]
enum ProviderStatus {
    Success,
    Timeout,
    Error(CompletionProviderError),
}

#[derive(Debug)]
pub struct EngineStats {
    pub provider_count: usize,
}
