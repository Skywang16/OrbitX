//! 优化的 I/O 处理模块
//!
//! 使用线程池优化I/O处理，减少线程数量和资源使用

use anyhow::{anyhow, Context};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::io::Read;
use std::sync::{Arc, Weak};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, trace, warn};

use crate::mux::io_thread_pool::{IoThreadPool, IoThreadPoolConfig};
use crate::mux::{MuxNotification, Pane, PaneId};
use crate::utils::error::AppResult;

/// I/O 处理配置
#[derive(Debug, Clone)]
pub struct IoConfig {
    /// 读取缓冲区大小
    pub buffer_size: usize,
    /// 批处理大小阈值（字节）
    pub batch_size: usize,
    /// 批处理时间阈值（毫秒）
    pub flush_interval_ms: u64,
}

impl Default for IoConfig {
    fn default() -> Self {
        Self {
            buffer_size: 4096,
            batch_size: 1024,
            flush_interval_ms: 16, // ~60 FPS
        }
    }
}

/// I/O 处理器模式
#[derive(Debug, Clone)]
pub enum IoMode {
    /// 传统模式：每个面板独立线程
    Legacy,
    /// 线程池模式：使用共享线程池
    ThreadPool,
}

impl Default for IoMode {
    fn default() -> Self {
        Self::ThreadPool // 默认使用线程池模式
    }
}

/// I/O 处理器
pub struct IoHandler {
    config: IoConfig,
    mode: IoMode,
    notification_sender: Sender<MuxNotification>,
    thread_pool: Option<IoThreadPool>,
}

impl IoHandler {
    /// 创建新的 I/O 处理器（默认使用线程池模式）
    pub fn new(notification_sender: Sender<MuxNotification>) -> Self {
        Self::with_mode(notification_sender, IoMode::default())
    }

    /// 使用指定模式创建 I/O 处理器
    pub fn with_mode(notification_sender: Sender<MuxNotification>, mode: IoMode) -> Self {
        let config = IoConfig::default();
        let thread_pool = match mode {
            IoMode::ThreadPool => {
                let pool_config = IoThreadPoolConfig {
                    worker_threads: num_cpus::get().clamp(2, 4),
                    task_queue_capacity: 1000,
                    buffer_size: config.buffer_size,
                    batch_size: config.batch_size,
                    flush_interval_ms: config.flush_interval_ms,
                };
                Some(IoThreadPool::with_config(
                    notification_sender.clone(),
                    pool_config,
                ))
            }
            IoMode::Legacy => None,
        };

        info!("创建I/O处理器，模式: {:?}", mode);

        Self {
            config,
            mode,
            notification_sender,
            thread_pool,
        }
    }

    /// 使用自定义配置创建 I/O 处理器
    pub fn with_config(notification_sender: Sender<MuxNotification>, config: IoConfig) -> Self {
        Self::with_config_and_mode(notification_sender, config, IoMode::default())
    }

    /// 使用自定义配置和模式创建 I/O 处理器
    pub fn with_config_and_mode(
        notification_sender: Sender<MuxNotification>,
        config: IoConfig,
        mode: IoMode,
    ) -> Self {
        let thread_pool = match mode {
            IoMode::ThreadPool => {
                let pool_config = IoThreadPoolConfig {
                    worker_threads: num_cpus::get().clamp(2, 4),
                    task_queue_capacity: 1000,
                    buffer_size: config.buffer_size,
                    batch_size: config.batch_size,
                    flush_interval_ms: config.flush_interval_ms,
                };
                Some(IoThreadPool::with_config(
                    notification_sender.clone(),
                    pool_config,
                ))
            }
            IoMode::Legacy => None,
        };

        info!("创建I/O处理器，模式: {:?}，配置: {:?}", mode, config);

        Self {
            config,
            mode,
            notification_sender,
            thread_pool,
        }
    }

    /// 获取当前配置
    pub fn config(&self) -> &IoConfig {
        &self.config
    }

    /// 获取当前模式
    pub fn mode(&self) -> &IoMode {
        &self.mode
    }

    /// 为面板启动 I/O 处理
    pub fn spawn_io_threads(&self, pane: Arc<dyn Pane>) -> AppResult<()> {
        let pane_id = pane.pane_id();
        debug!("为面板 {:?} 启动 I/O 处理，模式: {:?}", pane_id, self.mode);

        match &self.mode {
            IoMode::ThreadPool => {
                if let Some(thread_pool) = &self.thread_pool {
                    thread_pool
                        .start_pane_io(pane)
                        .with_context(|| format!("线程池启动面板 {:?} I/O 处理失败", pane_id))?;
                    info!("面板 {:?} 的 I/O 处理已提交到线程池", pane_id);
                } else {
                    error!("线程池模式下但线程池未初始化");
                    return Err(anyhow!("线程池未初始化"));
                }
            }
            IoMode::Legacy => {
                self.spawn_io_threads_legacy(pane)?;
                info!("面板 {:?} 的 I/O 处理线程已启动（传统模式）", pane_id);
            }
        }

        Ok(())
    }

    /// 停止面板 I/O 处理
    pub fn stop_pane_io(&self, pane_id: PaneId) -> AppResult<()> {
        debug!("停止面板 {:?} 的 I/O 处理，模式: {:?}", pane_id, self.mode);

        match &self.mode {
            IoMode::ThreadPool => {
                if let Some(thread_pool) = &self.thread_pool {
                    thread_pool
                        .stop_pane_io(pane_id)
                        .with_context(|| format!("线程池停止面板 {:?} I/O 处理失败", pane_id))?;
                    info!("面板 {:?} 的 I/O 处理停止请求已提交到线程池", pane_id);
                } else {
                    error!("线程池模式下但线程池未初始化");
                    return Err(anyhow!("线程池未初始化"));
                }
            }
            IoMode::Legacy => {
                // 传统模式下，线程会在面板标记为死亡时自动退出
                debug!("传统模式下，面板 {:?} 的线程将自动退出", pane_id);
            }
        }

        Ok(())
    }

    /// 获取 I/O 处理统计信息
    pub fn get_stats(&self) -> Option<crate::mux::io_thread_pool::IoThreadPoolStats> {
        match &self.mode {
            IoMode::ThreadPool => self.thread_pool.as_ref().map(|pool| pool.get_stats()),
            IoMode::Legacy => None,
        }
    }

    /// 关闭 I/O 处理器
    pub fn shutdown(&self) -> AppResult<()> {
        info!("关闭I/O处理器，模式: {:?}", self.mode);

        match &self.mode {
            IoMode::ThreadPool => {
                if let Some(thread_pool) = &self.thread_pool {
                    thread_pool.shutdown().context("关闭线程池失败")?;
                    info!("线程池已关闭");
                }
            }
            IoMode::Legacy => {
                // 传统模式下没有需要特别关闭的资源
                debug!("传统模式下无需特别关闭");
            }
        }

        Ok(())
    }

    /// 传统模式下启动 I/O 处理线程
    fn spawn_io_threads_legacy(&self, pane: Arc<dyn Pane>) -> AppResult<()> {
        let pane_id = pane.pane_id();
        debug!("为面板 {:?} 启动 I/O 处理线程（传统模式）", pane_id);

        // 获取 PTY 读取器
        let reader = pane
            .reader()
            .with_context(|| format!("无法获取面板 {:?} 的读取器", pane_id))?;

        // 创建弱引用避免循环引用
        let weak_pane = Arc::downgrade(&pane);

        // 创建线程间通信通道
        let (data_sender, data_receiver) = bounded::<Vec<u8>>(100);

        // 启动读取线程
        self.spawn_reader_thread(pane_id, reader, data_sender);

        // 启动批处理线程
        self.spawn_batch_processor_thread(weak_pane, data_receiver);

        Ok(())
    }

    /// 启动读取线程
    fn spawn_reader_thread(
        &self,
        pane_id: PaneId,
        mut reader: Box<dyn Read + Send>,
        data_sender: Sender<Vec<u8>>,
    ) {
        let buffer_size = self.config.buffer_size;

        thread::spawn(move || {
            debug!("面板 {:?} 读取线程已启动", pane_id);
            let mut buffer = vec![0u8; buffer_size];

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        // EOF - PTY 已关闭
                        debug!("面板 {:?} PTY 已关闭 (EOF)", pane_id);
                        break;
                    }
                    Ok(bytes_read) => {
                        // 成功读取数据
                        let data = buffer[..bytes_read].to_vec();
                        trace!("面板 {:?} 读取了 {} 字节数据", pane_id, bytes_read);

                        // 发送数据到批处理线程
                        if let Err(e) = data_sender.send(data) {
                            // 批处理线程已关闭，这是正常的清理过程
                            debug!("面板 {:?} 批处理线程已关闭，停止读取: {}", pane_id, e);
                            break;
                        }
                    }
                    Err(e) => {
                        // I/O 错误
                        match e.kind() {
                            std::io::ErrorKind::Interrupted => {
                                // 被中断，继续读取
                                trace!("面板 {:?} 读取被中断，继续", pane_id);
                                continue;
                            }
                            std::io::ErrorKind::WouldBlock => {
                                // 非阻塞模式下没有数据，短暂等待
                                thread::sleep(Duration::from_millis(1));
                                continue;
                            }
                            _ => {
                                // 其他错误，退出读取循环
                                warn!("面板 {:?} 读取错误: {}", pane_id, e);
                                break;
                            }
                        }
                    }
                }
            }

            debug!("面板 {:?} 读取线程已退出", pane_id);
        });
    }

    /// 启动批处理线程
    fn spawn_batch_processor_thread(
        &self,
        weak_pane: Weak<dyn Pane>,
        data_receiver: Receiver<Vec<u8>>,
    ) {
        let batch_size = self.config.batch_size;
        let flush_interval = Duration::from_millis(self.config.flush_interval_ms);
        let notification_sender = self.notification_sender.clone();

        thread::spawn(move || {
            let pane_id = if let Some(pane) = weak_pane.upgrade() {
                let id = pane.pane_id();
                debug!("面板 {:?} 批处理线程已启动", id);
                id
            } else {
                error!("批处理线程启动时面板已被释放");
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
                    debug!("面板 {:?} 已死亡，退出批处理线程", pane_id);
                    break;
                }

                // 尝试接收数据（非阻塞）
                match data_receiver.try_recv() {
                    Ok(data) => {
                        // 收到数据，添加到批处理缓冲区
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
                        // 发送端已断开，退出循环
                        debug!("面板 {:?} 数据通道已断开", pane_id);
                        break;
                    }
                }

                // 检查是否需要刷新批处理数据
                let should_flush = !batch_data.is_empty()
                    && (batch_data.len() >= batch_size || last_flush.elapsed() >= flush_interval);

                if should_flush {
                    // 发送批处理数据
                    let data_to_send = std::mem::take(&mut batch_data);
                    let notification = MuxNotification::PaneOutput {
                        pane_id,
                        data: data_to_send.clone(),
                    };

                    debug!(
                        "🚀 面板 {:?} 发送批处理数据: {} 字节, 内容预览: {:?}",
                        pane_id,
                        data_to_send.len(),
                        String::from_utf8_lossy(
                            &data_to_send[..std::cmp::min(50, data_to_send.len())]
                        )
                    );

                    if let Err(e) = notification_sender.send(notification) {
                        error!("面板 {:?} 发送通知失败: {}", pane_id, e);
                        break;
                    } else {
                        debug!("✅ 面板 {:?} 通知发送成功", pane_id);
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
                    data: batch_data,
                };

                if let Err(e) = notification_sender.send(notification) {
                    error!("面板 {:?} 发送最后的批处理数据失败: {}", pane_id, e);
                }
            }

            // 发送面板退出通知
            let exit_notification = MuxNotification::PaneExited {
                pane_id,
                exit_code: None, // 暂时不获取退出码
            };

            if let Err(e) = notification_sender.send(exit_notification) {
                error!("面板 {:?} 发送退出通知失败: {}", pane_id, e);
            }

            debug!("面板 {:?} 批处理线程已退出", pane_id);
        });
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
