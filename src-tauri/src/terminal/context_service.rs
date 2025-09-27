use crate::mux::{PaneId, TerminalMux};
use crate::shell::{ContextServiceIntegration, ShellIntegrationManager};
use crate::terminal::{
    context_registry::ActiveTerminalContextRegistry, types::*, CommandInfo, ShellType,
    TerminalContext,
};
use crate::utils::error::AppResult;
use anyhow::anyhow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct CachedContext {
    pub context: TerminalContext,
    pub cached_at: Instant,
    pub ttl: Duration,
}

impl CachedContext {
    pub fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub total_entries: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub eviction_count: u64,
    pub hit_rate: f64,
}

impl CacheStats {
    pub fn new() -> Self {
        Self {
            total_entries: 0,
            hit_count: 0,
            miss_count: 0,
            eviction_count: 0,
            hit_rate: 0.0,
        }
    }

    pub fn calculate_hit_rate(&mut self) {
        let total_requests = self.hit_count + self.miss_count;
        if total_requests > 0 {
            self.hit_rate = self.hit_count as f64 / total_requests as f64;
        }
    }
}

/// 终端上下文服务
///
/// 提供统一的终端上下文访问接口，整合活跃终端注册表、Shell集成管理器和终端多路复用器的信息
pub struct TerminalContextService {
    /// 活跃终端注册表
    registry: Arc<ActiveTerminalContextRegistry>,
    /// Shell集成管理器
    shell_integration: Arc<ShellIntegrationManager>,
    /// 终端多路复用器
    terminal_mux: Arc<TerminalMux>,
    /// 上下文缓存
    cache: Arc<RwLock<HashMap<PaneId, CachedContext>>>,
    /// 缓存统计
    cache_stats: Arc<RwLock<CacheStats>>,
    /// 默认缓存TTL
    default_cache_ttl: Duration,
    /// 查询超时时间
    query_timeout: Duration,
}

impl TerminalContextService {
    /// 创建新的终端上下文服务（不设置集成）
    pub fn new(
        registry: Arc<ActiveTerminalContextRegistry>,
        shell_integration: Arc<ShellIntegrationManager>,
        terminal_mux: Arc<TerminalMux>,
    ) -> Self {
        Self {
            registry,
            shell_integration,
            terminal_mux,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_stats: Arc::new(RwLock::new(CacheStats::new())),
            default_cache_ttl: Duration::from_secs(30), // 30秒缓存TTL
            query_timeout: Duration::from_millis(5000), // 5秒查询超时
        }
    }

    /// 创建新的终端上下文服务并设置Shell集成
    pub fn new_with_integration(
        registry: Arc<ActiveTerminalContextRegistry>,
        shell_integration: Arc<ShellIntegrationManager>,
        terminal_mux: Arc<TerminalMux>,
    ) -> Arc<Self> {
        let service = Arc::new(Self::new(registry, shell_integration, terminal_mux));

        service.shell_integration.set_context_service_integration(
            Arc::downgrade(&service) as std::sync::Weak<dyn ContextServiceIntegration>
        );

        service
    }

    /// 获取活跃终端的上下文
    ///
    /// # Returns
    /// * `Ok(TerminalContext)` - 活跃终端的上下文信息
    /// * `Err(anyhow::Error)` - 获取失败的错误信息
    pub async fn get_active_context(&self) -> AppResult<TerminalContext> {
        let active_pane_id = self
            .registry
            .terminal_context_get_active_pane()
            .ok_or(anyhow!("没有活跃的终端"))?;

        debug!("获取活跃终端上下文: pane_id={:?}", active_pane_id);
        self.get_context_by_pane(active_pane_id).await
    }

    /// 根据面板ID获取终端上下文
    ///
    /// # Arguments
    /// * `pane_id` - 面板ID
    ///
    /// # Returns
    /// * `Ok(TerminalContext)` - 指定面板的上下文信息
    /// * `Err(anyhow::Error)` - 获取失败的错误信息
    pub async fn get_context_by_pane(&self, pane_id: PaneId) -> AppResult<TerminalContext> {
        // 首先检查缓存
        if let Some(cached_context) = self.get_cached_context_internal(pane_id) {
            debug!("从缓存获取上下文: pane_id={:?}", pane_id);
            self.increment_cache_hit();
            return Ok(cached_context.context);
        }

        self.increment_cache_miss();

        // 使用超时机制查询上下文
        let context = timeout(self.query_timeout, self.query_context_internal(pane_id))
            .await
            .map_err(|_| anyhow!("查询终端上下文超时"))?
            .map_err(|e| {
                warn!("查询终端上下文失败: pane_id={:?}, error={}", pane_id, e);
                e.context("查询终端上下文失败")
            })?;

        // 缓存结果
        self.cache_context_internal(pane_id, context.clone());

        // 发送上下文更新事件
        self.send_context_updated_event(pane_id, &context);

        Ok(context)
    }

    /// 获取上下文，支持回退逻辑
    ///
    /// # Arguments
    /// * `pane_id` - 可选的面板ID，如果为None则使用活跃终端
    ///
    /// # Returns
    /// * `Ok(TerminalContext)` - 终端上下文信息
    /// * `Err(anyhow::Error)` - 获取失败的错误信息
    pub async fn get_context_with_fallback(
        &self,
        pane_id: Option<PaneId>,
    ) -> AppResult<TerminalContext> {
        // 1. 尝试获取指定面板的上下文
        if let Some(pane_id) = pane_id {
            if let Ok(context) = self.get_context_by_pane(pane_id).await {
                return Ok(context);
            }
            debug!("指定面板上下文获取失败，尝试回退: pane_id={:?}", pane_id);
        }

        // 2. 回退到活跃终端
        if let Ok(context) = self.get_active_context().await {
            return Ok(context);
        }
        debug!("活跃终端上下文获取失败，尝试缓存回退");

        // 3. 使用缓存的上下文
        if let Some(cached_context) = self.get_any_cached_context() {
            debug!("使用缓存的上下文作为回退");
            return Ok(cached_context);
        }

        // 4. 创建默认上下文（使用 ~ 作为默认值）
        debug!("创建默认上下文作为最后回退");
        Ok(self.create_default_context())
    }

    /// 获取活跃终端的当前工作目录
    ///
    /// # Returns
    /// * `Ok(String)` - 当前工作目录路径
    /// * `Err(anyhow::Error)` - 获取失败的错误信息
    pub async fn get_active_cwd(&self) -> AppResult<String> {
        let active_pane_id = self
            .registry
            .terminal_context_get_active_pane()
            .ok_or(anyhow!("没有活跃的终端"))?;

        self.shell_get_pane_cwd(active_pane_id).await
    }

    /// 获取指定面板的当前工作目录
    ///
    /// # Arguments
    /// * `pane_id` - 面板ID
    ///
    /// # Returns
    /// * `Ok(String)` - 当前工作目录路径
    /// * `Err(anyhow::Error)` - 获取失败的错误信息
    pub async fn shell_get_pane_cwd(&self, pane_id: PaneId) -> AppResult<String> {
        if !self.terminal_mux.pane_exists(pane_id) {
            return Err(anyhow!("面板不存在: {}", pane_id));
        }

        // 从Shell集成管理器获取CWD
        self.terminal_mux
            .shell_get_pane_cwd(pane_id)
            .ok_or_else(|| anyhow!("无法获取当前工作目录"))
    }

    /// 获取活跃终端的Shell类型
    ///
    /// # Returns
    /// * `Ok(ShellType)` - Shell类型
    /// * `Err(anyhow::Error)` - 获取失败的错误信息
    pub async fn get_active_shell_type(&self) -> AppResult<ShellType> {
        let active_pane_id = self
            .registry
            .terminal_context_get_active_pane()
            .ok_or(anyhow!("没有活跃的终端"))?;

        self.get_pane_shell_type(active_pane_id).await
    }

    /// 获取指定面板的Shell类型
    ///
    /// # Arguments
    /// * `pane_id` - 面板ID
    ///
    /// # Returns
    /// * `Ok(ShellType)` - Shell类型
    /// * `Err(anyhow::Error)` - 获取失败的错误信息
    pub async fn get_pane_shell_type(&self, pane_id: PaneId) -> AppResult<ShellType> {
        if !self.terminal_mux.pane_exists(pane_id) {
            return Err(anyhow!("面板不存在: {}", pane_id));
        }

        // 从Shell集成管理器获取Shell状态
        let shell_state = self.terminal_mux.get_pane_shell_state(pane_id);

        if let Some(state) = shell_state {
            if let Some(shell_type) = state.shell_type {
                return Ok(self.convert_shell_type(shell_type));
            }
        }

        // 默认返回Bash
        Ok(ShellType::Bash)
    }

    /// 使缓存失效
    ///
    /// # Arguments
    /// * `pane_id` - 要失效的面板ID
    pub fn invalidate_cache(&self, pane_id: PaneId) {
        if let Ok(mut cache) = self.cache.write() {
            if cache.remove(&pane_id).is_some() {
                debug!("缓存已失效: pane_id={:?}", pane_id);
                self.increment_eviction_count();
            }
        }
    }

    /// 清除所有缓存
    pub fn clear_all_cache(&self) {
        if let Ok(mut cache) = self.cache.write() {
            let evicted_count = cache.len();
            cache.clear();
            debug!("所有缓存已清除，失效数量: {}", evicted_count);

            if let Ok(mut stats) = self.cache_stats.write() {
                stats.eviction_count += evicted_count as u64;
                stats.total_entries = 0;
            }
        }
    }

    /// 获取缓存统计信息
    ///
    /// # Returns
    /// * `CacheStats` - 缓存统计信息
    pub fn get_cache_stats(&self) -> CacheStats {
        if let Ok(mut stats) = self.cache_stats.write() {
            if let Ok(cache) = self.cache.read() {
                stats.total_entries = cache.len();
            }
            stats.calculate_hit_rate();
            stats.clone()
        } else {
            CacheStats::new()
        }
    }

    // 私有方法

    /// 内部查询上下文的实现
    async fn query_context_internal(&self, pane_id: PaneId) -> AppResult<TerminalContext> {
        if !self.terminal_mux.pane_exists(pane_id) {
            return Err(anyhow!("面板不存在: {}", pane_id));
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
                context.set_current_command(Some(self.convert_command_info(current_cmd)));
            }

            let history: Vec<CommandInfo> = shell_state
                .command_history
                .into_iter()
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

    /// 从缓存获取上下文（内部使用）
    fn get_cached_context_internal(&self, pane_id: PaneId) -> Option<CachedContext> {
        if let Ok(mut cache) = self.cache.write() {
            if let Some(cached) = cache.get(&pane_id) {
                if !cached.is_expired() {
                    return Some(cached.clone());
                } else {
                    // 移除过期的缓存
                    cache.remove(&pane_id);
                    self.increment_eviction_count();
                }
            }
        }
        None
    }

    /// 缓存上下文（内部使用）
    fn cache_context_internal(&self, pane_id: PaneId, context: TerminalContext) {
        let cached_context = CachedContext {
            context,
            cached_at: Instant::now(),
            ttl: self.default_cache_ttl,
        };

        if let Ok(mut cache) = self.cache.write() {
            cache.insert(pane_id, cached_context);
            debug!("上下文已缓存: pane_id={:?}", pane_id);
        }
    }

    /// 从缓存获取上下文（测试用）
    #[cfg(test)]
    pub fn get_cached_context(&self, pane_id: PaneId) -> Option<CachedContext> {
        self.get_cached_context_internal(pane_id)
    }

    /// 缓存上下文（测试用）
    #[cfg(test)]
    pub fn cache_context(&self, pane_id: PaneId, context: TerminalContext) {
        self.cache_context_internal(pane_id, context)
    }

    /// 获取任意缓存的上下文（用于回退）
    fn get_any_cached_context(&self) -> Option<TerminalContext> {
        if let Ok(cache) = self.cache.read() {
            for cached in cache.values() {
                if !cached.is_expired() {
                    return Some(cached.context.clone());
                }
            }
        }
        None
    }

    /// 创建默认上下文
    fn create_default_context(&self) -> TerminalContext {
        let mut context = TerminalContext::new(PaneId::new(0));
        context.update_cwd("~".to_string());
        context.update_shell_type(ShellType::Bash);
        context.set_shell_integration(false);
        context
    }

    /// 转换Shell类型
    fn convert_shell_type(&self, shell_type: crate::shell::ShellType) -> ShellType {
        match shell_type {
            crate::shell::ShellType::Bash => ShellType::Bash,
            crate::shell::ShellType::Zsh => ShellType::Zsh,
            crate::shell::ShellType::Fish => ShellType::Fish,
            crate::shell::ShellType::PowerShell => ShellType::PowerShell,
            crate::shell::ShellType::Cmd => ShellType::Cmd,
            crate::shell::ShellType::Nushell => ShellType::Other("Nushell".to_string()),
            crate::shell::ShellType::Unknown(name) => ShellType::Other(name),
        }
    }

    /// 转换命令信息
    fn convert_command_info(&self, cmd: crate::shell::CommandInfo) -> CommandInfo {
        // 解析命令行文本以获取命令和参数
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
            working_directory: cmd.working_directory,
        }
    }

    /// 增加缓存命中计数
    fn increment_cache_hit(&self) {
        if let Ok(mut stats) = self.cache_stats.write() {
            stats.hit_count += 1;
        }
    }

    /// 增加缓存未命中计数
    fn increment_cache_miss(&self) {
        if let Ok(mut stats) = self.cache_stats.write() {
            stats.miss_count += 1;
        }
    }

    /// 增加缓存淘汰计数
    fn increment_eviction_count(&self) {
        if let Ok(mut stats) = self.cache_stats.write() {
            stats.eviction_count += 1;
        }
    }

    /// 发送上下文更新事件
    fn send_context_updated_event(&self, pane_id: PaneId, context: &TerminalContext) {
        let event = TerminalContextEvent::PaneContextUpdated {
            pane_id,
            context: context.clone(),
        };

        if let Err(e) = self.registry.send_event(event) {
            warn!("发送上下文更新事件失败: {}", e);
        }
    }
}

impl ContextServiceIntegration for TerminalContextService {
    fn invalidate_cache(&self, pane_id: PaneId) {
        self.invalidate_cache(pane_id);
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

impl Default for TerminalContextService {
    fn default() -> Self {
        // 注意：这个默认实现主要用于测试，实际使用时应该通过构造函数创建
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration =
            Arc::new(ShellIntegrationManager::new().expect("创建Shell集成管理器失败"));
        let terminal_mux = Arc::new(TerminalMux::new());

        // 由于Default trait要求返回Self而不是Arc<Self>，我们需要特殊处理
        let service = Self {
            registry,
            shell_integration,
            terminal_mux,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_stats: Arc::new(RwLock::new(CacheStats::new())),
            default_cache_ttl: Duration::from_secs(30),
            query_timeout: Duration::from_millis(5000),
        };

        service
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_context_service_creation() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new().unwrap());
        let terminal_mux = Arc::new(TerminalMux::new());

        let service = TerminalContextService::new(registry, shell_integration, terminal_mux);

        // 验证初始状态
        let stats = service.get_cache_stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.hit_count, 0);
        assert_eq!(stats.miss_count, 0);
    }

    #[tokio::test]
    async fn test_no_active_pane_error() {
        let service = TerminalContextService::default();

        // 没有活跃终端时应该返回错误
        let result = service.get_active_context().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "没有活跃的终端");
    }

    #[tokio::test]
    async fn test_pane_not_found_error() {
        let service = TerminalContextService::default();
        let non_existent_pane = PaneId::new(999);

        // 不存在的面板应该返回错误
        let result = service.get_context_by_pane(non_existent_pane).await;
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        println!("实际错误消息: '{}'", error_msg);
        assert!(
            error_msg.contains("面板不存在")
                || error_msg.contains("pane")
                || error_msg.contains("查询终端上下文失败")
        );
    }

    #[tokio::test]
    async fn test_fallback_to_default_context() {
        let service = TerminalContextService::default();

        // 使用回退逻辑应该返回默认上下文
        let result = service.get_context_with_fallback(None).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.pane_id, PaneId::new(0));
        assert_eq!(context.current_working_directory, Some("~".to_string()));
        assert!(matches!(context.shell_type, Some(ShellType::Bash)));
        assert!(!context.shell_integration_enabled);
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let service = TerminalContextService::default();
        let pane_id = PaneId::new(1);

        let mut test_context = TerminalContext::new(pane_id);
        test_context.update_cwd("/test/path".to_string());
        service.cache_context(pane_id, test_context.clone());

        // 验证缓存命中
        let cached = service.get_cached_context(pane_id);
        assert!(cached.is_some());
        assert_eq!(
            cached.unwrap().context.current_working_directory,
            Some("/test/path".to_string())
        );

        // 验证统计信息
        let stats = service.get_cache_stats();
        assert_eq!(stats.total_entries, 1);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new().unwrap());
        let terminal_mux = Arc::new(TerminalMux::new());

        let mut service = TerminalContextService::new(registry, shell_integration, terminal_mux);
        service.default_cache_ttl = Duration::from_millis(10); // 10ms TTL

        let pane_id = PaneId::new(1);
        let test_context = TerminalContext::new(pane_id);
        service.cache_context(pane_id, test_context);

        // 立即检查应该能获取到缓存
        assert!(service.get_cached_context(pane_id).is_some());

        // 等待缓存过期
        sleep(Duration::from_millis(20)).await;

        // 过期后应该获取不到缓存
        assert!(service.get_cached_context(pane_id).is_none());
    }

    #[tokio::test]
    async fn test_active_pane_context_integration() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new().unwrap());
        let terminal_mux = Arc::new(TerminalMux::new());
        let service =
            TerminalContextService::new(registry.clone(), shell_integration, terminal_mux);

        let pane_id = PaneId::new(1);

        registry.terminal_context_set_active_pane(pane_id).unwrap();

        // 验证活跃终端状态
        assert_eq!(registry.terminal_context_get_active_pane(), Some(pane_id));
        assert!(registry.terminal_context_is_pane_active(pane_id));

        // 测试获取活跃终端上下文（应该失败，因为面板不存在）
        let result = service.get_active_context().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_context_with_fallback_scenarios() {
        let service = TerminalContextService::default();

        // 场景1：指定不存在的面板ID，应该回退到默认上下文
        let non_existent_pane = PaneId::new(999);
        let result = service
            .get_context_with_fallback(Some(non_existent_pane))
            .await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.current_working_directory, Some("~".to_string()));
        assert!(matches!(context.shell_type, Some(ShellType::Bash)));

        // 场景2：不指定面板ID，没有活跃终端，应该回退到默认上下文
        let result = service.get_context_with_fallback(None).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.current_working_directory, Some("~".to_string()));
        assert!(!context.shell_integration_enabled);
    }

    #[tokio::test]
    async fn test_context_service_integration_creation() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new().unwrap());
        let terminal_mux = Arc::new(TerminalMux::new());

        // 测试带集成的创建方法
        let service =
            TerminalContextService::new_with_integration(registry, shell_integration, terminal_mux);

        // 验证服务创建成功
        let stats = service.get_cache_stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.hit_count, 0);
        assert_eq!(stats.miss_count, 0);
    }

    #[test]
    fn test_context_service_integration_trait() {
        let service = TerminalContextService::default();
        let pane_id = PaneId::new(1);

        // 测试缓存失效
        let test_context = TerminalContext::new(pane_id);
        service.cache_context(pane_id, test_context);
        assert!(service.get_cached_context(pane_id).is_some());

        // 通过trait方法失效缓存
        let integration: &dyn ContextServiceIntegration = &service;
        integration.invalidate_cache(pane_id);
        assert!(service.get_cached_context(pane_id).is_none());

        // 测试事件发送（这里只测试不会panic，实际事件发送需要有订阅者）
        integration.send_cwd_changed_event(pane_id, Some("/old".to_string()), "/new".to_string());
        integration.send_shell_integration_changed_event(pane_id, true);
    }

    #[tokio::test]
    async fn test_cwd_query_methods() {
        let service = TerminalContextService::default();

        // 测试获取活跃终端CWD（没有活跃终端）
        let result = service.get_active_cwd().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "没有活跃的终端");

        // 测试获取不存在面板的CWD
        let non_existent_pane = PaneId::new(999);
        let result = service.shell_get_pane_cwd(non_existent_pane).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("面板不存在"));
    }

    #[tokio::test]
    async fn test_shell_type_query_methods() {
        let service = TerminalContextService::default();

        // 测试获取活跃终端Shell类型（没有活跃终端）
        let result = service.get_active_shell_type().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "没有活跃的终端");

        // 测试获取不存在面板的Shell类型
        let non_existent_pane = PaneId::new(999);
        let result = service.get_pane_shell_type(non_existent_pane).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("面板不存在"));
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let service = TerminalContextService::default();
        let pane_id = PaneId::new(1);

        // 缓存一个上下文
        let test_context = TerminalContext::new(pane_id);
        service.cache_context(pane_id, test_context);

        // 验证缓存存在
        assert!(service.get_cached_context(pane_id).is_some());

        // 使缓存失效
        service.invalidate_cache(pane_id);

        // 验证缓存已被移除
        assert!(service.get_cached_context(pane_id).is_none());
    }

    #[tokio::test]
    async fn test_clear_all_cache() {
        let service = TerminalContextService::default();

        // 缓存多个上下文
        for i in 1..=3 {
            let pane_id = PaneId::new(i);
            let test_context = TerminalContext::new(pane_id);
            service.cache_context(pane_id, test_context);
        }

        // 验证缓存存在
        let stats_before = service.get_cache_stats();
        assert_eq!(stats_before.total_entries, 3);

        // 清除所有缓存
        service.clear_all_cache();

        // 验证所有缓存已被清除
        let stats_after = service.get_cache_stats();
        assert_eq!(stats_after.total_entries, 0);
        assert_eq!(stats_after.eviction_count, 3);
    }

    #[test]
    fn test_shell_type_conversion() {
        let service = TerminalContextService::default();

        // 测试各种Shell类型的转换
        assert!(matches!(
            service.convert_shell_type(crate::shell::ShellType::Bash),
            ShellType::Bash
        ));
        assert!(matches!(
            service.convert_shell_type(crate::shell::ShellType::Zsh),
            ShellType::Zsh
        ));
        assert!(matches!(
            service.convert_shell_type(crate::shell::ShellType::Fish),
            ShellType::Fish
        ));
        assert!(matches!(
            service.convert_shell_type(crate::shell::ShellType::PowerShell),
            ShellType::PowerShell
        ));
        assert!(matches!(
            service.convert_shell_type(crate::shell::ShellType::Cmd),
            ShellType::Cmd
        ));

        if let ShellType::Other(name) =
            service.convert_shell_type(crate::shell::ShellType::Unknown("custom".to_string()))
        {
            assert_eq!(name, "custom");
        } else {
            panic!("Shell类型转换失败");
        }
    }

    #[test]
    fn test_cache_stats_calculation() {
        let mut stats = CacheStats::new();

        // 初始状态
        assert_eq!(stats.hit_rate, 0.0);

        // 添加一些统计数据
        stats.hit_count = 7;
        stats.miss_count = 3;
        stats.calculate_hit_rate();

        // 验证命中率计算
        assert_eq!(stats.hit_rate, 0.7);
    }
}
