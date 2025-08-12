//! 优化的I/O线程池实现
//!
//! 使用线程池和异步I/O优化终端I/O处理，减少线程数量和内存使用

use anyhow::{anyhow, Context};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Weak};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, trace, warn};

use crate::mux::{MuxNotification, Pane, PaneId};
use crate::utils::error::AppResult;
use bytes::Bytes;

/// I/O任务类型
pub enum IoTask {
    /// 启动面板I/O处理
    StartPaneIo {
        pane: Arc<dyn Pane>,
        reader: Box<dyn Read + Send>,
    },
    /// 停止面板I/O处理
    StopPaneIo { pane_id: PaneId },
    /// 关闭线程池
    Shutdown,
}

impl std::fmt::Debug for IoTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoTask::StartPaneIo { pane, .. } => f
                .debug_struct("StartPaneIo")
                .field("pane_id", &pane.pane_id())
                .field("reader", &"<Box<dyn Read + Send>>")
                .finish(),
            IoTask::StopPaneIo { pane_id } => f
                .debug_struct("StopPaneIo")
                .field("pane_id", pane_id)
                .finish(),
            IoTask::Shutdown => f.debug_struct("Shutdown").finish(),
        }
    }
}

/// I/O线程池配置
#[derive(Debug, Clone)]
pub struct IoThreadPoolConfig {
    /// 工作线程数量
    pub worker_threads: usize,
    /// 任务队列容量
    pub task_queue_capacity: usize,
    /// 读取缓冲区大小
    pub buffer_size: usize,
    /// 批处理大小阈值（字节）
    pub batch_size: usize,
    /// 批处理时间阈值（毫秒）
    pub flush_interval_ms: u64,
}

impl Default for IoThreadPoolConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get().clamp(2, 4), // 2-4个工作线程
            task_queue_capacity: 1000,
            buffer_size: 4096,
            batch_size: 1024,
            flush_interval_ms: 16, // ~60 FPS
        }
    }
}

/// I/O线程池统计信息
#[derive(Debug, Clone)]
pub struct IoThreadPoolStats {
    /// 活跃面板数量
    pub active_panes: usize,
    /// 工作线程数量
    pub worker_threads: usize,
    /// 待处理任务数量
    pub pending_tasks: usize,
    /// 总处理的字节数
    pub total_bytes_processed: u64,
    /// 总处理的批次数
    pub total_batches_processed: u64,
}

/// 面板I/O状态
#[derive(Debug)]
struct PaneIoState {
    #[allow(dead_code)]
    pane: Weak<dyn Pane>,
    #[allow(dead_code)]
    data_sender: Sender<Vec<u8>>,
    reader_handle: Option<JoinHandle<()>>,
}

/// 优化的I/O线程池
pub struct IoThreadPool {
    config: IoThreadPoolConfig,
    task_sender: Sender<IoTask>,
    #[allow(dead_code)]
    notification_sender: Sender<MuxNotification>,
    worker_handles: Arc<std::sync::Mutex<Option<Vec<JoinHandle<()>>>>>,
    stats: Arc<std::sync::Mutex<IoThreadPoolStats>>,
}

impl IoThreadPool {
    /// 创建新的I/O线程池
    pub fn new(notification_sender: Sender<MuxNotification>) -> Self {
        Self::with_config(notification_sender, IoThreadPoolConfig::default())
    }

    /// 使用自定义配置创建I/O线程池
    pub fn with_config(
        notification_sender: Sender<MuxNotification>,
        config: IoThreadPoolConfig,
    ) -> Self {
        info!("创建I/O线程池，工作线程数: {}", config.worker_threads);

        let (task_sender, task_receiver) = bounded(config.task_queue_capacity);
        let stats = Arc::new(std::sync::Mutex::new(IoThreadPoolStats {
            active_panes: 0,
            worker_threads: config.worker_threads,
            pending_tasks: 0,
            total_bytes_processed: 0,
            total_batches_processed: 0,
        }));

        // 启动工作线程
        let mut worker_handles = Vec::new();
        for worker_id in 0..config.worker_threads {
            let handle = Self::spawn_worker_thread(
                worker_id,
                task_receiver.clone(),
                notification_sender.clone(),
                config.clone(),
                Arc::clone(&stats),
            );
            worker_handles.push(handle);
        }

        info!("I/O线程池创建完成，工作线程数: {}", config.worker_threads);

        Self {
            config,
            task_sender,
            notification_sender,
            worker_handles: Arc::new(std::sync::Mutex::new(Some(worker_handles))),
            stats,
        }
    }

    /// 启动面板I/O处理
    pub fn start_pane_io(&self, pane: Arc<dyn Pane>) -> AppResult<()> {
        let pane_id = pane.pane_id();
        debug!("启动面板 {:?} 的I/O处理", pane_id);

        // 获取读取器
        let reader = pane
            .reader()
            .with_context(|| format!("无法获取面板 {:?} 的读取器", pane_id))?;

        // 发送启动任务
        let task = IoTask::StartPaneIo { pane, reader };
        self.task_sender
            .send(task)
            .map_err(|e| anyhow!("发送启动任务失败: {}", e))?;

        // 更新统计信息
        if let Ok(mut stats) = self.stats.lock() {
            stats.active_panes += 1;
            stats.pending_tasks += 1;
        }

        debug!("面板 {:?} I/O处理启动请求已发送", pane_id);
        Ok(())
    }

    /// 停止面板I/O处理
    pub fn stop_pane_io(&self, pane_id: PaneId) -> AppResult<()> {
        debug!("停止面板 {:?} 的I/O处理", pane_id);

        let task = IoTask::StopPaneIo { pane_id };
        self.task_sender
            .send(task)
            .map_err(|e| anyhow!("发送停止任务失败: {}", e))?;

        // 更新统计信息
        if let Ok(mut stats) = self.stats.lock() {
            stats.active_panes = stats.active_panes.saturating_sub(1);
            stats.pending_tasks += 1;
        }

        debug!("面板 {:?} I/O处理停止请求已发送", pane_id);
        Ok(())
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> IoThreadPoolStats {
        self.stats
            .lock()
            .map(|stats| stats.clone())
            .unwrap_or_else(|_| IoThreadPoolStats {
                active_panes: 0,
                worker_threads: self.config.worker_threads,
                pending_tasks: 0,
                total_bytes_processed: 0,
                total_batches_processed: 0,
            })
    }

    /// 关闭线程池
    pub fn shutdown(&self) -> AppResult<()> {
        info!("开始关闭I/O线程池");

        // 发送关闭信号给所有工作线程
        let mut successful_sends = 0;
        for worker_id in 0..self.config.worker_threads {
            match self.task_sender.send(IoTask::Shutdown) {
                Ok(_) => {
                    successful_sends += 1;
                    debug!("成功向工作线程 {} 发送关闭信号", worker_id);
                }
                Err(e) => {
                    // 通道已断开，说明工作线程可能已经退出了
                    debug!(
                        "向工作线程 {} 发送关闭信号失败（通道已断开）: {}",
                        worker_id, e
                    );
                }
            }
        }

        info!(
            "已向 {}/{} 个工作线程发送关闭信号",
            successful_sends, self.config.worker_threads
        );

        // 等待所有工作线程完成（带超时）
        if let Ok(mut handles_guard) = self.worker_handles.lock() {
            if let Some(handles) = handles_guard.take() {
                let start_time = std::time::Instant::now();
                let timeout = Duration::from_secs(5); // 5秒超时

                for (worker_id, handle) in handles.into_iter().enumerate() {
                    if start_time.elapsed() > timeout {
                        warn!("等待工作线程退出超时，强制继续关闭");
                        break;
                    }

                    match handle.join() {
                        Ok(_) => {
                            debug!("工作线程 {} 正常退出", worker_id);
                        }
                        Err(e) => {
                            // 线程退出异常不一定是错误，可能是正常的关闭过程
                            debug!("工作线程 {} 退出: {:?}", worker_id, e);
                        }
                    }
                }
            }
        }

        info!("I/O线程池关闭完成");
        Ok(())
    }

    /// 启动工作线程
    fn spawn_worker_thread(
        worker_id: usize,
        task_receiver: Receiver<IoTask>,
        notification_sender: Sender<MuxNotification>,
        config: IoThreadPoolConfig,
        stats: Arc<std::sync::Mutex<IoThreadPoolStats>>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            info!("I/O工作线程 {} 已启动", worker_id);

            let mut active_panes: HashMap<PaneId, PaneIoState> = HashMap::new();

            loop {
                match task_receiver.recv() {
                    Ok(IoTask::StartPaneIo { pane, reader }) => {
                        let pane_id = pane.pane_id();
                        debug!("工作线程 {} 处理启动面板 {:?}", worker_id, pane_id);

                        // 如果面板已经在处理中，先停止旧的处理
                        if let Some(old_state) = active_panes.remove(&pane_id) {
                            warn!("面板 {:?} 已在处理中，停止旧的处理", pane_id);
                            if let Some(handle) = old_state.reader_handle {
                                // 不等待旧线程，让它自然退出
                                let _ = handle.join();
                            }
                        }

                        // 创建数据通道
                        let (data_sender, data_receiver) = bounded::<Vec<u8>>(100);

                        // 启动读取线程
                        let reader_handle = Self::spawn_reader_thread(
                            pane_id,
                            reader,
                            data_sender.clone(),
                            config.buffer_size,
                        );

                        // 启动批处理协程
                        Self::spawn_batch_processor(
                            Arc::downgrade(&pane),
                            data_receiver,
                            notification_sender.clone(),
                            config.batch_size,
                            Duration::from_millis(config.flush_interval_ms),
                            Arc::clone(&stats),
                        );

                        // 记录面板状态
                        let pane_state = PaneIoState {
                            pane: Arc::downgrade(&pane),
                            data_sender,
                            reader_handle: Some(reader_handle),
                        };
                        active_panes.insert(pane_id, pane_state);

                        // 更新统计信息
                        if let Ok(mut stats) = stats.lock() {
                            stats.pending_tasks = stats.pending_tasks.saturating_sub(1);
                        }

                        debug!("工作线程 {} 完成启动面板 {:?}", worker_id, pane_id);
                    }
                    Ok(IoTask::StopPaneIo { pane_id }) => {
                        debug!("工作线程 {} 处理停止面板 {:?}", worker_id, pane_id);

                        if let Some(pane_state) = active_panes.remove(&pane_id) {
                            // 等待读取线程完成
                            if let Some(handle) = pane_state.reader_handle {
                                let _ = handle.join();
                            }
                            debug!("工作线程 {} 完成停止面板 {:?}", worker_id, pane_id);
                        } else {
                            warn!("工作线程 {} 尝试停止不存在的面板 {:?}", worker_id, pane_id);
                        }

                        // 更新统计信息
                        if let Ok(mut stats) = stats.lock() {
                            stats.pending_tasks = stats.pending_tasks.saturating_sub(1);
                        }
                    }
                    Ok(IoTask::Shutdown) => {
                        info!("工作线程 {} 收到关闭信号", worker_id);
                        break;
                    }
                    Err(e) => {
                        error!("工作线程 {} 接收任务失败: {}", worker_id, e);
                        break;
                    }
                }
            }

            // 清理所有活跃面板
            for (pane_id, pane_state) in active_panes {
                debug!("工作线程 {} 清理面板 {:?}", worker_id, pane_id);
                if let Some(handle) = pane_state.reader_handle {
                    let _ = handle.join();
                }
            }

            info!("I/O工作线程 {} 已退出", worker_id);
        })
    }

    /// 启动读取线程
    fn spawn_reader_thread(
        pane_id: PaneId,
        mut reader: Box<dyn Read + Send>,
        data_sender: Sender<Vec<u8>>,
        buffer_size: usize,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            debug!("面板 {:?} 读取线程已启动", pane_id);
            let mut buffer = vec![0u8; buffer_size];

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        debug!("面板 {:?} PTY 已关闭 (EOF)", pane_id);
                        break;
                    }
                    Ok(bytes_read) => {
                        let data = buffer[..bytes_read].to_vec();
                        trace!("面板 {:?} 读取了 {} 字节数据", pane_id, bytes_read);

                        if let Err(e) = data_sender.send(data) {
                            debug!("面板 {:?} 批处理线程已关闭，停止读取: {}", pane_id, e);
                            break;
                        }
                    }
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::Interrupted => {
                            trace!("面板 {:?} 读取被中断，继续", pane_id);
                            continue;
                        }
                        std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(1));
                            continue;
                        }
                        _ => {
                            warn!("面板 {:?} 读取错误: {}", pane_id, e);
                            break;
                        }
                    },
                }
            }

            debug!("面板 {:?} 读取线程已退出", pane_id);
        })
    }

    /// 启动批处理协程（在工作线程中运行）
    fn spawn_batch_processor(
        weak_pane: Weak<dyn Pane>,
        data_receiver: Receiver<Vec<u8>>,
        notification_sender: Sender<MuxNotification>,
        batch_size: usize,
        flush_interval: Duration,
        stats: Arc<std::sync::Mutex<IoThreadPoolStats>>,
    ) {
        // 在当前工作线程中运行批处理逻辑，而不是启动新线程
        let pane_id = if let Some(pane) = weak_pane.upgrade() {
            let id = pane.pane_id();
            debug!("面板 {:?} 批处理器已启动", id);
            id
        } else {
            error!("批处理器启动时面板已被释放");
            return;
        };

        let mut batch_data = Vec::new();
        let mut last_flush = Instant::now();

        loop {
            // 检查面板是否还存活
            let pane_alive = if let Some(pane) = weak_pane.upgrade() {
                !pane.is_dead()
            } else {
                false
            };

            if !pane_alive {
                debug!("面板 {:?} 已死亡，退出批处理器", pane_id);
                break;
            }

            // 尝试接收数据（非阻塞）
            match data_receiver.try_recv() {
                Ok(data) => {
                    batch_data.extend_from_slice(&data);
                    trace!(
                        "面板 {:?} 批处理缓冲区大小: {} 字节",
                        pane_id,
                        batch_data.len()
                    );
                }
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    // 没有数据，检查是否需要超时刷新
                }
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    debug!("面板 {:?} 数据通道已断开", pane_id);
                    break;
                }
            }

            // 检查是否需要刷新批处理数据
            let should_flush = !batch_data.is_empty()
                && (batch_data.len() >= batch_size || last_flush.elapsed() >= flush_interval);

            if should_flush {
                let data_to_send = std::mem::take(&mut batch_data);
                let send_len = data_to_send.len();
                let notification = MuxNotification::PaneOutput {
                    pane_id,
                    data: Bytes::from(data_to_send),
                };

                debug!("面板 {:?} 发送批处理数据: {} 字节", pane_id, send_len);

                if let Err(e) = notification_sender.send(notification) {
                    debug!("面板 {:?} 发送通知失败（可能是正在关闭）: {}", pane_id, e);
                    break;
                }

                // 更新统计信息
                if let Ok(mut stats) = stats.lock() {
                    stats.total_bytes_processed += send_len as u64;
                    stats.total_batches_processed += 1;
                }

                last_flush = Instant::now();
            } else if batch_data.is_empty() {
                // 没有数据时短暂休眠，避免忙等待
                thread::sleep(Duration::from_millis(1));
            }
        }

        // 退出前发送剩余数据
        if !batch_data.is_empty() {
            let notification = MuxNotification::PaneOutput {
                pane_id,
                data: Bytes::from(batch_data),
            };

            if let Err(e) = notification_sender.send(notification) {
                debug!(
                    "面板 {:?} 发送最后的批处理数据失败（可能是正在关闭）: {}",
                    pane_id, e
                );
            }
        }

        // 发送面板退出通知
        let exit_notification = MuxNotification::PaneExited {
            pane_id,
            exit_code: None,
        };

        if let Err(e) = notification_sender.send(exit_notification) {
            debug!(
                "面板 {:?} 发送退出通知失败（可能是正在关闭）: {}",
                pane_id, e
            );
        }

        debug!("面板 {:?} 批处理器已退出", pane_id);
    }
}

// 添加num_cpus依赖的简单实现（如果不想添加外部依赖）
mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    }
}
