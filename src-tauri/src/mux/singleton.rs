//! TerminalMux 全局单例管理
//!
//! 确保整个应用只有一个 Mux 实例

use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use crate::mux::{MuxError, MuxResult, TerminalMux};

/// 全局TerminalMux单例实例
static GLOBAL_MUX: OnceLock<Arc<TerminalMux>> = OnceLock::new();

/// 通知处理线程句柄（用于优雅关停时 join）
static NOTIFICATION_THREAD: OnceLock<Mutex<Option<thread::JoinHandle<()>>>> = OnceLock::new();

/// 获取全局TerminalMux实例
///
/// 这个函数是线程安全的，第一次调用时会创建实例，
/// 后续调用会返回同一个实例的引用
///
/// 注意：推荐在应用初始化时使用 init_mux_with_shell_integration 来
/// 指定 ShellIntegrationManager，以确保回调正确注册
pub fn get_mux() -> Arc<TerminalMux> {
    GLOBAL_MUX
        .get_or_init(|| {
            tracing::warn!(
                "使用默认方式初始化全局TerminalMux，建议使用 init_mux_with_shell_integration"
            );
            init_mux_internal(None)
        })
        .clone()
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
    tracing::debug!("初始化全局TerminalMux实例");
    let mux = if let Some(integration) = shell_integration {
        tracing::info!("使用外部 ShellIntegrationManager 创建 TerminalMux");
        Arc::new(TerminalMux::new_with_shell_integration(integration))
    } else {
        Arc::new(TerminalMux::new())
    };

    // 启动通知处理线程
    let mux_clone = Arc::clone(&mux);
    let notification_thread = mux_clone.start_notification_processor();
    // 保存线程句柄，方便关停时 join
    let slot = NOTIFICATION_THREAD.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = slot.lock() {
        *guard = Some(notification_thread);
    } else {
        tracing::warn!("无法保存通知处理线程句柄（锁不可用）");
    }
    tracing::debug!("TerminalMux通知处理线程已启动");

    mux
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
        tracing::debug!("关闭全局TerminalMux实例");
        let result = mux.shutdown().map_err(MuxError::from);
        // 尝试回收通知处理线程
        if let Some(slot) = NOTIFICATION_THREAD.get() {
            if let Ok(mut guard) = slot.lock() {
                if let Some(handle) = guard.take() {
                    let _ = handle.join();
                    tracing::debug!("通知处理线程已回收");
                }
            }
        }
        result
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
