/*!
 * 活跃终端上下文注册表实现
 *
 * 提供线程安全的活跃终端状态管理和事件发送机制
 */

use crate::mux::PaneId;
use crate::terminal::types::TerminalContextEvent;
use crate::utils::error::AppResult;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tracing::{debug, warn};

/// 窗口ID类型（为未来多窗口支持预留）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WindowId(pub u32);

impl WindowId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl From<u32> for WindowId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

/// 活跃终端上下文注册表
///
/// 负责维护当前活跃终端的状态，提供线程安全的查询和更新操作
#[derive(Debug)]
pub struct ActiveTerminalContextRegistry {
    /// 全局活跃终端
    global_active_pane: Arc<RwLock<Option<PaneId>>>,
    /// 按窗口分组的活跃终端（未来扩展）
    window_active_panes: Arc<RwLock<HashMap<WindowId, PaneId>>>,
    /// 事件广播发送器
    event_sender: broadcast::Sender<TerminalContextEvent>,
}

impl ActiveTerminalContextRegistry {
    /// 创建新的活跃终端上下文注册表
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(1000);

        Self {
            global_active_pane: Arc::new(RwLock::new(None)),
            window_active_panes: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
        }
    }

    /// 设置全局活跃终端
    ///
    /// # Arguments
    /// * `pane_id` - 要设置为活跃的面板ID
    ///
    /// # Returns
    /// * `Ok(())` - 设置成功
    /// * `Err(ContextError)` - 设置失败
    pub fn set_active_pane(&self, pane_id: PaneId) -> AppResult<()> {
        let old_pane_id = {
            let mut active_pane = self
                .global_active_pane
                .write()
                .map_err(|e| anyhow!("获取写锁失败: {}", e))?;

            let old_id = *active_pane;

            // 检查是否真的有变化
            if old_id == Some(pane_id) {
                debug!("活跃终端未变化，跳过事件发送: {:?}", pane_id);
                return Ok(());
            }

            *active_pane = Some(pane_id);
            old_id
        };

        debug!("活跃终端已更新: {:?} -> {:?}", old_pane_id, Some(pane_id));

        // 发送事件
        let event = TerminalContextEvent::ActivePaneChanged {
            old_pane_id,
            new_pane_id: Some(pane_id),
        };

        if let Err(e) = self.event_sender.send(event) {
            warn!("发送活跃终端变化事件失败: {}", e);
        }

        Ok(())
    }

    /// 获取当前全局活跃终端
    ///
    /// # Returns
    /// * `Some(PaneId)` - 当前活跃的面板ID
    /// * `None` - 没有活跃的终端
    pub fn get_active_pane(&self) -> Option<PaneId> {
        match self.global_active_pane.read() {
            Ok(active_pane) => *active_pane,
            Err(e) => {
                warn!("获取活跃终端读锁失败: {}", e);
                None
            }
        }
    }

    /// 清除全局活跃终端
    ///
    /// # Returns
    /// * `Ok(())` - 清除成功
    /// * `Err(ContextError)` - 清除失败
    pub fn clear_active_pane(&self) -> AppResult<()> {
        let old_pane_id = {
            let mut active_pane = self
                .global_active_pane
                .write()
                .map_err(|e| anyhow!("获取写锁失败: {}", e))?;

            let old_id = *active_pane;
            *active_pane = None;
            old_id
        };

        if old_pane_id.is_some() {
            debug!("活跃终端已清除: {:?}", old_pane_id);

            // 发送事件
            let event = TerminalContextEvent::ActivePaneChanged {
                old_pane_id,
                new_pane_id: None,
            };

            if let Err(e) = self.event_sender.send(event) {
                warn!("发送活跃终端清除事件失败: {}", e);
            }
        }

        Ok(())
    }

    /// 检查指定面板是否为活跃终端
    ///
    /// # Arguments
    /// * `pane_id` - 要检查的面板ID
    ///
    /// # Returns
    /// * `true` - 该面板是活跃终端
    /// * `false` - 该面板不是活跃终端或获取状态失败
    pub fn is_pane_active(&self, pane_id: PaneId) -> bool {
        match self.global_active_pane.read() {
            Ok(active_pane) => active_pane.map_or(false, |id| id == pane_id),
            Err(e) => {
                warn!("检查面板活跃状态时获取读锁失败: {}", e);
                false
            }
        }
    }

    /// 设置指定窗口的活跃终端（未来扩展功能）
    ///
    /// # Arguments
    /// * `window_id` - 窗口ID
    /// * `pane_id` - 要设置为活跃的面板ID
    ///
    /// # Returns
    /// * `Ok(())` - 设置成功
    /// * `Err(ContextError)` - 设置失败
    pub fn set_window_active_pane(&self, window_id: WindowId, pane_id: PaneId) -> AppResult<()> {
        let old_pane_id = {
            let mut window_panes = self
                .window_active_panes
                .write()
                .map_err(|e| anyhow!("获取窗口活跃终端写锁失败: {}", e))?;

            window_panes.insert(window_id, pane_id)
        };

        debug!(
            "窗口 {} 的活跃终端已更新: {:?} -> {:?}",
            window_id.as_u32(),
            old_pane_id,
            Some(pane_id)
        );

        // 如果这是主窗口或者没有全局活跃终端，也更新全局状态
        if window_id.as_u32() == 0 || self.get_active_pane().is_none() {
            self.set_active_pane(pane_id)?;
        }

        Ok(())
    }

    /// 获取指定窗口的活跃终端（未来扩展功能）
    ///
    /// # Arguments
    /// * `window_id` - 窗口ID
    ///
    /// # Returns
    /// * `Some(PaneId)` - 该窗口当前活跃的面板ID
    /// * `None` - 该窗口没有活跃的终端
    pub fn get_window_active_pane(&self, window_id: WindowId) -> Option<PaneId> {
        match self.window_active_panes.read() {
            Ok(window_panes) => window_panes.get(&window_id).copied(),
            Err(e) => {
                warn!("获取窗口活跃终端读锁失败: {}", e);
                None
            }
        }
    }

    /// 移除指定窗口的活跃终端记录（未来扩展功能）
    ///
    /// # Arguments
    /// * `window_id` - 窗口ID
    ///
    /// # Returns
    /// * `Ok(Option<PaneId>)` - 移除成功，返回之前的活跃面板ID
    /// * `Err(ContextError)` - 移除失败
    pub fn remove_window_active_pane(&self, window_id: WindowId) -> AppResult<Option<PaneId>> {
        let removed_pane = {
            let mut window_panes = self
                .window_active_panes
                .write()
                .map_err(|e| anyhow!("获取窗口活跃终端写锁失败: {}", e))?;

            window_panes.remove(&window_id)
        };

        if let Some(pane_id) = removed_pane {
            debug!(
                "窗口 {} 的活跃终端记录已移除: {}",
                window_id.as_u32(),
                pane_id
            );

            // 如果移除的是当前全局活跃终端，清除全局状态
            if self.get_active_pane() == Some(pane_id) {
                self.clear_active_pane()?;
            }
        }

        Ok(removed_pane)
    }

    /// 获取事件接收器
    ///
    /// 用于订阅终端上下文事件
    ///
    /// # Returns
    /// * `broadcast::Receiver<TerminalContextEvent>` - 事件接收器
    pub fn subscribe_events(&self) -> broadcast::Receiver<TerminalContextEvent> {
        self.event_sender.subscribe()
    }

    /// 发送自定义事件
    ///
    /// # Arguments
    /// * `event` - 要发送的事件
    ///
    /// # Returns
    /// * `Ok(usize)` - 成功发送，返回接收者数量
    /// * `Err(ContextError)` - 发送失败
    pub fn send_event(&self, event: TerminalContextEvent) -> AppResult<usize> {
        self.event_sender
            .send(event)
            .map_err(|e| anyhow!("发送事件失败: {}", e))
    }

    /// 获取注册表统计信息
    ///
    /// # Returns
    /// * `RegistryStats` - 注册表统计信息
    pub fn get_stats(&self) -> RegistryStats {
        let global_active = self.get_active_pane();
        let window_count = self
            .window_active_panes
            .read()
            .map(|panes| panes.len())
            .unwrap_or(0);
        let subscriber_count = self.event_sender.receiver_count();

        RegistryStats {
            global_active_pane: global_active,
            window_active_pane_count: window_count,
            event_subscriber_count: subscriber_count,
        }
    }
}

impl Default for ActiveTerminalContextRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 注册表统计信息
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryStats {
    pub global_active_pane: Option<PaneId>,
    pub window_active_pane_count: usize,
    pub event_subscriber_count: usize,
}

// 实现线程安全的克隆
impl Clone for ActiveTerminalContextRegistry {
    fn clone(&self) -> Self {
        Self {
            global_active_pane: Arc::clone(&self.global_active_pane),
            window_active_panes: Arc::clone(&self.window_active_panes),
            event_sender: self.event_sender.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_active_pane_management() {
        let registry = ActiveTerminalContextRegistry::new();
        let pane_id = PaneId::new(1);

        // 初始状态应该没有活跃终端
        assert_eq!(registry.get_active_pane(), None);
        assert!(!registry.is_pane_active(pane_id));

        // 设置活跃终端
        registry.set_active_pane(pane_id).unwrap();
        assert_eq!(registry.get_active_pane(), Some(pane_id));
        assert!(registry.is_pane_active(pane_id));

        // 清除活跃终端
        registry.clear_active_pane().unwrap();
        assert_eq!(registry.get_active_pane(), None);
        assert!(!registry.is_pane_active(pane_id));
    }

    #[tokio::test]
    async fn test_window_active_pane_management() {
        let registry = ActiveTerminalContextRegistry::new();
        let window_id = WindowId::new(1);
        let pane_id = PaneId::new(1);

        // 初始状态应该没有窗口活跃终端
        assert_eq!(registry.get_window_active_pane(window_id), None);

        // 设置窗口活跃终端
        registry.set_window_active_pane(window_id, pane_id).unwrap();
        assert_eq!(registry.get_window_active_pane(window_id), Some(pane_id));

        // 移除窗口活跃终端
        let removed = registry.remove_window_active_pane(window_id).unwrap();
        assert_eq!(removed, Some(pane_id));
        assert_eq!(registry.get_window_active_pane(window_id), None);
    }

    #[tokio::test]
    async fn test_event_broadcasting() {
        let registry = ActiveTerminalContextRegistry::new();
        let mut receiver = registry.subscribe_events();
        let pane_id = PaneId::new(1);

        // 设置活跃终端应该触发事件
        registry.set_active_pane(pane_id).unwrap();

        // 接收事件
        let event = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("应该在超时前收到事件")
            .expect("应该成功接收事件");

        match event {
            TerminalContextEvent::ActivePaneChanged {
                old_pane_id,
                new_pane_id,
            } => {
                assert_eq!(old_pane_id, None);
                assert_eq!(new_pane_id, Some(pane_id));
            }
            _ => panic!("收到了错误的事件类型"),
        }
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let registry = Arc::new(ActiveTerminalContextRegistry::new());
        let mut handles = Vec::new();

        // 启动多个并发任务
        for i in 0..10 {
            let registry_clone = Arc::clone(&registry);
            let handle = tokio::spawn(async move {
                let pane_id = PaneId::new(i);

                // 设置活跃终端
                registry_clone.set_active_pane(pane_id).unwrap();

                // 检查状态
                let active = registry_clone.get_active_pane();
                let is_active = registry_clone.is_pane_active(pane_id);

                // 清除活跃终端
                registry_clone.clear_active_pane().unwrap();

                (active, is_active)
            });
            handles.push(handle);
        }

        // 等待所有任务完成
        for handle in handles {
            let (_active, _was_active) = handle.await.unwrap();
            // 由于并发执行，我们只能验证基本的一致性
            // 在并发环境下，状态可能会快速变化，所以我们主要验证操作不会崩溃
        }

        // 最终状态应该是没有活跃终端
        assert_eq!(registry.get_active_pane(), None);
    }

    #[test]
    fn test_registry_stats() {
        let registry = ActiveTerminalContextRegistry::new();
        let pane_id = PaneId::new(1);
        let window_id = WindowId::new(1);

        // 初始统计
        let stats = registry.get_stats();
        assert_eq!(stats.global_active_pane, None);
        assert_eq!(stats.window_active_pane_count, 0);

        // 设置活跃终端后的统计
        registry.set_active_pane(pane_id).unwrap();
        registry.set_window_active_pane(window_id, pane_id).unwrap();

        let stats = registry.get_stats();
        assert_eq!(stats.global_active_pane, Some(pane_id));
        assert_eq!(stats.window_active_pane_count, 1);
    }
}
