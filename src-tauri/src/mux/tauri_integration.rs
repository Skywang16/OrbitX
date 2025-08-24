//! Tauri 事件系统集成
//!
//! 提供 TerminalMux 与 Tauri 前端事件系统的集成

use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};

use crate::mux::{MuxNotification, SubscriberCallback, TerminalMux};
use crate::utils::error::AppResult;

/// Tauri 事件集成器
pub struct TauriEventIntegrator<R: Runtime> {
    app_handle: AppHandle<R>,
    subscriber_id: Option<usize>,
}

impl<R: Runtime> TauriEventIntegrator<R> {
    /// 创建新的 Tauri 事件集成器
    pub fn new(app_handle: AppHandle<R>) -> Self {
        Self {
            app_handle,
            subscriber_id: None,
        }
    }

    /// 启动事件集成
    pub fn start_integration(&mut self, mux: &Arc<TerminalMux>) -> AppResult<()> {
        if self.subscriber_id.is_some() {
            anyhow::bail!("事件集成已经启动");
        }

        let app_handle = self.app_handle.clone();
        let subscriber = Self::create_tauri_subscriber(app_handle);
        let subscriber_id = mux.subscribe(subscriber);

        self.subscriber_id = Some(subscriber_id);
        tracing::debug!("Tauri 事件集成已启动，订阅者 ID: {}", subscriber_id);

        Ok(())
    }

    /// 停止事件集成
    pub fn stop_integration(&mut self, mux: &Arc<TerminalMux>) -> AppResult<()> {
        if let Some(subscriber_id) = self.subscriber_id.take() {
            if mux.unsubscribe(subscriber_id) {
                tracing::debug!("Tauri 事件集成已停止，订阅者 ID: {}", subscriber_id);
                Ok(())
            } else {
                anyhow::bail!("无法取消订阅者 {}", subscriber_id);
            }
        } else {
            anyhow::bail!("事件集成未启动");
        }
    }

    /// 创建 Tauri 事件订阅者
    fn create_tauri_subscriber(app_handle: AppHandle<R>) -> SubscriberCallback {
        Box::new(move |notification| {
            let (event_name, payload) = Self::notification_to_tauri_payload(notification);

            match app_handle.emit(event_name, payload) {
                Ok(_) => {
                    tracing::debug!("Tauri 事件已发送: {}", event_name);
                    true // 继续保持订阅
                }
                Err(e) => {
                    tracing::error!("发送 Tauri 事件失败: {}", e);
                    // 即使发送失败，也继续保持订阅，可能是临时网络问题
                    true
                }
            }
        })
    }

    /// 将 MuxNotification 转换为 Tauri 事件负载
    fn notification_to_tauri_payload(
        notification: &MuxNotification,
    ) -> (&'static str, serde_json::Value) {
        match notification {
            MuxNotification::PaneOutput { pane_id, data } => (
                "terminal_output",
                json!({
                    "pane_id": pane_id.as_u32(),
                    "data": String::from_utf8_lossy(data)
                }),
            ),
            MuxNotification::PaneAdded(pane_id) => (
                "terminal_created",
                json!({
                    "pane_id": pane_id.as_u32()
                }),
            ),
            MuxNotification::PaneRemoved(pane_id) => (
                "terminal_closed",
                json!({
                    "pane_id": pane_id.as_u32()
                }),
            ),
            MuxNotification::PaneResized { pane_id, size } => (
                "terminal_resized",
                json!({
                    "pane_id": pane_id.as_u32(),
                    "rows": size.rows,
                    "cols": size.cols
                }),
            ),
            MuxNotification::PaneExited { pane_id, exit_code } => (
                "terminal_exit",
                json!({
                    "pane_id": pane_id.as_u32(),
                    "exit_code": exit_code
                }),
            ),
            MuxNotification::PaneCwdChanged { pane_id, cwd } => (
                "pane_cwd_changed",
                json!({
                    "pane_id": pane_id.as_u32(),
                    "cwd": cwd
                }),
            ),
        }
    }
}

impl<R: Runtime> Drop for TauriEventIntegrator<R> {
    fn drop(&mut self) {
        if self.subscriber_id.is_some() {
            tracing::warn!("TauriEventIntegrator 被丢弃时仍有活跃订阅");
        }
    }
}

/// 便利函数：为 TerminalMux 启用 Tauri 事件集成
pub fn enable_tauri_integration<R: Runtime>(
    mux: &Arc<TerminalMux>,
    app_handle: AppHandle<R>,
) -> AppResult<TauriEventIntegrator<R>> {
    let mut integrator = TauriEventIntegrator::new(app_handle);
    integrator.start_integration(mux)?;
    Ok(integrator)
}
