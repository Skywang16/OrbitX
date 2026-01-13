//! TerminalMux - 核心终端多路复用器
//!
//! 提供统一的终端会话管理、事件通知和PTY I/O处理

use crossbeam_channel::{unbounded, Sender};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use tracing::{error, instrument, warn};

use crate::mux::{
    error::{TerminalMuxError, TerminalMuxResult},
    IoHandler, LocalPane, MuxNotification, Pane, PaneId, PtySize, TerminalConfig,
};
use crate::shell::ShellIntegrationManager;

pub type SubscriberCallback = Box<dyn Fn(&MuxNotification) -> bool + Send + Sync>;

#[derive(Debug, Clone)]
pub struct TerminalMuxStatus {
    pub pane_count: usize,
    pub subscriber_count: usize,
    pub next_pane_id: u32,
    pub next_subscriber_id: u32,
}

pub struct TerminalMux {
    panes: RwLock<HashMap<PaneId, Arc<dyn Pane>>>,

    /// 事件订阅者 - 订阅ID -> 回调函数
    subscribers: RwLock<HashMap<usize, SubscriberCallback>>,

    /// 下一个面板ID生成器
    next_pane_id: AtomicU32,

    /// 下一个订阅者ID生成器
    next_subscriber_id: AtomicU32,

    /// 跨线程通知发送器
    notification_sender: Sender<MuxNotification>,

    /// 通知处理线程句柄（如果启用自动处理模式）
    notification_thread: Mutex<Option<thread::JoinHandle<()>>>,

    /// I/O 处理器
    io_handler: IoHandler,

    /// Shell Integration管理器
    shell_integration: Arc<ShellIntegrationManager>,

    /// 是否正在关闭（用于通知处理线程优雅退出）
    shutting_down: std::sync::atomic::AtomicBool,
}

impl TerminalMux {
    /// 创建并启动通知线程的共享实例（推荐）
    pub fn new_shared() -> Arc<Self> {
        Self::new_shared_with_shell_integration(Arc::new(ShellIntegrationManager::new()))
    }

    /// 创建并启动通知线程的共享实例（允许注入 ShellIntegrationManager）
    pub fn new_shared_with_shell_integration(
        shell_integration: Arc<ShellIntegrationManager>,
    ) -> Arc<Self> {
        let (notification_sender, notification_receiver) = unbounded();
        let io_handler = IoHandler::new(notification_sender.clone(), shell_integration.clone());

        let mux = Arc::new(Self {
            panes: RwLock::new(HashMap::new()),
            subscribers: RwLock::new(HashMap::new()),
            next_pane_id: AtomicU32::new(1),
            next_subscriber_id: AtomicU32::new(1),
            notification_sender,
            notification_thread: Mutex::new(None),
            io_handler,
            shell_integration,
            shutting_down: std::sync::atomic::AtomicBool::new(false),
        });

        // 通知线程拥有 receiver；TerminalMux 只保留 sender，避免“Option receiver”这种补丁式状态机。
        let mux_for_thread = Arc::clone(&mux);
        let handle = thread::spawn(move || loop {
            if mux_for_thread
                .shutting_down
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                break;
            }

            match notification_receiver.recv_timeout(Duration::from_millis(20)) {
                Ok(notification) => mux_for_thread.notify_internal(&notification),
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }
        });

        {
            let mut guard = mux
                .notification_thread
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *guard = Some(handle);
        }

        mux
    }

    pub fn shell_integration(&self) -> &Arc<ShellIntegrationManager> {
        &self.shell_integration
    }

    pub fn get_status(&self) -> TerminalMuxResult<TerminalMuxStatus> {
        let panes = self
            .panes
            .read()
            .map_err(|err| TerminalMuxError::from_read_poison("panes", err))?;
        let subscribers = self
            .subscribers
            .read()
            .map_err(|err| TerminalMuxError::from_read_poison("subscribers", err))?;

        let status = TerminalMuxStatus {
            pane_count: panes.len(),
            subscriber_count: subscribers.len(),
            next_pane_id: self.next_pane_id.load(Ordering::Relaxed),
            next_subscriber_id: self.next_subscriber_id.load(Ordering::Relaxed),
        };

        Ok(status)
    }

    /// 生成下一个唯一的面板ID
    fn next_pane_id(&self) -> PaneId {
        let id = self.next_pane_id.fetch_add(1, Ordering::Relaxed);
        PaneId::new(id)
    }

    /// 生成下一个唯一的订阅者ID
    fn next_subscriber_id(&self) -> usize {
        self.next_subscriber_id.fetch_add(1, Ordering::Relaxed) as usize
    }

    /// 创建新面板
    pub async fn create_pane(&self, size: PtySize) -> TerminalMuxResult<PaneId> {
        let config = TerminalConfig::default();
        self.create_pane_with_config(size, &config).await
    }

    /// 使用指定配置创建新面板
    ///
    /// - 使用结构化日志格式
    /// - 包含性能指标
    #[instrument(skip(self, config), fields(pane_id, shell = %config.shell_config.shell_info.display_name))]
    pub async fn create_pane_with_config(
        &self,
        size: PtySize,
        config: &TerminalConfig,
    ) -> TerminalMuxResult<PaneId> {
        let pane_id = self.next_pane_id();
        let pane = Arc::new(LocalPane::new_with_config(pane_id, size, config)?);

        // 添加到面板映射
        {
            let mut panes = self
                .panes
                .write()
                .map_err(|err| TerminalMuxError::from_write_poison("panes", err))?;

            if panes.contains_key(&pane_id) {
                return Err(TerminalMuxError::PaneExists { pane_id });
            }
            panes.insert(pane_id, pane.clone());
        }

        // 设置面板的Shell类型到shell_integration
        let shell_type =
            crate::shell::ShellType::from_program(&config.shell_config.shell_info.path);
        self.shell_integration
            .set_pane_shell_type(pane_id, shell_type.clone());

        // 启动I/O处理线程
        self.io_handler.spawn_io_threads(pane.clone())?;

        // 发送面板添加通知
        self.notify(MuxNotification::PaneAdded(pane_id));
        Ok(pane_id)
    }

    /// 获取面板引用
    pub fn get_pane(&self, pane_id: PaneId) -> Option<Arc<dyn Pane>> {
        let panes = self.panes.read().ok()?;
        panes.get(&pane_id).cloned()
    }

    /// 检查面板是否存在
    pub fn pane_exists(&self, pane_id: PaneId) -> bool {
        self.panes
            .read()
            .map(|panes| panes.contains_key(&pane_id))
            .unwrap_or(false)
    }

    /// 移除面板
    ///
    /// - 使用结构化日志格式
    /// - 包含性能指标
    #[instrument(skip(self), fields(pane_id = ?pane_id))]
    pub fn remove_pane(&self, pane_id: PaneId) -> TerminalMuxResult<()> {
        let pane = {
            let mut panes = self
                .panes
                .write()
                .map_err(|err| TerminalMuxError::from_write_poison("panes", err))?;

            panes
                .remove(&pane_id)
                .ok_or_else(|| TerminalMuxError::PaneNotFound { pane_id })?
        };

        // 标记面板为死亡状态，停止I/O线程
        pane.mark_dead();

        // 停止I/O处理
        if let Err(e) = self.io_handler.stop_pane_io(pane_id) {
            warn!("停止面板 {:?} I/O处理失败: {}", pane_id, e);
        }

        // 发送面板移除通知
        self.notify(MuxNotification::PaneRemoved(pane_id));
        Ok(())
    }

    /// 获取所有面板ID列表
    pub fn list_panes(&self) -> Vec<PaneId> {
        self.panes
            .read()
            .map(|panes| panes.keys().copied().collect())
            .unwrap_or_default()
    }

    /// 获取面板数量
    pub fn pane_count(&self) -> usize {
        self.panes.read().map(|panes| panes.len()).unwrap_or(0)
    }

    /// 写入数据到指定面板
    ///
    /// - 使用结构化日志格式
    /// - 包含性能指标
    #[instrument(skip(self, data), fields(pane_id = ?pane_id, data_len = data.len()), level = "trace")]
    pub fn write_to_pane(&self, pane_id: PaneId, data: &[u8]) -> TerminalMuxResult<()> {
        let pane = self
            .get_pane(pane_id)
            .ok_or_else(|| TerminalMuxError::PaneNotFound { pane_id })?;

        pane.write(data)?;
        Ok(())
    }

    /// 调整面板大小
    ///
    /// - 使用结构化日志格式
    /// - 包含性能指标
    #[instrument(skip(self), fields(pane_id = ?pane_id, size = ?size))]
    pub fn resize_pane(&self, pane_id: PaneId, size: PtySize) -> TerminalMuxResult<()> {
        let pane = self
            .get_pane(pane_id)
            .ok_or_else(|| TerminalMuxError::PaneNotFound { pane_id })?;

        pane.resize(size)?;

        // 发送大小调整通知
        self.notify(MuxNotification::PaneResized { pane_id, size });
        Ok(())
    }

    /// 订阅事件通知
    pub fn subscribe<F>(&self, subscriber: F) -> usize
    where
        F: Fn(&MuxNotification) -> bool + Send + Sync + 'static,
    {
        let subscriber_id = self.next_subscriber_id();

        if let Ok(mut subscribers) = self.subscribers.write() {
            subscribers.insert(subscriber_id, Box::new(subscriber));
        } else {
            error!("无法获取订阅者写锁");
        }

        subscriber_id
    }

    /// 取消订阅
    pub fn unsubscribe(&self, subscriber_id: usize) -> bool {
        if let Ok(mut subscribers) = self.subscribers.write() {
            subscribers.remove(&subscriber_id).is_some()
        } else {
            error!("无法获取订阅者写锁");
            false
        }
    }

    /// 发送通知给所有订阅者
    pub fn notify(&self, notification: MuxNotification) {
        if let Err(e) = self.notification_sender.send(notification) {
            error!("跨线程通知发送失败: {}", e);
        }
    }

    /// 内部通知实现（通知线程内串行执行）
    fn notify_internal(&self, notification: &MuxNotification) {
        let mut dead_subscribers = Vec::new();

        if let Ok(subscribers) = self.subscribers.read() {
            for (&subscriber_id, callback) in subscribers.iter() {
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    callback(notification)
                })) {
                    Ok(true) => {
                        // 订阅者处理成功，继续保持订阅
                    }
                    Ok(false) => {
                        // 订阅者返回false，标记为需要移除
                        dead_subscribers.push(subscriber_id);
                    }
                    Err(_) => {
                        // 订阅者回调panic，标记为需要移除
                        error!("订阅者 {} 回调panic", subscriber_id);
                        dead_subscribers.push(subscriber_id);
                    }
                }
            }
        }

        // 清理无效订阅者
        if !dead_subscribers.is_empty() {
            if let Ok(mut subscribers) = self.subscribers.write() {
                for subscriber_id in dead_subscribers {
                    subscribers.remove(&subscriber_id);
                }
            }
        }
    }

    /// 创建调试订阅者（用于测试和调试）
    pub fn create_debug_subscriber() -> impl Fn(&MuxNotification) -> bool + Send + Sync + 'static {
        move |notification: &MuxNotification| {
            tracing::debug!("MuxNotification: {:?}", notification);
            true
        }
    }

    /// 设置面板的Shell Integration
    pub fn setup_pane_integration(&self, pane_id: PaneId) -> TerminalMuxResult<()> {
        self.shell_integration.enable_integration(pane_id);
        Ok(())
    }

    /// 设置面板的Shell Integration并注入脚本
    pub fn setup_pane_integration_with_script(
        &self,
        pane_id: PaneId,
        silent: bool,
    ) -> TerminalMuxResult<()> {
        use crate::shell::ShellType;

        // 启用Shell Integration
        self.shell_integration.enable_integration(pane_id);

        // 从shell_integration获取已设置的Shell类型
        let panes = self
            .panes
            .read()
            .map_err(|err| TerminalMuxError::from_read_poison("panes", err))?;

        if panes.get(&pane_id).is_none() {
            return Err(TerminalMuxError::PaneNotFound { pane_id });
        }

        let shell_type = self
            .shell_integration
            .with_pane_state(pane_id, |state| state.shell_type.clone())
            .flatten()
            .unwrap_or_else(|| {
                warn!("面板 {:?} 没有设置Shell类型，使用默认Bash", pane_id);
                ShellType::Bash
            });

        if !silent {
            let script = self
                .shell_integration
                .generate_shell_script(&shell_type)
                .map_err(|err| {
                    TerminalMuxError::Internal(format!("Shell integration error: {}", err))
                })?;
            self.write_to_pane(pane_id, script.as_bytes())?;
        }

        Ok(())
    }

    /// 检查面板是否已集成Shell Integration
    pub fn is_pane_integrated(&self, pane_id: PaneId) -> bool {
        self.shell_integration.get_pane_state(pane_id).is_some()
    }

    /// 获取面板的当前工作目录
    pub fn shell_get_pane_cwd(&self, pane_id: PaneId) -> Option<String> {
        self.shell_integration
            .get_current_working_directory(pane_id)
    }

    /// 更新面板的当前工作目录
    pub fn shell_update_pane_cwd(&self, pane_id: PaneId, cwd: String) {
        self.shell_integration
            .update_current_working_directory(pane_id, cwd);
    }

    pub fn get_pane_shell_state(&self, pane_id: PaneId) -> Option<crate::shell::PaneShellState> {
        self.shell_integration.get_pane_shell_state(pane_id)
    }

    pub fn set_pane_shell_type(&self, pane_id: PaneId, shell_type: crate::shell::ShellType) {
        self.shell_integration
            .set_pane_shell_type(pane_id, shell_type);
    }

    /// 生成Shell集成脚本
    pub fn generate_shell_integration_script(
        &self,
        shell_type: &crate::shell::ShellType,
    ) -> TerminalMuxResult<String> {
        self.shell_integration
            .generate_shell_script(shell_type)
            .map_err(|err| TerminalMuxError::Internal(format!("Shell integration error: {}", err)))
    }

    /// 生成Shell环境变量
    pub fn generate_shell_env_vars(
        &self,
        shell_type: &crate::shell::ShellType,
    ) -> std::collections::HashMap<String, String> {
        self.shell_integration.generate_shell_env_vars(shell_type)
    }

    /// 启用面板Shell Integration
    pub fn enable_pane_integration(&self, pane_id: PaneId) {
        self.shell_integration.enable_integration(pane_id);
    }

    /// 禁用面板Shell Integration
    pub fn disable_pane_integration(&self, pane_id: PaneId) {
        self.shell_integration.disable_integration(pane_id);
    }

    pub fn get_pane_current_command(
        &self,
        pane_id: PaneId,
    ) -> Option<std::sync::Arc<crate::shell::CommandInfo>> {
        self.shell_integration.get_current_command(pane_id)
    }

    pub fn get_pane_command_history(
        &self,
        pane_id: PaneId,
    ) -> Vec<std::sync::Arc<crate::shell::CommandInfo>> {
        self.shell_integration.get_command_history(pane_id)
    }

    /// 清理所有资源
    pub fn shutdown(&self) -> TerminalMuxResult<()> {
        let shutdown_start = std::time::Instant::now();

        // 标记为关闭状态，使通知处理线程能尽快退出
        self.shutting_down
            .store(true, std::sync::atomic::Ordering::Relaxed);

        if let Ok(mut guard) = self.notification_thread.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
            }
        }

        let pane_ids: Vec<PaneId> = self.list_panes();

        // 立即标记所有面板为死亡状态，加速关闭过程
        {
            let panes = self
                .panes
                .read()
                .map_err(|err| TerminalMuxError::from_read_poison("panes", err))?;
            for (_pane_id, pane) in panes.iter() {
                pane.mark_dead();
            }
        }

        // 逐个关闭面板
        let mut failed_panes = Vec::new();
        for pane_id in pane_ids {
            match self.remove_pane(pane_id) {
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("关闭面板 {:?} 失败: {}", pane_id, e);
                    failed_panes.push(pane_id);
                }
            }

            if shutdown_start.elapsed() > Duration::from_secs(3) {
                tracing::warn!("关闭超时，强制退出剩余面板");
                break;
            }
        }

        if !failed_panes.is_empty() {
            tracing::warn!(
                "有 {} 个面板关闭失败: {:?}",
                failed_panes.len(),
                failed_panes
            );
        }

        // 清理所有订阅者
        if let Ok(mut subscribers) = self.subscribers.write() {
            subscribers.clear();
        }

        // 关闭I/O处理器
        let _ = self.io_handler.shutdown();

        Ok(())
    }
}

// 实现Debug trait用于调试
impl std::fmt::Debug for TerminalMux {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalMux")
            .field("pane_count", &self.pane_count())
            .field("next_pane_id", &self.next_pane_id.load(Ordering::Relaxed))
            .field(
                "next_subscriber_id",
                &self.next_subscriber_id.load(Ordering::Relaxed),
            )
            .finish()
    }
}

// 线程安全标记
// 依赖成员类型的 Send/Sync 自动推导即可，无需显式 unsafe 标记
