/*!
 * 统一的终端事件处理器
 *
 * 提供单一的事件集成路径，整合所有终端相关事件的处理逻辑，
 * 确保事件的单一来源和清晰的传播路径。
 */

use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::broadcast;
use tracing::{debug, error, warn};

use crate::mux::{MuxNotification, SubscriberCallback, TerminalMux};
use crate::terminal::types::TerminalContextEvent;
use crate::utils::error::AppResult;

/// 统一的终端事件处理器
///
/// 负责整合来自不同源的终端事件，并统一发送到前端
pub struct TerminalEventHandler<R: Runtime> {
    app_handle: AppHandle<R>,
    mux_subscriber_id: Option<usize>,
    context_event_receiver: Option<broadcast::Receiver<TerminalContextEvent>>,
}

/// Type alias for the concrete event handler used in this application
pub type ConcreteTerminalEventHandler = TerminalEventHandler<tauri::Wry>;

// Implement Send and Sync to allow the handler to be managed by Tauri
unsafe impl<R: Runtime> Send for TerminalEventHandler<R> {}
unsafe impl<R: Runtime> Sync for TerminalEventHandler<R> {}

impl<R: Runtime> TerminalEventHandler<R> {
    /// 创建新的终端事件处理器
    pub fn new(app_handle: AppHandle<R>) -> Self {
        Self {
            app_handle,
            mux_subscriber_id: None,
            context_event_receiver: None,
        }
    }

    /// 启动事件处理器
    ///
    /// 订阅来自 TerminalMux 和 TerminalContextRegistry 的事件
    pub fn start(
        &mut self,
        mux: &Arc<TerminalMux>,
        context_event_receiver: broadcast::Receiver<TerminalContextEvent>,
    ) -> AppResult<()> {
        if self.mux_subscriber_id.is_some() {
            anyhow::bail!("事件处理器已经启动");
        }

        // 订阅 TerminalMux 事件
        let app_handle = self.app_handle.clone();
        let mux_subscriber = Self::create_mux_subscriber(app_handle.clone());
        let subscriber_id = mux.subscribe(mux_subscriber);
        self.mux_subscriber_id = Some(subscriber_id);

        // 保存上下文事件接收器
        self.context_event_receiver = Some(context_event_receiver);

        // 启动上下文事件处理任务
        self.start_context_event_task();

        debug!("终端事件处理器已启动，Mux订阅者ID: {}", subscriber_id);
        Ok(())
    }

    /// 停止事件处理器
    pub fn stop(&mut self, mux: &Arc<TerminalMux>) -> AppResult<()> {
        if let Some(subscriber_id) = self.mux_subscriber_id.take() {
            if mux.unsubscribe(subscriber_id) {
                debug!("终端事件处理器已停止，Mux订阅者ID: {}", subscriber_id);
            } else {
                warn!("无法取消Mux订阅者 {}", subscriber_id);
            }
        }

        // 清理上下文事件接收器
        self.context_event_receiver = None;

        Ok(())
    }

    /// 创建 TerminalMux 事件订阅者
    fn create_mux_subscriber(app_handle: AppHandle<R>) -> SubscriberCallback {
        Box::new(move |notification| {
            let (event_name, payload) = Self::mux_notification_to_tauri_event(notification);

            // 添加详细的调试日志
            if let Err(e) = app_handle.emit(event_name, payload.clone()) {
                error!("发送Mux事件失败: {}, 错误: {}, payload: {}", event_name, e, payload);
            }

            true // 继续保持订阅
        })
    }

    /// 启动上下文事件处理任务
    fn start_context_event_task(&mut self) {
        if let Some(mut receiver) = self.context_event_receiver.take() {
            let app_handle = self.app_handle.clone();

            // Use tauri::async_runtime::spawn instead of tokio::spawn to ensure
            // we're using Tauri's async runtime during app initialization
            tauri::async_runtime::spawn(async move {
                while let Ok(event) = receiver.recv().await {
                    Self::handle_context_event(&app_handle, event);
                }
                debug!("上下文事件处理任务已结束");
            });
        }
    }

    /// 处理终端上下文事件
    fn handle_context_event(app_handle: &AppHandle<R>, event: TerminalContextEvent) {
        let (event_name, payload) = Self::context_event_to_tauri_event(&event);

        match app_handle.emit(event_name, payload) {
            Ok(_) => {
                debug!("上下文事件已发送: {}", event_name);
            }
            Err(e) => {
                error!("发送上下文事件失败: {}, 错误: {}", event_name, e);
            }
        }
    }

    /// 将 MuxNotification 转换为 Tauri 事件
    pub fn mux_notification_to_tauri_event(
        notification: &MuxNotification,
    ) -> (&'static str, serde_json::Value) {
        match notification {
            MuxNotification::PaneOutput { pane_id, data } => (
                "terminal_output",
                json!({
                    "paneId": pane_id.as_u32(),
                    "data": String::from_utf8_lossy(data)
                }),
            ),
            MuxNotification::PaneAdded(pane_id) => (
                "terminal_created",
                json!({
                    "paneId": pane_id.as_u32()
                }),
            ),
            MuxNotification::PaneRemoved(pane_id) => (
                "terminal_closed",
                json!({
                    "paneId": pane_id.as_u32()
                }),
            ),
            MuxNotification::PaneResized { pane_id, size } => (
                "terminal_resized",
                json!({
                    "paneId": pane_id.as_u32(),
                    "rows": size.rows,
                    "cols": size.cols
                }),
            ),
            MuxNotification::PaneExited { pane_id, exit_code } => (
                "terminal_exit",
                json!({
                    "paneId": pane_id.as_u32(),
                    "exitCode": exit_code
                }),
            ),
            MuxNotification::PaneCwdChanged { pane_id, cwd } => (
                "pane_cwd_changed",
                json!({
                    "paneId": pane_id.as_u32(),
                    "cwd": cwd
                }),
            ),
        }
    }

    /// 将 TerminalContextEvent 转换为 Tauri 事件
    pub fn context_event_to_tauri_event(
        event: &TerminalContextEvent,
    ) -> (&'static str, serde_json::Value) {
        match event {
            TerminalContextEvent::ActivePaneChanged {
                old_pane_id,
                new_pane_id,
            } => (
                "active_pane_changed",
                json!({
                    "oldPaneId": old_pane_id.map(|id| id.as_u32()),
                    "newPaneId": new_pane_id.map(|id| id.as_u32())
                }),
            ),
            TerminalContextEvent::PaneContextUpdated { pane_id, context } => (
                "pane_context_updated",
                json!({
                    "paneId": pane_id.as_u32(),
                    "context": context
                }),
            ),
            TerminalContextEvent::PaneCwdChanged {
                pane_id,
                old_cwd,
                new_cwd,
            } => (
                "pane_cwd_changed",
                json!({
                    "paneId": pane_id.as_u32(),
                    "oldCwd": old_cwd,
                    "newCwd": new_cwd
                }),
            ),
            TerminalContextEvent::PaneShellIntegrationChanged { pane_id, enabled } => (
                "pane_shell_integration_changed",
                json!({
                    "paneId": pane_id.as_u32(),
                    "enabled": enabled
                }),
            ),
        }
    }

    /// 手动发送事件（用于测试或特殊情况）
    pub fn emit_event(&self, event_name: &str, payload: serde_json::Value) -> AppResult<()> {
        self.app_handle
            .emit(event_name, payload)
            .map_err(|e| anyhow::anyhow!("发送事件失败: {}", e))?;
        Ok(())
    }

    /// 获取当前状态信息
    pub fn get_status(&self) -> EventHandlerStatus {
        EventHandlerStatus {
            mux_subscribed: self.mux_subscriber_id.is_some(),
            context_subscribed: self.context_event_receiver.is_some(),
            mux_subscriber_id: self.mux_subscriber_id,
        }
    }
}

impl<R: Runtime> Drop for TerminalEventHandler<R> {
    fn drop(&mut self) {
        if self.mux_subscriber_id.is_some() {
            warn!("TerminalEventHandler 被丢弃时仍有活跃的Mux订阅");
        }
    }
}

/// 事件处理器状态信息
#[derive(Debug, Clone)]
pub struct EventHandlerStatus {
    pub mux_subscribed: bool,
    pub context_subscribed: bool,
    pub mux_subscriber_id: Option<usize>,
}

/// 便利函数：创建并启动终端事件处理器
pub fn create_terminal_event_handler<R: Runtime>(
    app_handle: AppHandle<R>,
    mux: &Arc<TerminalMux>,
    context_event_receiver: broadcast::Receiver<TerminalContextEvent>,
) -> AppResult<TerminalEventHandler<R>> {
    let mut handler = TerminalEventHandler::new(app_handle);
    handler.start(mux, context_event_receiver)?;
    Ok(handler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mux::PaneId;

    #[test]
    fn test_mux_notification_to_tauri_event() {
        let pane_id = PaneId::new(1);
        let notification = MuxNotification::PaneAdded(pane_id);

        let (event_name, payload) =
            TerminalEventHandler::<tauri::Wry>::mux_notification_to_tauri_event(&notification);

        assert_eq!(event_name, "terminal_created");
        assert_eq!(payload["paneId"], 1);
    }

    #[test]
    fn test_context_event_to_tauri_event() {
        let pane_id = PaneId::new(1);
        let event = TerminalContextEvent::ActivePaneChanged {
            old_pane_id: None,
            new_pane_id: Some(pane_id),
        };

        let (event_name, payload) =
            TerminalEventHandler::<tauri::Wry>::context_event_to_tauri_event(&event);

        assert_eq!(event_name, "active_pane_changed");
        assert_eq!(payload["oldPaneId"], serde_json::Value::Null);
        assert_eq!(payload["newPaneId"], 1);
    }

    #[test]
    fn test_cwd_changed_event_conversion() {
        let pane_id = PaneId::new(1);
        let event = TerminalContextEvent::PaneCwdChanged {
            pane_id,
            old_cwd: Some("/old/path".to_string()),
            new_cwd: "/new/path".to_string(),
        };

        let (event_name, payload) =
            TerminalEventHandler::<tauri::Wry>::context_event_to_tauri_event(&event);

        assert_eq!(event_name, "pane_cwd_changed");
        assert_eq!(payload["paneId"], 1);
        assert_eq!(payload["oldCwd"], "/old/path");
        assert_eq!(payload["newCwd"], "/new/path");
    }

    #[test]
    fn test_shell_integration_changed_event_conversion() {
        let pane_id = PaneId::new(1);
        let event = TerminalContextEvent::PaneShellIntegrationChanged {
            pane_id,
            enabled: true,
        };

        let (event_name, payload) =
            TerminalEventHandler::<tauri::Wry>::context_event_to_tauri_event(&event);

        assert_eq!(event_name, "pane_shell_integration_changed");
        assert_eq!(payload["paneId"], 1);
        assert_eq!(payload["enabled"], true);
    }

    // Note: Event handler status test requires a real Tauri app context
    // which is not easily mockable in unit tests. Integration tests would be more appropriate.
}
