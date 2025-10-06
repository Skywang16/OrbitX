//! 智能补全引擎

use crate::completion::providers::{
    CompletionProvider, ContextAwareProviderWrapper, FilesystemProvider, GitCompletionProvider,
    HistoryProvider, NpmCompletionProvider, SystemCommandsProvider,
};
use crate::completion::smart_provider::SmartCompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionResponse};
use crate::storage::cache::UnifiedCache;
use crate::utils::error::AppResult;
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, timeout};
use tracing::{info, warn};

#[derive(Debug, Clone)]
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
    config: CompletionEngineConfig,
    cache: Arc<UnifiedCache>,
}

impl CompletionEngine {
    pub fn new(config: CompletionEngineConfig, cache: Arc<UnifiedCache>) -> AppResult<Self> {
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
    ) -> AppResult<Self> {
        let mut engine = Self::new(config, Arc::clone(&cache))?;

        let filesystem_provider = Arc::new(FilesystemProvider::default());
        let system_commands_provider = Arc::new(SystemCommandsProvider::default());
        let history_provider = Arc::new(HistoryProvider::new(Arc::clone(&cache)));
        let git_provider = Arc::new(GitCompletionProvider::new(Arc::clone(&cache)));
        let npm_provider = Arc::new(NpmCompletionProvider::new(Arc::clone(&cache)));

        let context_aware_provider = {
            use crate::completion::output_analyzer::OutputAnalyzer;
            let analyzer = OutputAnalyzer::global();
            let provider_mutex = analyzer.get_context_provider();
            Arc::new(ContextAwareProviderWrapper::new(provider_mutex))
        };

        let smart_provider = Arc::new(SmartCompletionProvider::new(
            filesystem_provider.clone(),
            system_commands_provider.clone(),
            history_provider.clone(),
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
    ) -> AppResult<CompletionResponse> {
        let start = Instant::now();
        let fingerprint = Self::context_fingerprint(context);
        let result_cache_key = Self::result_cache_key(&fingerprint);

        if let Some(cached) = self
            .cache
            .get_deserialized::<CompletionResponse>(&result_cache_key)
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
            let provider_cache_key = Self::provider_cache_key(handle.name(), &fingerprint);
            if let Some(entry) = self
                .cache
                .get_deserialized::<ProviderCacheEntry>(&provider_cache_key)
                .await?
            {
                if !entry.items.is_empty() {
                    aggregated_items.extend(entry.items.clone());
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

        let config = self.config.clone();
        let cache = Arc::clone(&self.cache);
        let mut task_stream = stream::iter(pending.into_iter().map(|(handle, cache_key)| {
            let context = context.clone();
            let cache = Arc::clone(&cache);
            let config = config.clone();
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
                        "completion.provider_timeout: 补全提供者超时"
                    );
                    provider_logs.push(format!(
                        "{}(timeout, {}ms, {} attempts)",
                        name,
                        elapsed.as_millis(),
                        attempts
                    ));
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
            replace_start: context.word_start,
            replace_end: context.cursor_position,
            has_more,
        };

        if self.config.result_cache_ttl > Duration::from_millis(0) {
            if let Err(error) = self
                .cache
                .set_serialized_with_ttl(&result_cache_key, &response, self.config.result_cache_ttl)
                .await
            {
                warn!(error = %error, "completion.cache_store_failed");
            }
        }

        if !provider_logs.is_empty() {
            info!(
                input = %context.input,
                providers = %provider_logs.join(", "),
                total_items = response.items.len(),
                total_time_ms = start.elapsed().as_millis(),
                "completion.summary"
            );
        }

        Ok(response)
    }

    pub fn get_stats(&self) -> AppResult<EngineStats> {
        Ok(EngineStats {
            provider_count: self.providers.len(),
        })
    }

    pub async fn clear_cached_results(&self) -> AppResult<()> {
        let keys = self.cache.keys().await;
        for key in keys {
            if key.starts_with("completion/") {
                self.cache.remove(&key).await;
            }
        }
        Ok(())
    }

    fn finalize_items(&self, items: Vec<CompletionItem>) -> Vec<CompletionItem> {
        let mut merged: HashMap<String, CompletionItem> = HashMap::new();

        for item in items.into_iter() {
            if item.score < self.config.score_floor {
                continue;
            }

            match merged.get_mut(&item.text) {
                Some(existing) => {
                    if item.score > existing.score {
                        *existing = item;
                    }
                }
                None => {
                    merged.insert(item.text.clone(), item);
                }
            }
        }

        let mut deduped: Vec<CompletionItem> = merged.into_values().collect();
        deduped.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.text.cmp(&b.text))
        });
        deduped
    }

    async fn run_provider(
        handle: ProviderHandle,
        context: CompletionContext,
        cache: Arc<UnifiedCache>,
        cache_key: String,
        config: CompletionEngineConfig,
    ) -> ProviderOutcome {
        let start = Instant::now();
        let mut attempts = 0;
        let mut last_status = ProviderStatus::Timeout;

        while attempts <= config.max_retries {
            attempts += 1;
            let provider = Arc::clone(&handle.provider);
            let ctx = context.clone();

            match timeout(config.provider_timeout, async move {
                provider.provide_completions(&ctx).await
            })
            .await
            {
                Ok(Ok(items)) => {
                    if !items.is_empty() {
                        let entry = ProviderCacheEntry::new(items.clone());
                        if let Err(error) = cache
                            .set_serialized_with_ttl(&cache_key, &entry, config.provider_cache_ttl)
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
                    last_status = ProviderStatus::Error(error.to_string());
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

    fn context_fingerprint(context: &CompletionContext) -> String {
        let mut hasher = Sha256::new();
        hasher.update(context.input.as_bytes());
        hasher.update(context.cursor_position.to_le_bytes());
        hasher.update(context.working_directory.to_string_lossy().as_bytes());
        hasher.update(context.current_word.as_bytes());

        let mut env_pairs: Vec<_> = context.environment.iter().collect();
        env_pairs.sort_by(|a, b| a.0.cmp(b.0));
        for (key, value) in env_pairs.iter().take(16) {
            hasher.update(key.as_bytes());
            hasher.update(value.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    fn result_cache_key(fingerprint: &str) -> String {
        format!("completion/result/{}", fingerprint)
    }

    fn provider_cache_key(provider: &str, fingerprint: &str) -> String {
        format!("completion/provider/{}/{}", provider, fingerprint)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderCacheEntry {
    items: Vec<CompletionItem>,
    cached_at: u64,
}

impl ProviderCacheEntry {
    fn new(items: Vec<CompletionItem>) -> Self {
        Self {
            items,
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

#[derive(Debug, Clone)]
enum ProviderStatus {
    Success,
    Timeout,
    Error(String),
}

#[derive(Debug)]
pub struct EngineStats {
    pub provider_count: usize,
}
