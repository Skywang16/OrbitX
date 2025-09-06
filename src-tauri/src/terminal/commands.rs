/*!
 * 终端上下文管理 Tauri 命令接口
 *
 * 提供前端调用的终端上下文管理命令，包括：
 * - 活跃终端管理
 * - 终端上下文查询
 * - 参数验证和错误处理
 */

use crate::mux::PaneId;
use crate::terminal::{
    ActiveTerminalContextRegistry, ContextError, TerminalContext, TerminalContextService,
};
use std::sync::Arc;
use tauri::State;
use tracing::{debug, error, warn};

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

/// 设置活跃终端面板
///
/// 将指定的面板ID设置为当前活跃的终端。这会更新后端的活跃终端注册表，
/// 并触发相应的事件通知前端。
///
/// # Arguments
/// * `pane_id` - 要设置为活跃的面板ID
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(())` - 设置成功
/// * `Err(String)` - 设置失败的错误信息
///
/// # Examples
/// ```javascript
/// // 前端调用示例
/// await invoke('set_active_pane', { paneId: 123 });
/// ```
#[tauri::command]
pub async fn set_active_pane(
    pane_id: u32,
    state: State<'_, TerminalContextState>,
) -> Result<(), String> {
    debug!("设置活跃终端面板: pane_id={}", pane_id);

    // 参数验证
    if pane_id == 0 {
        let error_msg = "面板ID不能为0".to_string();
        warn!("{}", error_msg);
        return Err(error_msg);
    }

    let pane_id = PaneId::new(pane_id);

    // 调用注册表设置活跃终端
    match state.registry.set_active_pane(pane_id) {
        Ok(()) => {
            debug!("成功设置活跃终端面板: pane_id={:?}", pane_id);
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("设置活跃终端面板失败: {}", e);
            error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 获取当前活跃终端面板ID
///
/// 返回当前活跃的终端面板ID。如果没有活跃的终端，返回None。
///
/// # Arguments
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(Some(u32))` - 当前活跃的面板ID
/// * `Ok(None)` - 没有活跃的终端
/// * `Err(String)` - 获取失败的错误信息
///
/// # Examples
/// ```javascript
/// // 前端调用示例
/// const activePaneId = await invoke('get_active_pane');
/// if (activePaneId !== null) {
///     console.log('当前活跃终端:', activePaneId);
/// }
/// ```
#[tauri::command]
pub async fn get_active_pane(
    state: State<'_, TerminalContextState>,
) -> Result<Option<u32>, String> {
    debug!("获取当前活跃终端面板");

    let active_pane = state.registry.get_active_pane();
    let result = active_pane.map(|pane_id| pane_id.as_u32());

    debug!("当前活跃终端面板: {:?}", result);
    Ok(result)
}

/// 获取指定终端的上下文信息
///
/// 根据提供的面板ID获取终端的完整上下文信息，包括当前工作目录、
/// Shell类型、命令历史等。如果不提供面板ID，则获取当前活跃终端的上下文。
///
/// # Arguments
/// * `pane_id` - 可选的面板ID，如果为None则使用活跃终端
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(TerminalContext)` - 终端上下文信息
/// * `Err(String)` - 获取失败的错误信息
///
/// # Examples
/// ```javascript
/// // 获取指定终端的上下文
/// const context = await invoke('get_terminal_context', { paneId: 123 });
/// console.log('工作目录:', context.currentWorkingDirectory);
///
/// // 获取活跃终端的上下文
/// const activeContext = await invoke('get_terminal_context');
/// ```
#[tauri::command]
pub async fn get_terminal_context(
    pane_id: Option<u32>,
    state: State<'_, TerminalContextState>,
) -> Result<TerminalContext, String> {
    debug!("获取终端上下文: pane_id={:?}", pane_id);

    // 参数验证
    if let Some(id) = pane_id {
        if id == 0 {
            let error_msg = "面板ID不能为0".to_string();
            warn!("{}", error_msg);
            return Err(error_msg);
        }
    }

    let pane_id = pane_id.map(PaneId::new);

    // 使用上下文服务获取终端上下文，支持回退逻辑
    match state
        .context_service
        .get_context_with_fallback(pane_id)
        .await
    {
        Ok(context) => {
            debug!(
                "成功获取终端上下文: pane_id={:?}, cwd={:?}",
                context.pane_id, context.current_working_directory
            );
            Ok(context)
        }
        Err(e) => {
            let error_msg = format!("获取终端上下文失败: {}", e);
            error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 获取当前活跃终端的上下文信息
///
/// 专门用于获取当前活跃终端的上下文信息的便捷方法。
/// 这是 `get_terminal_context(None)` 的简化版本。
///
/// # Arguments
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(TerminalContext)` - 活跃终端的上下文信息
/// * `Err(String)` - 获取失败的错误信息
///
/// # Examples
/// ```javascript
/// // 前端调用示例
/// const activeContext = await invoke('get_active_terminal_context');
/// console.log('活跃终端工作目录:', activeContext.currentWorkingDirectory);
/// console.log('Shell类型:', activeContext.shellType);
/// ```
#[tauri::command]
pub async fn get_active_terminal_context(
    state: State<'_, TerminalContextState>,
) -> Result<TerminalContext, String> {
    debug!("获取活跃终端上下文");

    match state.context_service.get_active_context().await {
        Ok(context) => {
            debug!(
                "成功获取活跃终端上下文: pane_id={:?}, cwd={:?}",
                context.pane_id, context.current_working_directory
            );
            Ok(context)
        }
        Err(ContextError::NoActivePane) => {
            // 没有活跃终端时，使用回退逻辑
            debug!("没有活跃终端，使用回退逻辑");
            match state.context_service.get_context_with_fallback(None).await {
                Ok(context) => {
                    debug!("使用回退逻辑成功获取终端上下文");
                    Ok(context)
                }
                Err(e) => {
                    let error_msg = format!("获取活跃终端上下文失败（回退也失败）: {}", e);
                    error!("{}", error_msg);
                    Err(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("获取活跃终端上下文失败: {}", e);
            error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 清除当前活跃终端
///
/// 清除当前设置的活跃终端，使系统回到没有活跃终端的状态。
/// 这通常在所有终端都关闭时调用。
///
/// # Arguments
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(())` - 清除成功
/// * `Err(String)` - 清除失败的错误信息
///
/// # Examples
/// ```javascript
/// // 前端调用示例
/// await invoke('clear_active_pane');
/// ```
#[tauri::command]
pub async fn clear_active_pane(state: State<'_, TerminalContextState>) -> Result<(), String> {
    debug!("清除活跃终端面板");

    match state.registry.clear_active_pane() {
        Ok(()) => {
            debug!("成功清除活跃终端面板");
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("清除活跃终端面板失败: {}", e);
            error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 检查指定面板是否为活跃终端
///
/// 检查给定的面板ID是否是当前活跃的终端。
///
/// # Arguments
/// * `pane_id` - 要检查的面板ID
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(bool)` - true表示是活跃终端，false表示不是
/// * `Err(String)` - 检查失败的错误信息
///
/// # Examples
/// ```javascript
/// // 前端调用示例
/// const isActive = await invoke('is_pane_active', { paneId: 123 });
/// if (isActive) {
///     console.log('面板123是活跃终端');
/// }
/// ```
#[tauri::command]
pub async fn is_pane_active(
    pane_id: u32,
    state: State<'_, TerminalContextState>,
) -> Result<bool, String> {
    debug!("检查面板是否为活跃终端: pane_id={}", pane_id);

    // 参数验证
    if pane_id == 0 {
        let error_msg = "面板ID不能为0".to_string();
        warn!("{}", error_msg);
        return Err(error_msg);
    }

    let pane_id = PaneId::new(pane_id);
    let is_active = state.registry.is_pane_active(pane_id);

    debug!(
        "面板活跃状态检查结果: pane_id={:?}, is_active={}",
        pane_id, is_active
    );
    Ok(is_active)
}

/// 使指定面板的上下文缓存失效
///
/// 强制清除指定面板的缓存上下文信息，下次查询时将重新获取最新数据。
/// 这在终端状态发生重大变化时很有用。
///
/// # Arguments
/// * `pane_id` - 要失效缓存的面板ID
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(())` - 缓存失效成功
/// * `Err(String)` - 操作失败的错误信息
///
/// # Examples
/// ```javascript
/// // 前端调用示例
/// await invoke('invalidate_context_cache', { paneId: 123 });
/// ```
#[tauri::command]
pub async fn invalidate_context_cache(
    pane_id: u32,
    state: State<'_, TerminalContextState>,
) -> Result<(), String> {
    debug!("使上下文缓存失效: pane_id={}", pane_id);

    // 参数验证
    if pane_id == 0 {
        let error_msg = "面板ID不能为0".to_string();
        warn!("{}", error_msg);
        return Err(error_msg);
    }

    let pane_id = PaneId::new(pane_id);
    state.context_service.invalidate_cache(pane_id);

    debug!("成功使上下文缓存失效: pane_id={:?}", pane_id);
    Ok(())
}

/// 清除所有上下文缓存
///
/// 清除所有终端的缓存上下文信息，强制下次查询时重新获取最新数据。
/// 这在系统重置或调试时很有用。
///
/// # Arguments
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(())` - 缓存清除成功
/// * `Err(String)` - 操作失败的错误信息
///
/// # Examples
/// ```javascript
/// // 前端调用示例
/// await invoke('clear_all_context_cache');
/// ```
#[tauri::command]
pub async fn clear_all_context_cache(state: State<'_, TerminalContextState>) -> Result<(), String> {
    debug!("清除所有上下文缓存");

    state.context_service.clear_all_cache();

    debug!("成功清除所有上下文缓存");
    Ok(())
}

/// 获取上下文缓存统计信息
///
/// 返回当前上下文缓存的统计信息，包括缓存命中率、条目数量等。
/// 这对于监控和调试缓存性能很有用。
///
/// # Arguments
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(CacheStats)` - 缓存统计信息
/// * `Err(String)` - 获取失败的错误信息
///
/// # Examples
/// ```javascript
/// // 前端调用示例
/// const stats = await invoke('get_context_cache_stats');
/// console.log('缓存命中率:', stats.hitRate);
/// console.log('缓存条目数:', stats.totalEntries);
/// ```
#[tauri::command]
pub async fn get_context_cache_stats(
    state: State<'_, TerminalContextState>,
) -> Result<crate::terminal::CacheStats, String> {
    debug!("获取上下文缓存统计信息");

    let stats = state.context_service.get_cache_stats();

    debug!(
        "上下文缓存统计: 总条目={}, 命中率={:.2}%",
        stats.total_entries,
        stats.hit_rate * 100.0
    );

    Ok(stats)
}

/// 获取活跃终端注册表统计信息
///
/// 返回活跃终端注册表的统计信息，包括当前活跃终端、事件订阅者数量等。
///
/// # Arguments
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(RegistryStats)` - 注册表统计信息
/// * `Err(String)` - 获取失败的错误信息
///
/// # Examples
/// ```javascript
/// // 前端调用示例
/// const stats = await invoke('get_registry_stats');
/// console.log('当前活跃终端:', stats.globalActivePaneId);
/// console.log('事件订阅者数量:', stats.eventSubscriberCount);
/// ```
#[tauri::command]
pub async fn get_registry_stats(
    state: State<'_, TerminalContextState>,
) -> Result<crate::terminal::context_registry::RegistryStats, String> {
    debug!("获取活跃终端注册表统计信息");

    let stats = state.registry.get_stats();

    debug!(
        "注册表统计: 活跃终端={:?}, 订阅者数量={}",
        stats.global_active_pane, stats.event_subscriber_count
    );

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mux::TerminalMux;
    use crate::shell::ShellIntegrationManager;
    use std::sync::Arc;

    /// 创建测试用的终端上下文状态
    fn create_test_state() -> TerminalContextState {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new().unwrap());
        let terminal_mux = Arc::new(TerminalMux::new());
        let context_service = Arc::new(TerminalContextService::new(
            registry.clone(),
            shell_integration,
            terminal_mux,
        ));

        TerminalContextState::new(registry, context_service)
    }

    #[tokio::test]
    async fn test_set_and_get_active_pane() {
        let state = create_test_state();
        let pane_id = 123u32;

        // 初始状态应该没有活跃终端
        let result = state.registry.get_active_pane();
        assert_eq!(result, None);

        // 设置活跃终端
        let result = state.registry.set_active_pane(PaneId::new(pane_id));
        assert!(result.is_ok());

        // 验证活跃终端已设置
        let result = state.registry.get_active_pane();
        assert_eq!(result, Some(PaneId::new(pane_id)));
    }

    #[tokio::test]
    async fn test_invalid_pane_id_validation() {
        // 测试参数验证逻辑
        assert!(PaneId::new(0).as_u32() == 0); // 验证0是有效的PaneId值

        // 实际的验证逻辑在命令函数中，这里测试基础功能
        let state = create_test_state();
        let valid_pane_id = PaneId::new(123);

        // 测试设置有效的面板ID
        let result = state.registry.set_active_pane(valid_pane_id);
        assert!(result.is_ok());

        // 测试检查面板活跃状态
        let is_active = state.registry.is_pane_active(valid_pane_id);
        assert!(is_active);
    }

    #[tokio::test]
    async fn test_is_pane_active() {
        let state = create_test_state();
        let pane_id = PaneId::new(123);

        // 初始状态下面板不应该是活跃的
        let is_active = state.registry.is_pane_active(pane_id);
        assert!(!is_active);

        // 设置活跃终端后应该返回true
        state.registry.set_active_pane(pane_id).unwrap();
        let is_active = state.registry.is_pane_active(pane_id);
        assert!(is_active);

        // 其他面板应该不是活跃的
        let other_pane = PaneId::new(456);
        let is_active = state.registry.is_pane_active(other_pane);
        assert!(!is_active);
    }

    #[tokio::test]
    async fn test_clear_active_pane() {
        let state = create_test_state();
        let pane_id = PaneId::new(123);

        // 设置活跃终端
        state.registry.set_active_pane(pane_id).unwrap();
        assert_eq!(state.registry.get_active_pane(), Some(pane_id));

        // 清除活跃终端
        let result = state.registry.clear_active_pane();
        assert!(result.is_ok());

        // 验证活跃终端已清除
        assert_eq!(state.registry.get_active_pane(), None);
    }

    #[tokio::test]
    async fn test_get_terminal_context_fallback() {
        let state = create_test_state();

        // 没有活跃终端时，应该返回默认上下文
        let result = state.context_service.get_context_with_fallback(None).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.current_working_directory, Some("~".to_string()));
        assert!(matches!(
            context.shell_type,
            Some(crate::terminal::ShellType::Bash)
        ));
    }

    #[tokio::test]
    async fn test_get_active_terminal_context_fallback() {
        let state = create_test_state();

        // 没有活跃终端时，get_active_context应该返回错误
        let result = state.context_service.get_active_context().await;
        assert!(matches!(result, Err(ContextError::NoActivePane)));

        // 但是get_context_with_fallback应该返回默认上下文
        let result = state.context_service.get_context_with_fallback(None).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.current_working_directory, Some("~".to_string()));
        assert!(!context.shell_integration_enabled);
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let state = create_test_state();
        let pane_id = PaneId::new(123);

        // 测试缓存失效操作
        state.context_service.invalidate_cache(pane_id);

        // 测试清除所有缓存
        state.context_service.clear_all_cache();

        // 测试获取缓存统计
        let stats = state.context_service.get_cache_stats();
        assert_eq!(stats.total_entries, 0); // 初始状态应该没有缓存条目
    }

    #[tokio::test]
    async fn test_get_registry_stats() {
        let state = create_test_state();

        // 测试获取注册表统计
        let stats = state.registry.get_stats();
        assert_eq!(stats.global_active_pane, None); // 初始状态没有活跃终端
        assert_eq!(stats.window_active_pane_count, 0);
    }

    #[tokio::test]
    async fn test_state_creation_and_access() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let shell_integration = Arc::new(ShellIntegrationManager::new().unwrap());
        let terminal_mux = Arc::new(TerminalMux::new());
        let context_service = Arc::new(TerminalContextService::new(
            registry.clone(),
            shell_integration,
            terminal_mux,
        ));

        let state = TerminalContextState::new(registry.clone(), context_service.clone());

        // 验证状态访问方法
        assert!(Arc::ptr_eq(state.registry(), &registry));
        assert!(Arc::ptr_eq(state.context_service(), &context_service));
    }

    #[tokio::test]
    async fn test_context_service_integration() {
        let state = create_test_state();
        let pane_id = PaneId::new(123);

        // 设置活跃终端
        state.registry.set_active_pane(pane_id).unwrap();

        // 测试获取活跃终端上下文（应该失败，因为面板不存在于mux中）
        let result = state.context_service.get_active_context().await;
        assert!(matches!(result, Err(ContextError::PaneNotFound { .. })));

        // 测试使用回退逻辑
        let result = state
            .context_service
            .get_context_with_fallback(Some(pane_id))
            .await;
        assert!(result.is_ok());

        let context = result.unwrap();
        // 由于面板不存在，应该回退到默认上下文
        assert_eq!(context.current_working_directory, Some("~".to_string()));
    }
}
