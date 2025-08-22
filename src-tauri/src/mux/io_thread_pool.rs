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
use crate::shell::ShellIntegrationManager;
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
    worker_handles: Arc<std::sync::Mutex<Option<Vec<JoinHandle<()>>>>>,
    stats: Arc<std::sync::Mutex<IoThreadPoolStats>>,
    pane_registry: Arc<std::sync::Mutex<HashMap<PaneId, Weak<dyn Pane>>>>,
}

impl IoThreadPool {
    /// 使用自定义配置创建I/O线程池
    pub fn with_config(
        notification_sender: Sender<MuxNotification>,
        shell_integration: Arc<ShellIntegrationManager>,
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

        let pane_registry: Arc<std::sync::Mutex<HashMap<PaneId, Weak<dyn Pane>>>> =
            Arc::new(std::sync::Mutex::new(HashMap::new()));

        // 启动工作线程
        let mut worker_handles = Vec::new();
        for worker_id in 0..config.worker_threads {
            let handle = Self::spawn_worker_thread(
                worker_id,
                task_receiver.clone(),
                notification_sender.clone(),
                config.clone(),
                Arc::clone(&stats),
                Arc::clone(&shell_integration),
                Arc::clone(&pane_registry),
            );
            worker_handles.push(handle);
        }

        info!("I/O线程池创建完成，工作线程数: {}", config.worker_threads);

        Self {
            config,
            task_sender,
            worker_handles: Arc::new(std::sync::Mutex::new(Some(worker_handles))),
            stats,
            pane_registry,
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

        // 先记录到全局注册表，便于 stop/shutdown 能够标记退出
        if let Ok(mut reg) = self.pane_registry.lock() {
            reg.insert(pane_id, Arc::downgrade(&pane));
        }

        // 发送启动任务
        let task = IoTask::StartPaneIo { pane, reader };
        self.task_sender
            .send(task)
            .map_err(|e| anyhow!("发送启动任务失败: {}", e))?;

        // 更新统计信息（仅标记有一个启动任务待处理）
        if let Ok(mut stats) = self.stats.lock() {
            stats.pending_tasks += 1;
        }

        debug!("面板 {:?} I/O处理启动请求已发送", pane_id);
        Ok(())
    }

    /// 停止面板I/O处理
    pub fn stop_pane_io(&self, pane_id: PaneId) -> AppResult<()> {
        debug!("停止面板 {:?} 的I/O处理", pane_id);

        // 直接通过注册表标记 pane 为死亡，促使批处理循环尽快退出
        if let Ok(reg) = self.pane_registry.lock() {
            if let Some(weak) = reg.get(&pane_id) {
                if let Some(pane_arc) = weak.upgrade() {
                    pane_arc.mark_dead();
                }
            }
        }

        let task = IoTask::StopPaneIo { pane_id };
        self.task_sender
            .send(task)
            .map_err(|e| anyhow!("发送停止任务失败: {}", e))?;

        // 更新统计信息（仅标记有一个停止任务待处理）
        if let Ok(mut stats) = self.stats.lock() {
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

        // 优先标记所有pane为死亡，促使批处理循环尽快退出
        let mut marked = 0usize;
        if let Ok(reg) = self.pane_registry.lock() {
            for (_id, weak) in reg.iter() {
                if let Some(pane) = weak.upgrade() {
                    pane.mark_dead();
                    marked += 1;
                }
            }
        }
        debug!("在关闭前已标记 {} 个pane为死亡", marked);

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

        // 等待所有工作线程完成（带更短的超时和并发等待）
        if let Ok(mut handles_guard) = self.worker_handles.lock() {
            if let Some(handles) = handles_guard.take() {
                let timeout = Duration::from_millis(500); // 进一步缩短超时到500毫秒
                let start_time = std::time::Instant::now();
                
                // 使用非阻塞的方式检查线程是否完成
                let mut finished_threads = vec![false; handles.len()];
                let mut active_handles: Vec<Option<std::thread::JoinHandle<()>>> = 
                    handles.into_iter().map(Some).collect();
                
                // 轮询检查线程完成状态
                while start_time.elapsed() < timeout {
                    let mut all_finished = true;
                    
                    for (worker_id, (finished, handle_opt)) in 
                        finished_threads.iter_mut().zip(active_handles.iter_mut()).enumerate() {
                        
                        if !*finished {
                            if let Some(handle) = handle_opt.take() {
                                // 非阻塞检查线程是否完成
                                if handle.is_finished() {
                                    match handle.join() {
                                        Ok(_) => {
                                            debug!("工作线程 {} 正常退出", worker_id);
                                            *finished = true;
                                        }
                                        Err(e) => {
                                            debug!("工作线程 {} 退出异常: {:?}", worker_id, e);
                                            *finished = true;
                                        }
                                    }
                                } else {
                                    // 线程还在运行，放回去继续检查
                                    *handle_opt = Some(handle);
                                    all_finished = false;
                                }
                            }
                        }
                    }
                    
                    if all_finished {
                        break;
                    }
                    
                    // 更短的休眠时间以提高响应速度
                    std::thread::sleep(Duration::from_millis(5));
                }
                
                // 检查是否有未完成的线程
                let unfinished_count = finished_threads.iter().filter(|&&f| !f).count();
                if unfinished_count > 0 {
                    warn!("有 {} 个工作线程在超时后仍未退出，强制继续关闭", unfinished_count);
                } else {
                    debug!("所有工作线程已正常退出");
                }
            }
        }

        info!("I/O线程池关闭完成");

        // 清空注册表以避免残留
        if let Ok(mut reg) = self.pane_registry.lock() {
            reg.clear();
        }
        Ok(())
    }

    /// 启动工作线程
    fn spawn_worker_thread(
        worker_id: usize,
        task_receiver: Receiver<IoTask>,
        notification_sender: Sender<MuxNotification>,
        config: IoThreadPoolConfig,
        stats: Arc<std::sync::Mutex<IoThreadPoolStats>>,
        shell_integration: Arc<ShellIntegrationManager>,
        pane_registry: Arc<std::sync::Mutex<HashMap<PaneId, Weak<dyn Pane>>>>,
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
                                // 等待旧读线程退出，避免资源泄漏
                                let _ = handle.join();
                            }
                            // 重启路径：活跃面板-1（随后新的启动会再次登记为活跃）
                            if let Ok(mut stats) = stats.lock() {
                                stats.active_panes = stats.active_panes.saturating_sub(1);
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

                        // 记录面板状态（先登记，后运行批处理）
                        let pane_state = PaneIoState {
                            pane: Arc::downgrade(&pane),
                            data_sender,
                            reader_handle: Some(reader_handle),
                        };
                        active_panes.insert(pane_id, pane_state);

                        // 更新统计信息：活跃面板+1（真正开始处理时）
                        if let Ok(mut stats) = stats.lock() {
                            stats.active_panes += 1;
                        }

                        // 更新统计信息（Start 任务完成：此刻任务已由工作线程受理完成）
                        if let Ok(mut stats) = stats.lock() {
                            stats.pending_tasks = stats.pending_tasks.saturating_sub(1);
                        }

                        // 启动批处理（在当前工作线程中阻塞运行，直到 pane 结束或通道断开）
                        Self::spawn_batch_processor(
                            Arc::downgrade(&pane),
                            data_receiver,
                            notification_sender.clone(),
                            config.batch_size,
                            Duration::from_millis(config.flush_interval_ms),
                            Arc::clone(&stats),
                            Arc::clone(&shell_integration),
                        );

                        // 批处理返回后执行清理：移除并等待读取线程结束
                        if let Some(mut pane_state) = active_panes.remove(&pane_id) {
                            if let Some(handle) = pane_state.reader_handle.take() {
                                let _ = handle.join();
                            }
                            // EOF/自然退出路径：活跃面板-1
                            if let Ok(mut stats) = stats.lock() {
                                stats.active_panes = stats.active_panes.saturating_sub(1);
                            }
                            // 从注册表移除
                            if let Ok(mut reg) = pane_registry.lock() {
                                reg.remove(&pane_id);
                            }
                        }

                        debug!("工作线程 {} 完成面板 {:?} 的处理", worker_id, pane_id);
                    }
                    Ok(IoTask::StopPaneIo { pane_id }) => {
                        debug!("工作线程 {} 处理停止面板 {:?}", worker_id, pane_id);

                        if let Some(pane_state) = active_panes.remove(&pane_id) {
                            // 等待读取线程完成
                            if let Some(handle) = pane_state.reader_handle {
                                let _ = handle.join();
                            }
                            // 更新统计信息：停止任务完成，活跃面板-1
                            if let Ok(mut stats) = stats.lock() {
                                stats.active_panes = stats.active_panes.saturating_sub(1);
                            }
                            // 从注册表移除
                            if let Ok(mut reg) = pane_registry.lock() {
                                reg.remove(&pane_id);
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
                // 从注册表移除
                if let Ok(mut reg) = pane_registry.lock() {
                    reg.remove(&pane_id);
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
        shell_integration: Arc<ShellIntegrationManager>,
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

            // 早期退出机制：尝试发送一个测试消息来检查通道状态
            // 注意：这里我们移除这个检查，因为crossbeam_channel::Sender没有is_disconnected方法
            // 改为依赖其他退出条件

            // 根据 flush_interval 计算阻塞等待时间，避免忙等待
            let elapsed = last_flush.elapsed();
            let timeout = if batch_data.is_empty() {
                // 没有数据时，等待一个完整的 flush 周期
                flush_interval
            } else {
                // 已有数据时，等待到达时间阈值（或为0，立即检查刷新）
                flush_interval.saturating_sub(elapsed)
            };

            // 使用更短的超时来更频繁地检查面板状态
            let recv_timeout = std::cmp::min(timeout, Duration::from_millis(50));
            
            match data_receiver.recv_timeout(recv_timeout) {
                Ok(data) => {
                    batch_data.extend_from_slice(&data);
                    trace!(
                        "面板 {:?} 批处理缓冲区大小: {} 字节",
                        pane_id,
                        batch_data.len()
                    );
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // 超时：后续根据 should_flush 判断是否刷新
                    // 更频繁地检查面板状态以便快速退出
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    debug!("面板 {:?} 数据通道已断开", pane_id);
                    break;
                }
            }

            // 检查是否需要刷新批处理数据（大小阈值或时间阈值）
            let should_flush = !batch_data.is_empty()
                && (batch_data.len() >= batch_size || last_flush.elapsed() >= flush_interval);

            if should_flush {
                let data_to_send = std::mem::take(&mut batch_data);
                let data_str = String::from_utf8_lossy(&data_to_send);
                shell_integration.process_output(pane_id, &data_str);
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
            }
        }

        // 退出前发送剩余数据
        if !batch_data.is_empty() {
            let send_len = batch_data.len();
            let data_str = String::from_utf8_lossy(&batch_data);
            shell_integration.process_output(pane_id, &data_str);

            let notification = MuxNotification::PaneOutput {
                pane_id,
                data: Bytes::from(batch_data),
            };

            match notification_sender.send(notification) {
                Ok(_) => {
                    if let Ok(mut stats) = stats.lock() {
                        stats.total_bytes_processed += send_len as u64;
                        stats.total_batches_processed += 1;
                    }
                }
                Err(e) => {
                    debug!(
                        "面板 {:?} 发送最后的批处理数据失败（可能是正在关闭）: {}",
                        pane_id, e
                    );
                }
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
