//! 终端事件处理器

use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime};
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

        // 订阅 TerminalMux 事件（对 PaneOutput 采用缓冲节流，其它事件即时发送）
        let app_handle = self.app_handle.clone();
        let mux_subscriber: SubscriberCallback = Box::new(move |notification| match notification {
            MuxNotification::PaneOutput { pane_id, data } => {
                // 使用 ChannelManager 直接发送字节流，避免在后端进行任何字符串解码
                let state =
                    app_handle.state::<crate::terminal::channel_state::TerminalChannelState>();
                state.manager.send_data(pane_id.as_u32(), data.as_ref());
                true
            }
            MuxNotification::PaneRemoved(pane_id) => {
                // 通知 Channel 已关闭
                let state =
                    app_handle.state::<crate::terminal::channel_state::TerminalChannelState>();
                state.manager.close(pane_id.as_u32());
                let (event_name, payload) = Self::mux_notification_to_tauri_event(notification);
                if let Err(e) = app_handle.emit(event_name, payload.clone()) {
                    error!(
                        "发送Mux事件失败: {}, 错误: {}, payload: {}",
                        event_name, e, payload
                    );
                }
                true
            }
            _ => {
                let (event_name, payload) = Self::mux_notification_to_tauri_event(notification);
                if let Err(e) = app_handle.emit(event_name, payload.clone()) {
                    error!(
                        "发送Mux事件失败: {}, 错误: {}, payload: {}",
                        event_name, e, payload
                    );
                }
                true
            }
        });
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

    // 旧的字符串缓冲刷新任务已移除，改为通过 Channel 直接推送字节流

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
        // 避免与 Mux 事件造成的重复：不再转发上下文层面的 pane_cwd_changed 到前端
        if let TerminalContextEvent::PaneCwdChanged { .. } = &event {
            debug!("忽略上下文层面的 pane_cwd_changed（以 Mux 事件为唯一来源）");
            return;
        }
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
            TerminalContextEvent::PaneShellIntegrationChanged { pane_id, enabled } => (
                "pane_shell_integration_changed",
                json!({
                    "paneId": pane_id.as_u32(),
                    "enabled": enabled
                }),
            ),
            // Note: PaneCwdChanged 事件不应从 Context 层发送给前端，Mux 是唯一来源
            TerminalContextEvent::PaneCwdChanged { .. } => unreachable!(
                "PaneCwdChanged should never be serialized from context_event_to_tauri_event; Mux is the single source"
            ),
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
        // 不再允许从 Context 层序列化 PaneCwdChanged 事件，应该为不可达
        let result = std::panic::catch_unwind(|| {
            let _ = TerminalEventHandler::<tauri::Wry>::context_event_to_tauri_event(&event);
        });
        assert!(result.is_err());
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
