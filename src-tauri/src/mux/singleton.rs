//! TerminalMux 全局单例管理
//!
//! 确保整个应用只有一个 Mux 实例

use std::sync::{Arc, OnceLock};

use crate::mux::{MuxError, MuxResult, TerminalMux};

/// 全局TerminalMux单例实例
static GLOBAL_MUX: OnceLock<Arc<TerminalMux>> = OnceLock::new();

/// 获取全局TerminalMux实例
///
/// 这个函数是线程安全的，第一次调用时会创建实例，
/// 后续调用会返回同一个实例的引用
///
/// 注意：推荐在应用初始化时使用 init_mux_with_shell_integration 来
/// 指定 ShellIntegrationManager，以确保回调正确注册
pub fn get_mux() -> Arc<TerminalMux> {
    GLOBAL_MUX.get_or_init(|| init_mux_internal(None)).clone()
}

/// 使用指定的 ShellIntegrationManager 初始化全局 TerminalMux
///
/// 这个函数应该在应用启动时调用一次，确保 Mux 使用正确的 ShellIntegrationManager
/// 如果已经初始化过，这个函数会返回错误
pub fn init_mux_with_shell_integration(
    shell_integration: std::sync::Arc<crate::shell::ShellIntegrationManager>,
) -> Result<Arc<TerminalMux>, &'static str> {
    GLOBAL_MUX
        .set(init_mux_internal(Some(shell_integration)))
        .map_err(|_| "TerminalMux已经初始化")?;
    Ok(GLOBAL_MUX.get().unwrap().clone())
}

fn init_mux_internal(
    shell_integration: Option<std::sync::Arc<crate::shell::ShellIntegrationManager>>,
) -> Arc<TerminalMux> {
    if let Some(integration) = shell_integration {
        TerminalMux::new_shared_with_shell_integration(integration)
    } else {
        TerminalMux::new_shared()
    }
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
pub fn shutdown_mux() -> MuxResult<()> {
    if let Some(mux) = GLOBAL_MUX.get() {
        mux.shutdown().map_err(MuxError::from)
    } else {
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
        mux.notify(notification);
    }
}

/// Mux统计信息
#[derive(Debug, Clone)]
pub struct MuxStats {
    pub pane_count: usize,
    pub is_initialized: bool,
}
