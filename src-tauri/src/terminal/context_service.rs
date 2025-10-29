use crate::mux::{PaneId, TerminalMux};
use crate::shell::{ContextServiceIntegration, ShellIntegrationManager};
use crate::storage::cache::{CacheEntrySnapshot, UnifiedCache};
use crate::terminal::{
    context_registry::ActiveTerminalContextRegistry,
    error::{ContextServiceError, ContextServiceResult},
    CommandInfo, ShellType, TerminalContext, TerminalContextEvent,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;
use tracing::{debug, warn};

const CONTEXT_CACHE_PREFIX: &str = "terminal/context";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TerminalCachePayload {
    context: TerminalContext,
    cached_at: u64,
}

impl TerminalCachePayload {
    fn new(context: TerminalContext) -> Self {
        Self {
            context,
            cached_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub total_entries: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub eviction_count: u64,
    pub hit_rate: f64,
}

impl CacheStats {
    fn from_counters(
        total_entries: usize,
        hit_count: u64,
        miss_count: u64,
        eviction_count: u64,
    ) -> Self {
        let total_requests = hit_count + miss_count;
        let hit_rate = if total_requests > 0 {
            hit_count as f64 / total_requests as f64
        } else {
            0.0
        };

        Self {
            total_entries,
            hit_count,
            miss_count,
            eviction_count,
            hit_rate,
        }
    }
}

pub struct TerminalContextService {
    registry: Arc<ActiveTerminalContextRegistry>,
    shell_integration: Arc<ShellIntegrationManager>,
    terminal_mux: Arc<TerminalMux>,
    cache: Arc<UnifiedCache>,
    cache_hits: Arc<AtomicU64>,
    cache_misses: Arc<AtomicU64>,
    cache_evictions: Arc<AtomicU64>,
    query_timeout: Duration,
    min_cache_ttl: Duration,
    base_cache_ttl: Duration,
    max_cache_ttl: Duration,
}

impl TerminalContextService {
    pub fn new(
        registry: Arc<ActiveTerminalContextRegistry>,
        shell_integration: Arc<ShellIntegrationManager>,
        terminal_mux: Arc<TerminalMux>,
        cache: Arc<UnifiedCache>,
    ) -> Self {
        Self {
            registry,
            shell_integration,
            terminal_mux,
            cache,
            cache_hits: Arc::new(AtomicU64::new(0)),
            cache_misses: Arc::new(AtomicU64::new(0)),
            cache_evictions: Arc::new(AtomicU64::new(0)),
            query_timeout: Duration::from_millis(1500),
            min_cache_ttl: Duration::from_secs(3),
            base_cache_ttl: Duration::from_secs(12),
            max_cache_ttl: Duration::from_secs(90),
        }
    }

    pub fn new_with_integration(
        registry: Arc<ActiveTerminalContextRegistry>,
        shell_integration: Arc<ShellIntegrationManager>,
        terminal_mux: Arc<TerminalMux>,
        cache: Arc<UnifiedCache>,
    ) -> Arc<Self> {
        let service = Arc::new(Self::new(registry, shell_integration, terminal_mux, cache));

        service.shell_integration.set_context_service_integration(
            Arc::downgrade(&service) as std::sync::Weak<dyn ContextServiceIntegration>
        );

        service
    }

    pub async fn get_active_context(&self) -> ContextServiceResult<TerminalContext> {
        let active_pane_id = self
            .registry
            .terminal_context_get_active_pane()
            .ok_or(ContextServiceError::NoActivePane)?;

        debug!("获取活跃终端上下文: pane_id={:?}", active_pane_id);
        self.get_context_by_pane(active_pane_id).await
    }

    pub async fn get_context_by_pane(
        &self,
        pane_id: PaneId,
    ) -> ContextServiceResult<TerminalContext> {
        if let Some(cached) = self.load_from_cache(pane_id).await? {
            return Ok(cached);
        }

        let context = timeout(self.query_timeout, self.query_context_internal(pane_id))
            .await
            .map_err(|_| ContextServiceError::QueryTimeout)??;

        self.store_in_cache(pane_id, &context).await;
        self.send_context_updated_event(pane_id, &context);

        Ok(context)
    }

    pub async fn get_context_with_fallback(
        &self,
        pane_id: Option<PaneId>,
    ) -> ContextServiceResult<TerminalContext> {
        if let Some(pane_id) = pane_id {
            if let Ok(context) = self.get_context_by_pane(pane_id).await {
                return Ok(context);
            }
        }

        if let Some(active_pane) = self.registry.terminal_context_get_active_pane() {
            if let Ok(context) = self.get_context_by_pane(active_pane).await {
                return Ok(context);
            }
        }

        if let Some(context) = self.load_any_cached_context().await? {
            return Ok(context);
        }

        Ok(self.create_default_context())
    }

    pub async fn get_active_cwd(&self) -> ContextServiceResult<String> {
        let context = self.get_active_context().await?;
        context
            .current_working_directory
            .ok_or(ContextServiceError::WorkingDirectoryMissing)
    }

    pub async fn shell_get_pane_cwd(&self, pane_id: PaneId) -> ContextServiceResult<String> {
        let context = self.get_context_by_pane(pane_id).await?;
        context
            .current_working_directory
            .ok_or(ContextServiceError::WorkingDirectoryMissing)
    }

    pub async fn get_active_shell_type(&self) -> ContextServiceResult<ShellType> {
        let context = self.get_active_context().await?;
        context
            .shell_type
            .ok_or(ContextServiceError::ShellTypeMissing)
    }

    pub async fn get_pane_shell_type(&self, pane_id: PaneId) -> ContextServiceResult<ShellType> {
        let context = self.get_context_by_pane(pane_id).await?;
        context
            .shell_type
            .ok_or(ContextServiceError::ShellTypeMissing)
    }

    pub async fn invalidate_cache_entry(&self, pane_id: PaneId) {
        let cache_key = Self::cache_key(pane_id);
        if self.cache.remove(&cache_key).await.is_some() {
            self.cache_evictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub async fn clear_all_cache(&self) {
        let keys = self.cache.keys().await;
        let mut removed = 0u64;

        for key in keys {
            if key.starts_with(CONTEXT_CACHE_PREFIX) {
                if self.cache.remove(&key).await.is_some() {
                    removed += 1;
                }
            }
        }

        if removed > 0 {
            self.cache_evictions.fetch_add(removed, Ordering::Relaxed);
        }
    }

    pub async fn get_cache_stats(&self) -> CacheStats {
        let total_entries = self
            .cache
            .keys()
            .await
            .into_iter()
            .filter(|key| key.starts_with(CONTEXT_CACHE_PREFIX))
            .count();

        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let evictions = self.cache_evictions.load(Ordering::Relaxed);

        CacheStats::from_counters(total_entries, hits, misses, evictions)
    }

    async fn load_from_cache(
        &self,
        pane_id: PaneId,
    ) -> ContextServiceResult<Option<TerminalContext>> {
        let cache_key = Self::cache_key(pane_id);
        match self.cache.snapshot(&cache_key).await {
            Some(snapshot) => {
                match serde_json::from_value::<TerminalCachePayload>(snapshot.value.clone()) {
                    Ok(payload) => {
                        self.cache_hits.fetch_add(1, Ordering::Relaxed);
                        self.adjust_cache_ttl(&cache_key, &payload, &snapshot).await;
                        Ok(Some(payload.context))
                    }
                    Err(error) => {
                        warn!(
                            pane = pane_id.as_u32(),
                            error = %error,
                            "terminal.cache.deserialize_failed"
                        );
                        self.cache.remove(&cache_key).await;
                        self.cache_evictions.fetch_add(1, Ordering::Relaxed);
                        self.cache_misses.fetch_add(1, Ordering::Relaxed);
                        Ok(None)
                    }
                }
            }
            None => {
                self.cache_misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        }
    }

    async fn load_any_cached_context(&self) -> ContextServiceResult<Option<TerminalContext>> {
        let mut best: Option<(std::time::Instant, TerminalContext)> = None;
        let keys = self.cache.keys().await;

        for key in keys {
            if !key.starts_with(CONTEXT_CACHE_PREFIX) {
                continue;
            }

            if let Some(snapshot) = self.cache.snapshot(&key).await {
                match serde_json::from_value::<TerminalCachePayload>(snapshot.value.clone()) {
                    Ok(payload) => match &mut best {
                        Some((current, _)) if *current >= snapshot.last_accessed => {}
                        _ => best = Some((snapshot.last_accessed, payload.context)),
                    },
                    Err(error) => {
                        warn!(error = %error, "terminal.cache.deserialize_failed");
                        self.cache.remove(&key).await;
                        self.cache_evictions.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }

        Ok(best.map(|(_, ctx)| ctx))
    }

    async fn store_in_cache(&self, pane_id: PaneId, context: &TerminalContext) {
        let cache_key = Self::cache_key(pane_id);
        let payload = TerminalCachePayload::new(context.clone());
        let ttl = self.compute_adaptive_ttl(context, 0, None);

        if let Err(error) = self
            .cache
            .set_serialized_with_ttl(&cache_key, &payload, ttl)
            .await
        {
            warn!(error = %error, "terminal.cache.store_failed");
        }
    }

    async fn adjust_cache_ttl(
        &self,
        cache_key: &str,
        payload: &TerminalCachePayload,
        snapshot: &CacheEntrySnapshot,
    ) {
        let desired =
            self.compute_adaptive_ttl(&payload.context, snapshot.hit_count, snapshot.remaining_ttl);
        if let Some(remaining) = snapshot.remaining_ttl {
            if remaining + Duration::from_millis(250) >= desired {
                return;
            }
        }
        self.cache.update_ttl(cache_key, Some(desired)).await;
    }

    fn compute_adaptive_ttl(
        &self,
        context: &TerminalContext,
        hit_count: u64,
        remaining: Option<Duration>,
    ) -> Duration {
        let mut ttl = if context.is_active {
            self.max_cache_ttl
        } else {
            self.base_cache_ttl
        };

        if let Ok(idle) = SystemTime::now().duration_since(context.last_activity) {
            if idle > Duration::from_secs(120) {
                ttl = self.min_cache_ttl;
            } else if idle < Duration::from_secs(10) {
                ttl = ttl.saturating_mul(2);
            }
        }

        if hit_count > 12 {
            ttl = ttl.saturating_mul(2);
        } else if hit_count > 4 {
            ttl = ttl.saturating_mul(3) / 2;
        }

        if let Some(remaining) = remaining {
            if remaining > ttl {
                ttl = remaining;
            }
        }

        if ttl < self.min_cache_ttl {
            ttl = self.min_cache_ttl;
        }
        if ttl > self.max_cache_ttl {
            ttl = self.max_cache_ttl;
        }

        ttl
    }

    async fn query_context_internal(
        &self,
        pane_id: PaneId,
    ) -> ContextServiceResult<TerminalContext> {
        if !self.terminal_mux.pane_exists(pane_id) {
            return Err(ContextServiceError::PaneNotFound {
                pane_id: pane_id.as_u32(),
            });
        }

        let mut context = TerminalContext::new(pane_id);
        context.set_active(self.registry.terminal_context_is_pane_active(pane_id));

        if let Some(cwd) = self.terminal_mux.shell_get_pane_cwd(pane_id) {
            context.update_cwd(cwd);
        }

        if let Some(shell_state) = self.terminal_mux.get_pane_shell_state(pane_id) {
            if let Some(shell_type) = shell_state.shell_type {
                context.update_shell_type(self.convert_shell_type(shell_type));
            }

            context.set_shell_integration(
                shell_state.integration_state == crate::shell::ShellIntegrationState::Enabled,
            );

            if let Some(current_cmd) = shell_state.current_command {
                context.set_current_command(Some(self.convert_command_info(&current_cmd)));
            }

            let history: Vec<CommandInfo> = shell_state
                .command_history
                .iter()
                .map(|cmd| self.convert_command_info(cmd))
                .collect();
            context.command_history = history;

            if let Some(title) = shell_state.window_title {
                context.update_window_title(title);
            }
        }

        debug!("成功查询终端上下文: pane_id={:?}", pane_id);
        Ok(context)
    }

    fn create_default_context(&self) -> TerminalContext {
        let mut context = TerminalContext::new(PaneId::new(0));
        context.update_cwd("~".to_string());
        context.update_shell_type(ShellType::Bash);
        context.set_shell_integration(false);
        context
    }

    fn convert_shell_type(&self, shell_type: crate::shell::ShellType) -> ShellType {
        match shell_type {
            crate::shell::ShellType::Bash => ShellType::Bash,
            crate::shell::ShellType::Zsh => ShellType::Zsh,
            crate::shell::ShellType::Fish => ShellType::Fish,
            crate::shell::ShellType::Other(name) => ShellType::Other(name),
        }
    }

    fn convert_command_info(&self, cmd: &crate::shell::CommandInfo) -> CommandInfo {
        let (command, args) = if let Some(command_line) = &cmd.command_line {
            let parts: Vec<&str> = command_line.split_whitespace().collect();
            if parts.is_empty() {
                ("".to_string(), Vec::new())
            } else {
                let command = parts[0].to_string();
                let args = parts[1..].iter().map(|s| s.to_string()).collect();
                (command, args)
            }
        } else {
            ("".to_string(), Vec::new())
        };

        CommandInfo {
            command,
            args,
            start_time: cmd.start_time_wallclock,
            end_time: cmd.end_time_wallclock,
            exit_code: cmd.exit_code,
            working_directory: cmd.working_directory.clone(),
        }
    }

    fn send_context_updated_event(&self, pane_id: PaneId, context: &TerminalContext) {
        let event = TerminalContextEvent::PaneContextUpdated {
            pane_id,
            context: context.clone(),
        };

        if let Err(error) = self.registry.send_event(event) {
            warn!("Failed to send context update event: {}", error);
        }
    }

    fn cache_key(pane_id: PaneId) -> String {
        format!("{}/{}", CONTEXT_CACHE_PREFIX, pane_id)
    }
}

impl ContextServiceIntegration for TerminalContextService {
    fn invalidate_cache(&self, pane_id: PaneId) {
        let cache = Arc::clone(&self.cache);
        let evictions = Arc::clone(&self.cache_evictions);
        let cache_key = Self::cache_key(pane_id);

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if cache.remove(&cache_key).await.is_some() {
                    evictions.fetch_add(1, Ordering::Relaxed);
                }
            });
        } else if let Ok(rt) = tokio::runtime::Runtime::new() {
            rt.block_on(async {
                if cache.remove(&cache_key).await.is_some() {
                    evictions.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
    }

    fn send_cwd_changed_event(&self, pane_id: PaneId, old_cwd: Option<String>, new_cwd: String) {
        let event = TerminalContextEvent::PaneCwdChanged {
            pane_id,
            old_cwd,
            new_cwd,
        };

        if let Err(e) = self.registry.send_event(event) {
            warn!("发送CWD变化事件失败: {}", e);
        }
    }

    fn send_shell_integration_changed_event(&self, pane_id: PaneId, enabled: bool) {
        let event = TerminalContextEvent::PaneShellIntegrationChanged { pane_id, enabled };

        if let Err(e) = self.registry.send_event(event) {
            warn!("发送Shell集成状态变化事件失败: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mux::TerminalMux;
    use crate::shell::ShellIntegrationManager;

    fn create_service() -> TerminalContextService {
        TerminalContextService::new(
            Arc::new(ActiveTerminalContextRegistry::new()),
            Arc::new(ShellIntegrationManager::new()),
            Arc::new(TerminalMux::new()),
            Arc::new(UnifiedCache::new()),
        )
    }

    #[tokio::test]
    async fn test_cache_stats_tracking() {
        let service = create_service();
        let pane_id = PaneId::new(1);

        assert!(service.load_from_cache(pane_id).await.unwrap().is_none());
        let stats = service.get_cache_stats().await;
        assert_eq!(stats.hit_count, 0);
        assert_eq!(stats.miss_count, 1);

        let context = TerminalContext::new(pane_id);
        service.store_in_cache(pane_id, &context).await;
        assert!(service.load_from_cache(pane_id).await.unwrap().is_some());
        let stats = service.get_cache_stats().await;
        assert_eq!(stats.hit_count, 1);
    }

    #[tokio::test]
    async fn test_clear_all_cache() {
        let service = create_service();

        for id in 1..=3 {
            let pane = PaneId::new(id);
            service
                .store_in_cache(pane, &TerminalContext::new(pane))
                .await;
        }

        service.clear_all_cache().await;
        let stats = service.get_cache_stats().await;
        assert_eq!(stats.total_entries, 0);
        assert!(stats.eviction_count >= 3);
    }
}
