//! TerminalMux - 核心终端多路复用器
//!
//! 提供统一的终端会话管理、事件通知和PTY I/O处理

use anyhow::{anyhow, bail, Context};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, instrument, trace, warn};

use crate::mux::{
    IoHandler, IoThreadPoolStats, LocalPane, MuxNotification, Pane, PaneId, PtySize, TerminalConfig,
};
use crate::shell::ShellIntegrationManager;
use crate::utils::error::AppResult;

/// 订阅者回调函数类型
pub type SubscriberCallback = Box<dyn Fn(&MuxNotification) -> bool + Send + Sync>;

/// TerminalMux状态信息
#[derive(Debug, Clone)]
pub struct TerminalMuxStatus {
    /// 当前面板数量
    pub pane_count: usize,
    /// 当前订阅者数量
    pub subscriber_count: usize,
    /// 下一个面板ID
    pub next_pane_id: u32,
    /// 下一个订阅者ID
    pub next_subscriber_id: u32,
    /// 主线程ID
    pub main_thread_id: thread::ThreadId,
}

/// TerminalMux - 核心终端多路复用器
pub struct TerminalMux {
    /// 面板存储 - 使用RwLock支持并发读取
    panes: RwLock<HashMap<PaneId, Arc<dyn Pane>>>,

    /// 事件订阅者 - 订阅ID -> 回调函数
    subscribers: RwLock<HashMap<usize, SubscriberCallback>>,

    /// 主线程ID - 用于线程安全检查
    main_thread_id: thread::ThreadId,

    /// 下一个面板ID生成器
    next_pane_id: AtomicU32,

    /// 下一个订阅者ID生成器
    next_subscriber_id: AtomicU32,

    /// 跨线程通知发送器
    notification_sender: Sender<MuxNotification>,

    /// 跨线程通知接收器（仅在主线程使用）
    notification_receiver: Arc<RwLock<Option<Receiver<MuxNotification>>>>,

    /// I/O 处理器
    io_handler: IoHandler,

    /// Shell Integration管理器
    shell_integration: Arc<ShellIntegrationManager>,

    /// 是否正在关闭（用于通知处理线程优雅退出）
    shutting_down: std::sync::atomic::AtomicBool,
}

impl TerminalMux {
    /// 创建新的TerminalMux实例
    ///
    /// 统一状态管理规范：
    /// - 按依赖顺序初始化各组件
    /// - 验证配置和依赖关系
    /// - 提供详细的错误信息
    pub fn new() -> Self {
        let (notification_sender, notification_receiver) = unbounded();
        debug!("创建通知通道成功");

        let shell_integration =
            Arc::new(ShellIntegrationManager::new().expect("创建Shell Integration管理器失败"));
        let io_handler = IoHandler::new(notification_sender.clone(), shell_integration.clone());

        let notification_sender_clone = notification_sender.clone();
        shell_integration.register_cwd_callback(move |pane_id, new_cwd| {
            let notification = MuxNotification::PaneCwdChanged {
                pane_id,
                cwd: new_cwd.to_string(),
            };
            let _ = notification_sender_clone.send(notification);
        });

        Self {
            panes: RwLock::new(HashMap::new()),
            subscribers: RwLock::new(HashMap::new()),
            main_thread_id: thread::current().id(),
            next_pane_id: AtomicU32::new(1),
            next_subscriber_id: AtomicU32::new(1),
            notification_sender,
            notification_receiver: Arc::new(RwLock::new(Some(notification_receiver))),
            io_handler,
            shell_integration,
            shutting_down: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// 统一的初始化方法
    ///
    /// 提供更高级的初始化，包括验证和错误处理
    pub fn new_with_validation() -> AppResult<Self> {
        let mux = Self::new();

        // 验证状态完整性
        mux.validate()?;

        debug!("带验证的TerminalMux初始化完成");
        Ok(mux)
    }

    /// 验证状态完整性
    ///
    /// 统一验证规范：
    /// - 验证各组件是否可访问
    /// - 检查内部状态一致性
    /// - 提供详细的错误信息
    pub fn validate(&self) -> AppResult<()> {
        // 验证面板映射是否可访问
        let _unused = self.panes.read().map_err(|_| anyhow!("无法获取面板读锁"))?;
        debug!("面板映射验证通过");

        // 验证订阅者映射是否可访问
        let _unused = self
            .subscribers
            .read()
            .map_err(|_| anyhow!("无法获取订阅者读锁"))?;
        debug!("订阅者映射验证通过");

        // 验证通知接收器是否可访问
        let _unused = self
            .notification_receiver
            .read()
            .map_err(|_| anyhow!("无法获取通知接收器读锁"))?;
        debug!("通知接收器验证通过");

        // 验证原子计数器状态
        let pane_id_counter = self.next_pane_id.load(Ordering::Relaxed);
        let subscriber_id_counter = self.next_subscriber_id.load(Ordering::Relaxed);

        if pane_id_counter == 0 {
            bail!("面板ID计数器状态异常");
        }

        if subscriber_id_counter == 0 {
            bail!("订阅者ID计数器状态异常");
        }

        debug!(
            "计数器状态验证通过: pane_id_counter={}, subscriber_id_counter={}",
            pane_id_counter, subscriber_id_counter
        );

        debug!("TerminalMux状态验证完成");
        Ok(())
    }

    /// 获取状态统计信息
    ///
    /// 提供详细的状态信息用于监控和调试
    pub fn get_status(&self) -> AppResult<TerminalMuxStatus> {
        let panes = self.panes.read().map_err(|_| anyhow!("无法获取面板读锁"))?;
        let subscribers = self
            .subscribers
            .read()
            .map_err(|_| anyhow!("无法获取订阅者读锁"))?;

        let status = TerminalMuxStatus {
            pane_count: panes.len(),
            subscriber_count: subscribers.len(),
            next_pane_id: self.next_pane_id.load(Ordering::Relaxed),
            next_subscriber_id: self.next_subscriber_id.load(Ordering::Relaxed),
            main_thread_id: self.main_thread_id,
        };

        debug!("获取状态信息: {:?}", status);
        Ok(status)
    }

    /// 获取I/O处理统计信息
    pub fn get_io_stats(&self) -> Option<IoThreadPoolStats> {
        self.io_handler.get_stats()
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

    // === 面板管理方法 ===

    /// 创建新面板
    pub async fn create_pane(&self, size: PtySize) -> AppResult<PaneId> {
        let config = TerminalConfig::default();
        self.create_pane_with_config(size, &config).await
    }

    /// 使用指定配置创建新面板
    ///
    /// 统一日志记录规范：
    /// - 记录操作开始、关键步骤和完成时间
    /// - 使用结构化日志格式
    /// - 包含性能指标
    #[instrument(skip(self, config), fields(pane_id, shell = %config.shell_config.program))]
    pub async fn create_pane_with_config(
        &self,
        size: PtySize,
        config: &TerminalConfig,
    ) -> AppResult<PaneId> {
        let pane_id = self.next_pane_id();

        // 创建面板实例
        debug!("创建LocalPane实例: pane_id={:?}", pane_id);
        let pane = Arc::new(
            LocalPane::new_with_config(pane_id, size, config)
                .with_context(|| format!("创建LocalPane失败: pane_id={:?}", pane_id))?,
        );

        // 添加到面板映射
        debug!("添加面板到映射: pane_id={:?}", pane_id);
        {
            let mut panes = self.panes.write().map_err(|_| {
                error!("无法获取面板写锁: pane_id={:?}", pane_id);
                anyhow!("无法获取面板写锁")
            })?;

            if panes.contains_key(&pane_id) {
                error!("面板ID已存在: pane_id={:?}", pane_id);
                bail!("面板 {:?} 已存在", pane_id);
            }

            panes.insert(pane_id, pane.clone());
            debug!(
                "面板添加到映射成功: pane_id={:?}, total_panes={}",
                pane_id,
                panes.len()
            );
        }

        // 启动I/O处理线程
        debug!("启动I/O处理线程: pane_id={:?}", pane_id);
        self.io_handler
            .spawn_io_threads(pane.clone())
            .with_context(|| format!("启动I/O线程失败: pane_id={:?}", pane_id))?;

        // 发送面板添加通知
        debug!("发送面板添加通知: pane_id={:?}", pane_id);
        self.notify(MuxNotification::PaneAdded(pane_id));

        debug!(
            "创建面板成功: pane_id={:?}, size={}x{}, shell={}, total_panes={}",
            pane_id,
            size.cols,
            size.rows,
            config.shell_config.program,
            self.pane_count()
        );
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
    /// 统一日志记录规范：
    /// - 记录操作开始、关键步骤和完成时间
    /// - 使用结构化日志格式
    /// - 包含性能指标
    #[instrument(skip(self), fields(pane_id = ?pane_id))]
    pub fn remove_pane(&self, pane_id: PaneId) -> AppResult<()> {
        let pane = {
            debug!("获取面板写锁: pane_id={:?}", pane_id);
            let mut panes = self.panes.write().map_err(|_| {
                error!("无法获取面板写锁: pane_id={:?}", pane_id);
                anyhow!("无法获取面板写锁")
            })?;

            debug!("从映射中移除面板: pane_id={:?}", pane_id);
            let pane = panes.remove(&pane_id).ok_or_else(|| {
                error!("面板不存在: pane_id={:?}", pane_id);
                anyhow!("面板 {:?} 不存在", pane_id)
            })?;

            debug!(
                "面板从映射中移除成功: pane_id={:?}, remaining_panes={}",
                pane_id,
                panes.len()
            );
            pane
        };

        // 标记面板为死亡状态，停止I/O线程
        debug!("标记面板为死亡状态: pane_id={:?}", pane_id);
        pane.mark_dead();

        // 停止I/O处理
        if let Err(e) = self.io_handler.stop_pane_io(pane_id) {
            warn!("停止面板 {:?} I/O处理失败: {}", pane_id, e);
        }

        // 发送面板移除通知
        debug!("发送面板移除通知: pane_id={:?}", pane_id);
        self.notify(MuxNotification::PaneRemoved(pane_id));

        debug!(
            "移除面板成功: pane_id={:?}, remaining_panes={}",
            pane_id,
            self.pane_count()
        );
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

    // === I/O 操作方法 ===

    /// 写入数据到指定面板
    ///
    /// 统一日志记录规范：
    /// - 记录操作开始、关键步骤和完成时间
    /// - 使用结构化日志格式
    /// - 包含性能指标
    #[instrument(skip(self, data), fields(pane_id = ?pane_id, data_len = data.len()))]
    pub fn write_to_pane(&self, pane_id: PaneId, data: &[u8]) -> AppResult<()> {
        let pane = self.get_pane(pane_id).ok_or_else(|| {
            error!("面板不存在: pane_id={:?}", pane_id);
            anyhow!("面板 {:?} 不存在", pane_id)
        })?;

        pane.write(data)
            .with_context(|| format!("写入数据失败: pane_id={:?}", pane_id))?;

        debug!(
            "写入数据成功: pane_id={:?}, data_len={}",
            pane_id,
            data.len()
        );
        Ok(())
    }

    /// 调整面板大小
    ///
    /// 统一日志记录规范：
    /// - 记录操作开始、关键步骤和完成时间
    /// - 使用结构化日志格式
    /// - 包含性能指标
    #[instrument(skip(self), fields(pane_id = ?pane_id, size = ?size))]
    pub fn resize_pane(&self, pane_id: PaneId, size: PtySize) -> AppResult<()> {
        let pane = self.get_pane(pane_id).ok_or_else(|| {
            error!("面板不存在: pane_id={:?}", pane_id);
            anyhow!("面板 {:?} 不存在", pane_id)
        })?;

        pane.resize(size)
            .with_context(|| format!("调整面板大小失败: pane_id={:?}", pane_id))?;

        // 发送大小调整通知
        debug!("发送面板大小调整通知: pane_id={:?}", pane_id);
        self.notify(MuxNotification::PaneResized { pane_id, size });

        debug!(
            "调整面板大小成功: pane_id={:?}, size={}x{}",
            pane_id, size.cols, size.rows
        );
        Ok(())
    }

    // === 通知系统方法 ===

    /// 订阅事件通知
    pub fn subscribe<F>(&self, subscriber: F) -> usize
    where
        F: Fn(&MuxNotification) -> bool + Send + Sync + 'static,
    {
        let subscriber_id = self.next_subscriber_id();

        if let Ok(mut subscribers) = self.subscribers.write() {
            subscribers.insert(subscriber_id, Box::new(subscriber));
            debug!("添加订阅者: {}", subscriber_id);
        } else {
            error!("无法获取订阅者写锁");
        }

        subscriber_id
    }

    /// 取消订阅
    pub fn unsubscribe(&self, subscriber_id: usize) -> bool {
        if let Ok(mut subscribers) = self.subscribers.write() {
            let removed = subscribers.remove(&subscriber_id).is_some();
            if removed {
                debug!("移除订阅者: {}", subscriber_id);
            }
            removed
        } else {
            error!("无法获取订阅者写锁");
            false
        }
    }

    /// 发送通知给所有订阅者
    pub fn notify(&self, notification: MuxNotification) {
        if thread::current().id() == self.main_thread_id {
            self.notify_internal(&notification);
        } else {
            // 从其他线程发送通知，使用通道发送到主线程
            if let Err(e) = self.notification_sender.send(notification) {
                error!("跨线程通知发送失败: {}", e);
            } else {
                debug!("跨线程通知已发送");
            }
        }
    }

    /// 内部通知实现
    fn notify_internal(&self, notification: &MuxNotification) {
        let mut dead_subscribers = Vec::new();

        if let Ok(subscribers) = self.subscribers.read() {
            for (&subscriber_id, callback) in subscribers.iter() {
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    callback(notification)
                })) {
                    Ok(true) => {
                        // 订阅者处理成功，继续保持订阅
                        trace!("订阅者 {} 处理通知成功", subscriber_id);
                    }
                    Ok(false) => {
                        // 订阅者返回false，标记为需要移除
                        debug!("订阅者 {} 请求取消订阅", subscriber_id);
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
                    debug!("清理无效订阅者: {}", subscriber_id);
                }
            }
        }
    }

    /// 从任意线程发送通知到主线程
    pub fn notify_from_any_thread(&self, notification: MuxNotification) {
        if let Err(e) = self.notification_sender.send(notification) {
            error!("跨线程通知发送失败: {}", e);
        } else {
            debug!("跨线程通知已发送");
        }
    }

    /// 处理来自其他线程的通知（应该在主线程定期调用）
    pub fn process_notifications(&self) {
        if let Ok(receiver_guard) = self.notification_receiver.read() {
            if let Some(receiver) = receiver_guard.as_ref() {
                // 处理所有待处理的通知
                while let Ok(notification) = receiver.try_recv() {
                    self.notify_internal(&notification);
                }
            }
        }
    }

    /// 启动通知处理线程（可选的自动处理模式）
    pub fn start_notification_processor(self: Arc<Self>) -> thread::JoinHandle<()> {
        let mux = Arc::clone(&self);
        thread::spawn(move || {
            // 取出接收器，避免重复访问
            let receiver = {
                if let Ok(mut receiver_guard) = mux.notification_receiver.write() {
                    receiver_guard.take()
                } else {
                    error!("无法获取通知接收器");
                    return;
                }
            };

            if let Some(receiver) = receiver {
                loop {
                    if mux.shutting_down.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }

                    match receiver.recv_timeout(Duration::from_millis(20)) {
                        Ok(notification) => {
                            mux.notify_internal(&notification);
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                            // 更频繁地检查关闭标志（50ms而不是200ms）
                            continue;
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                            break;
                        }
                    }
                }
            }
        })
    }

    /// 全局通知发送器（用于从任意线程发送通知）
    /// 这需要配合单例模式使用
    pub fn notify_from_any_thread_static(notification: MuxNotification) {
        crate::mux::singleton::notify_global(notification);
    }

    // === Tauri 事件集成 ===

    /// 将MuxNotification转换为Tauri事件名称和数据
    ///
    /// 统一事件命名规范：
    /// - 使用下划线格式 (terminal_output) 而不是连字符格式 (terminal-output)
    /// - 确保事件命名的一致性
    /// - 使用结构体自动序列化确保字段名一致性
    pub fn notification_to_tauri_event(notification: &MuxNotification) -> (&'static str, String) {
        use crate::mux::{
            TerminalClosedEvent, TerminalCreatedEvent, TerminalExitEvent, TerminalOutputEvent,
            TerminalResizedEvent,
        };

        fn safe_serialize<T: serde::Serialize>(event: &T, fallback: &str) -> String {
            serde_json::to_string(event).unwrap_or_else(|e| {
                tracing::error!("JSON序列化失败: {}, 使用默认值", e);
                fallback.to_string()
            })
        }

        match notification {
            MuxNotification::PaneOutput { pane_id, data } => {
                let event = TerminalOutputEvent {
                    pane_id: *pane_id,
                    data: String::from_utf8_lossy(data).to_string(),
                };
                (
                    "terminal_output",
                    safe_serialize(&event, "{\"pane_id\":0,\"data\":\"\"}"),
                )
            }
            MuxNotification::PaneAdded(pane_id) => {
                let event = TerminalCreatedEvent { pane_id: *pane_id };
                (
                    "terminal_created",
                    safe_serialize(&event, "{\"pane_id\":0}"),
                )
            }
            MuxNotification::PaneRemoved(pane_id) => {
                let event = TerminalClosedEvent { pane_id: *pane_id };
                ("terminal_closed", safe_serialize(&event, "{\"pane_id\":0}"))
            }
            MuxNotification::PaneResized { pane_id, size } => {
                let event = TerminalResizedEvent {
                    pane_id: *pane_id,
                    rows: size.rows,
                    cols: size.cols,
                };
                (
                    "terminal_resized",
                    safe_serialize(&event, "{\"pane_id\":0,\"rows\":24,\"cols\":80}"),
                )
            }
            MuxNotification::PaneExited { pane_id, exit_code } => {
                let event = TerminalExitEvent {
                    pane_id: *pane_id,
                    exit_code: *exit_code,
                };
                (
                    "terminal_exit",
                    safe_serialize(&event, "{\"pane_id\":0,\"exit_code\":0}"),
                )
            }
            MuxNotification::PaneCwdChanged { pane_id, cwd } => (
                "pane_cwd_changed",
                format!("{{\"pane_id\":{},\"cwd\":\"{}\"}}", pane_id.as_u32(), cwd),
            ),
        }
    }

    /// 创建一个简单的日志订阅者（用于调试）
    pub fn create_debug_subscriber() -> SubscriberCallback {
        Box::new(|notification| {
            let (event_name, payload) = Self::notification_to_tauri_event(&notification);
            tracing::debug!("事件: {} -> {}", event_name, payload);
            true
        })
    }

    // === Shell Integration 方法 ===

    /// 设置面板的Shell Integration
    pub fn setup_pane_integration(&self, pane_id: PaneId) -> AppResult<()> {
        self.shell_integration.enable_integration(pane_id);
        Ok(())
    }

    /// 设置面板的Shell Integration并注入脚本
    pub fn setup_pane_integration_with_script(
        &self,
        pane_id: PaneId,
        silent: bool,
    ) -> AppResult<()> {
        use crate::shell::ShellType;

        // 启用Shell Integration
        self.shell_integration.enable_integration(pane_id);

        // 检测Shell类型
        if let Ok(panes) = self.panes.read() {
            if let Some(_pane) = panes.get(&pane_id) {
                let shell_type = ShellType::Bash;
                self.shell_integration
                    .set_pane_shell_type(pane_id, shell_type.clone());

                if !silent {
                    // 非静默模式：生成完整脚本
                    if let Ok(script) = self.shell_integration.generate_shell_script(&shell_type) {
                        self.write_to_pane(pane_id, script.as_bytes())?;
                    }
                }
            } else {
                return Err(anyhow::Error::msg(format!("面板 {} 不存在", pane_id)));
            }
        }

        Ok(())
    }

    /// 检查面板是否已集成Shell Integration
    pub fn is_pane_integrated(&self, pane_id: PaneId) -> bool {
        self.shell_integration.get_pane_state(pane_id).is_some()
    }

    /// 获取面板的当前工作目录
    pub fn get_pane_cwd(&self, pane_id: PaneId) -> Option<String> {
        self.shell_integration
            .get_current_working_directory(pane_id)
    }

    /// 更新面板的当前工作目录
    pub fn update_pane_cwd(&self, pane_id: PaneId, cwd: String) {
        self.shell_integration
            .update_current_working_directory(pane_id, cwd);
    }

    /// 获取面板的完整Shell状态
    pub fn get_pane_shell_state(&self, pane_id: PaneId) -> Option<crate::shell::PaneShellState> {
        self.shell_integration.get_pane_shell_state(pane_id)
    }

    /// 设置面板的Shell类型
    pub fn set_pane_shell_type(&self, pane_id: PaneId, shell_type: crate::shell::ShellType) {
        self.shell_integration
            .set_pane_shell_type(pane_id, shell_type);
    }

    /// 生成Shell集成脚本
    pub fn generate_shell_integration_script(
        &self,
        shell_type: &crate::shell::ShellType,
    ) -> anyhow::Result<String> {
        self.shell_integration.generate_shell_script(shell_type)
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

    /// 获取面板的当前命令信息
    pub fn get_pane_current_command(&self, pane_id: PaneId) -> Option<crate::shell::CommandInfo> {
        self.shell_integration.get_current_command(pane_id)
    }

    /// 获取面板的命令历史
    pub fn get_pane_command_history(&self, pane_id: PaneId) -> Vec<crate::shell::CommandInfo> {
        self.shell_integration.get_command_history(pane_id)
    }

    // === 生命周期管理 ===

    /// 清理所有资源
    pub fn shutdown(&self) -> AppResult<()> {
        let shutdown_start = std::time::Instant::now();

        // 标记为关闭状态，使通知处理线程能尽快退出
        self.shutting_down
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // 获取所有面板ID和引用
        let pane_ids: Vec<PaneId> = self.list_panes();
        tracing::debug!("准备关闭 {} 个面板", pane_ids.len());

        // 立即标记所有面板为死亡状态，加速关闭过程
        {
            let panes = self.panes.read().map_err(|_| anyhow!("无法获取面板读锁"))?;
            for (_pane_id, pane) in panes.iter() {
                pane.mark_dead();
            }
        }
        tracing::debug!("所有面板已标记为死亡状态");

        // 逐个关闭面板
        let mut failed_panes = Vec::new();
        for pane_id in pane_ids {
            match self.remove_pane(pane_id) {
                Ok(_) => {
                    tracing::debug!("面板 {:?} 关闭成功", pane_id);
                }
                Err(e) => {
                    tracing::warn!("关闭面板 {:?} 失败: {}", pane_id, e);
                    failed_panes.push(pane_id);
                }
            }

            // 如果单个面板关闭时间过长，总体超时保护
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
            let count = subscribers.len();
            subscribers.clear();
            tracing::debug!("清理了 {} 个订阅者", count);
        } else {
            tracing::warn!("无法获取订阅者锁进行清理");
        }

        // 关闭I/O处理器
        match self.io_handler.shutdown() {
            Ok(_) => {
                tracing::debug!("I/O处理器已关闭");
            }
            Err(e) => {
                tracing::warn!("关闭I/O处理器失败（可能已经关闭）: {}", e);
            }
        }

        tracing::debug!("TerminalMux 关闭完成");
        Ok(())
    }
}

impl Default for TerminalMux {
    fn default() -> Self {
        Self::new()
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
