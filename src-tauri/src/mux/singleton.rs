//! TerminalMux 全局单例管理
//!
//! 确保整个应用只有一个 Mux 实例

use std::sync::{Arc, OnceLock};

use crate::mux::TerminalMux;
use crate::utils::error::AppResult;

/// 全局TerminalMux单例实例
static GLOBAL_MUX: OnceLock<Arc<TerminalMux>> = OnceLock::new();

/// 获取全局TerminalMux实例
///
/// 这个函数是线程安全的，第一次调用时会创建实例，
/// 后续调用会返回同一个实例的引用
pub fn get_mux() -> Arc<TerminalMux> {
    GLOBAL_MUX
        .get_or_init(|| {
            tracing::debug!("初始化全局TerminalMux实例");
            Arc::new(TerminalMux::new())
        })
        .clone()
}

/// 初始化全局TerminalMux实例
///
/// 这个函数可以在应用启动时显式调用，确保Mux在需要时已经初始化
/// 如果已经初始化过，这个函数不会有任何效果
pub fn init_mux() -> Arc<TerminalMux> {
    get_mux()
}

/// 关闭全局TerminalMux实例
///
/// 注意：这个函数只能在应用关闭时调用一次
/// 调用后，get_mux()仍然会返回已关闭的实例
pub fn shutdown_mux() -> AppResult<()> {
    if let Some(mux) = GLOBAL_MUX.get() {
        tracing::debug!("关闭全局TerminalMux实例");
        mux.shutdown()
    } else {
        tracing::warn!("尝试关闭未初始化的TerminalMux");
        Ok(())
    }
}

/// 检查全局Mux是否已经初始化
pub fn is_mux_initialized() -> bool {
    GLOBAL_MUX.get().is_some()
}

/// 获取全局Mux的统计信息（用于调试）
pub fn get_mux_stats() -> Option<MuxStats> {
    GLOBAL_MUX.get().map(|mux| MuxStats {
        pane_count: mux.pane_count(),
        is_initialized: true,
    })
}

/// 从任意线程发送通知到全局Mux
///
/// 这个函数可以从任何线程安全调用，通知会被发送到主线程处理
pub fn notify_global(notification: crate::mux::MuxNotification) {
    if let Some(mux) = GLOBAL_MUX.get() {
        mux.notify_from_any_thread(notification);
    } else {
        tracing::warn!("尝试向未初始化的全局Mux发送通知: {:?}", notification);
    }
}

/// Mux统计信息
#[derive(Debug, Clone)]
pub struct MuxStats {
    pub pane_count: usize,
    pub is_initialized: bool,
}
