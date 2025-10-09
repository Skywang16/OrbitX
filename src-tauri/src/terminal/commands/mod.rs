/*!
 * 终端上下文管理 Tauri 命令接口
 *
 * 提供前端调用的终端上下文管理命令，包括：
 * - 活跃终端管理
 * - 终端上下文查询
 * - 参数验证和错误处理
 */

use crate::terminal::{ActiveTerminalContextRegistry, TerminalContextService};
use std::sync::Arc;

/// 终端上下文管理状态
///
/// 包含活跃终端注册表和终端上下文服务的共享状态
pub struct TerminalContextState {
    /// 活跃终端上下文注册表
    pub registry: Arc<ActiveTerminalContextRegistry>,
    /// 终端上下文服务
    pub context_service: Arc<TerminalContextService>,
}

impl TerminalContextState {
    /// 创建新的终端上下文状态
    ///
    /// # Arguments
    /// * `registry` - 活跃终端上下文注册表
    /// * `context_service` - 终端上下文服务
    ///
    /// # Returns
    /// * `TerminalContextState` - 新的状态实例
    pub fn new(
        registry: Arc<ActiveTerminalContextRegistry>,
        context_service: Arc<TerminalContextService>,
    ) -> Self {
        Self {
            registry,
            context_service,
        }
    }

    /// 获取活跃终端注册表的引用
    pub fn registry(&self) -> &Arc<ActiveTerminalContextRegistry> {
        &self.registry
    }

    /// 获取终端上下文服务的引用
    pub fn context_service(&self) -> &Arc<TerminalContextService> {
        &self.context_service
    }
}

// 导出各功能域模块
pub mod cache;
pub mod context;
pub mod pane;
pub mod stats;
pub mod stream;

// 重新导出所有命令函数，保持向后兼容
pub use cache::{terminal_context_clear_all_cache, terminal_context_invalidate_cache};
pub use context::{terminal_context_get, terminal_context_get_active};
pub use pane::{
    terminal_context_clear_active_pane, terminal_context_get_active_pane,
    terminal_context_is_pane_active, terminal_context_set_active_pane,
};
pub use stats::{terminal_context_get_cache_stats, terminal_context_get_registry_stats};
pub use stream::{terminal_subscribe_output, terminal_subscribe_output_cancel};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mux::TerminalMux;
    use crate::shell::ShellIntegrationManager;
    use crate::storage::cache::UnifiedCache;
    use std::sync::Arc;

    /// 创建测试用的终端上下文状态
    pub(crate) fn create_test_state() -> TerminalContextState {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new());
        let terminal_mux = Arc::new(TerminalMux::new());
        let cache = Arc::new(UnifiedCache::new());
        let context_service = Arc::new(TerminalContextService::new(
            registry.clone(),
            shell_integration,
            terminal_mux,
            cache,
        ));

        TerminalContextState::new(registry, context_service)
    }

    #[tokio::test]
    async fn test_state_creation_and_access() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new());
        let terminal_mux = Arc::new(TerminalMux::new());
        let cache = Arc::new(UnifiedCache::new());
        let context_service = Arc::new(TerminalContextService::new(
            registry.clone(),
            shell_integration,
            terminal_mux,
            cache,
        ));

        let state = TerminalContextState::new(registry.clone(), context_service.clone());

        // 验证状态访问方法
        assert!(Arc::ptr_eq(state.registry(), &registry));
        assert!(Arc::ptr_eq(state.context_service(), &context_service));
    }
}
