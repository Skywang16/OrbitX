//! 优化的 I/O 处理模块
//!
//! 使用线程池优化I/O处理，减少线程数量和资源使用

use crate::{
    mux::{
        io_thread_pool::{IoThreadPool, IoThreadPoolConfig},
        pane::Pane,
        MuxNotification, PaneId,
    },
    shell::ShellIntegrationManager,
    utils::error::AppResult,
};
use anyhow::{anyhow, Context};
use bytes::Bytes;
use crossbeam_channel::Sender;
use std::{
    io::{self, Read},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tracing::{debug, error, info, warn};

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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    shell_integration: Arc<ShellIntegrationManager>,
}

impl IoHandler {
    /// 创建新的 I/O 处理器
    pub fn new(
        notification_sender: Sender<MuxNotification>,
        shell_integration: Arc<ShellIntegrationManager>,
    ) -> Self {
        Self::with_config_and_mode(
            notification_sender,
            IoConfig::default(),
            IoMode::default(),
            shell_integration,
        )
    }

    /// 使用自定义配置和模式创建 I/O 处理器
    pub fn with_config_and_mode(
        notification_sender: Sender<MuxNotification>,
        config: IoConfig,
        mode: IoMode,
        shell_integration: Arc<ShellIntegrationManager>,
    ) -> Self {
        let thread_pool = match mode {
            IoMode::ThreadPool => {
                let pool_config = IoThreadPoolConfig {
                    worker_threads: std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(8),
                    task_queue_capacity: 2000,
                    buffer_size: config.buffer_size,
                    batch_size: config.batch_size,
                    flush_interval_ms: config.flush_interval_ms,
                };
                Some(IoThreadPool::with_config(
                    notification_sender.clone(),
                    shell_integration.clone(),
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
            shell_integration,
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
                } else {
                    error!("线程池模式下但线程池未初始化");
                    return Err(anyhow!("线程池未初始化"));
                }
            }
            IoMode::Legacy => {
                self.spawn_io_threads_legacy(pane)?;
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
                }
            }
            IoMode::Legacy => {
                // 传统模式下没有需要特别关闭的资源
            }
        }
        Ok(())
    }

    /// 传统模式下启动 I/O 处理线程
    fn spawn_io_threads_legacy(&self, pane: Arc<dyn Pane>) -> AppResult<()> {
        let pane_id = pane.pane_id();
        let mut reader = pane.reader()?;
        let notification_sender = self.notification_sender.clone();
        let buffer_size = self.config.buffer_size;
        let batch_size = self.config.batch_size;
        let flush_interval = Duration::from_millis(self.config.flush_interval_ms);
        let shell_integration = self.shell_integration.clone();

        thread::spawn(move || {
            let mut buffer = vec![0; buffer_size];
            let mut batch_data = Vec::with_capacity(batch_size);
            let mut last_flush = Instant::now();

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        debug!("面板 {:?} 的读线程收到EOF，正在退出", pane_id);
                        break;
                    }
                    Ok(len) => {
                        batch_data.extend_from_slice(&buffer[..len]);
                    }
                    Err(e) => {
                        if e.kind() != io::ErrorKind::Interrupted {
                            warn!("面板 {:?} 读线程出错: {}", pane_id, e);
                        }
                        break;
                    }
                }

                let should_flush = !batch_data.is_empty()
                    && (batch_data.len() >= batch_size || last_flush.elapsed() >= flush_interval);

                if should_flush {
                    let data_to_send = std::mem::take(&mut batch_data);

                    let data_str = String::from_utf8_lossy(&data_to_send);
                    shell_integration.process_output(pane_id, &data_str);

                    let notification = MuxNotification::PaneOutput {
                        pane_id,
                        data: Bytes::from(data_to_send.clone()),
                    };
                    
                    if let Err(e) = notification_sender.send(notification) {
                        error!("面板 {:?} 发送通知失败: {}", pane_id, e);
                    }
                    last_flush = Instant::now();
                } else if batch_data.is_empty() {
                    thread::sleep(Duration::from_millis(1));
                }
            }

            if !batch_data.is_empty() {
                let data_str = String::from_utf8_lossy(&batch_data);
                shell_integration.process_output(pane_id, &data_str);

                let notification = MuxNotification::PaneOutput {
                    pane_id,
                    data: Bytes::from(batch_data),
                };
                if let Err(e) = notification_sender.send(notification) {
                    error!("面板 {:?} 发送最终通知失败: {}", pane_id, e);
                }
            }
            // 发送退出通知，保持与线程池模式一致
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
            debug!("面板 {:?} 的读线程已终止", pane_id);
        });

        Ok(())
    }
}
