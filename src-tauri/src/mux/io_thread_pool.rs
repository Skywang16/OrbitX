//! 优化的I/O线程池实现
//!
//! 使用线程池和异步I/O优化终端I/O处理，减少线程数量和内存使用

use anyhow::{anyhow, Context};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Weak};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tracing::debug;

use crate::mux::{BatchProcessor, BatchProcessorConfig, MuxNotification, Pane, PaneId};
use crate::shell::ShellIntegrationManager;
use crate::utils::error::AppResult;

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
            worker_threads: num_cpus::get(),
            task_queue_capacity: 2000,
            buffer_size: 4096,
            batch_size: 1024,
            flush_interval_ms: 16,
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
    _data_sender: Sender<Vec<u8>>, // 保持通道开启
    reader_handle: Option<JoinHandle<()>>,
}

/// 优化的I/O线程池
pub struct IoThreadPool {
    config: IoThreadPoolConfig,
    task_sender: Sender<IoTask>,
    worker_handles: Arc<std::sync::Mutex<Option<Vec<JoinHandle<()>>>>>,
    stats: Arc<std::sync::Mutex<IoThreadPoolStats>>,
    pane_registry: Arc<std::sync::Mutex<HashMap<PaneId, Weak<dyn Pane>>>>,
    batch_processor: Arc<BatchProcessor>,
}

impl IoThreadPool {
    /// 使用自定义配置创建I/O线程池
    pub fn with_config(
        notification_sender: Sender<MuxNotification>,
        shell_integration: Arc<ShellIntegrationManager>,
        config: IoThreadPoolConfig,
    ) -> Self {
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

        let batch_config = BatchProcessorConfig {
            processor_threads: (config.worker_threads / 2).clamp(2, 4),
            batch_size: config.batch_size,
            flush_interval: Duration::from_millis(config.flush_interval_ms),
            task_queue_capacity: 500,
        };
        let batch_processor = Arc::new(BatchProcessor::new(
            notification_sender.clone(),
            shell_integration.clone(),
            batch_config,
        ));

        // 启动工作线程
        let mut worker_handles = Vec::new();
        for worker_id in 0..config.worker_threads {
            let handle = Self::spawn_worker_thread(
                worker_id,
                task_receiver.clone(),
                config.clone(),
                Arc::clone(&stats),
                Arc::clone(&pane_registry),
                Arc::clone(&batch_processor),
            );
            worker_handles.push(handle);
        }

        Self {
            config,
            task_sender,
            worker_handles: Arc::new(std::sync::Mutex::new(Some(worker_handles))),
            stats,
            pane_registry,
            batch_processor,
        }
    }

    /// 启动面板I/O处理
    pub fn start_pane_io(&self, pane: Arc<dyn Pane>) -> AppResult<()> {
        let pane_id = pane.pane_id();

        let reader = pane
            .reader()
            .with_context(|| format!("无法获取面板 {:?} 的读取器", pane_id))?;

        if let Ok(mut reg) = self.pane_registry.lock() {
            reg.insert(pane_id, Arc::downgrade(&pane));
        }

        let task = IoTask::StartPaneIo { pane, reader };
        self.task_sender
            .send(task)
            .map_err(|e| anyhow!("发送启动任务失败: {}", e))?;

        if let Ok(mut stats) = self.stats.lock() {
            stats.pending_tasks += 1;
        }

        Ok(())
    }

    /// 停止面板I/O处理
    pub fn stop_pane_io(&self, pane_id: PaneId) -> AppResult<()> {
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

        if let Ok(mut stats) = self.stats.lock() {
            stats.pending_tasks += 1;
        }

        Ok(())
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> IoThreadPoolStats {
        let stats = self
            .stats
            .lock()
            .map(|stats| stats.clone())
            .unwrap_or_else(|_| IoThreadPoolStats {
                active_panes: 0,
                worker_threads: self.config.worker_threads,
                pending_tasks: 0,
                total_bytes_processed: 0,
                total_batches_processed: 0,
            });

        stats
    }

    /// 关闭线程池
    pub fn shutdown(&self) -> AppResult<()> {
        if let Ok(reg) = self.pane_registry.lock() {
            for (_id, weak) in reg.iter() {
                if let Some(pane) = weak.upgrade() {
                    pane.mark_dead();
                }
            }
        }

        for _ in 0..self.config.worker_threads {
            let _ = self.task_sender.send(IoTask::Shutdown);
        }

        if let Ok(mut handles_guard) = self.worker_handles.lock() {
            if let Some(handles) = handles_guard.take() {
                for handle in handles {
                    let _ = handle.join();
                }
            }
        }

        let _ = self.batch_processor.shutdown();

        if let Ok(mut reg) = self.pane_registry.lock() {
            reg.clear();
        }
        Ok(())
    }

    /// 启动工作线程
    fn spawn_worker_thread(
        _worker_id: usize,
        task_receiver: Receiver<IoTask>,
        config: IoThreadPoolConfig,
        stats: Arc<std::sync::Mutex<IoThreadPoolStats>>,
        pane_registry: Arc<std::sync::Mutex<HashMap<PaneId, Weak<dyn Pane>>>>,
        batch_processor: Arc<BatchProcessor>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut active_panes: HashMap<PaneId, PaneIoState> = HashMap::new();

            loop {
                match task_receiver.recv() {
                    Ok(IoTask::StartPaneIo { pane, reader }) => {
                        let pane_id = pane.pane_id();

                        if let Some(old_state) = active_panes.remove(&pane_id) {
                            if let Some(handle) = old_state.reader_handle {
                                let _ = handle.join();
                            }
                            if let Ok(mut stats) = stats.lock() {
                                stats.active_panes = stats.active_panes.saturating_sub(1);
                            }
                        }

                        let (data_sender, data_receiver) = bounded::<Vec<u8>>(100);

                        let reader_handle = Self::spawn_reader_thread(
                            pane_id,
                            reader,
                            data_sender.clone(),
                            config.buffer_size,
                        );

                        let pane_state = PaneIoState {
                            _data_sender: data_sender,
                            reader_handle: Some(reader_handle),
                        };
                        active_panes.insert(pane_id, pane_state);

                        if let Ok(mut stats) = stats.lock() {
                            stats.active_panes += 1;
                            stats.pending_tasks = stats.pending_tasks.saturating_sub(1);
                        }

                        if let Err(_) =
                            batch_processor.register_pane(Arc::downgrade(&pane), data_receiver)
                        {
                            if let Some(mut pane_state) = active_panes.remove(&pane_id) {
                                if let Some(handle) = pane_state.reader_handle.take() {
                                    let _ = handle.join();
                                }
                                if let Ok(mut stats) = stats.lock() {
                                    stats.active_panes = stats.active_panes.saturating_sub(1);
                                }
                                if let Ok(mut reg) = pane_registry.lock() {
                                    reg.remove(&pane_id);
                                }
                            }
                        }
                    }
                    Ok(IoTask::StopPaneIo { pane_id }) => {
                        let _ = batch_processor.unregister_pane(pane_id);

                        if let Some(pane_state) = active_panes.remove(&pane_id) {
                            if let Some(handle) = pane_state.reader_handle {
                                let _ = handle.join();
                            }
                            if let Ok(mut stats) = stats.lock() {
                                stats.active_panes = stats.active_panes.saturating_sub(1);
                            }
                            if let Ok(mut reg) = pane_registry.lock() {
                                reg.remove(&pane_id);
                            }
                        }

                        if let Ok(mut stats) = stats.lock() {
                            stats.pending_tasks = stats.pending_tasks.saturating_sub(1);
                        }
                    }
                    Ok(IoTask::Shutdown) => {
                        break;
                    }
                    Err(e) => {
                        // 正常关闭路径：任务通道断开，降低为 debug 以减少测试噪音
                        debug!("工作线程接收任务失败: {}", e);
                        break;
                    }
                }
            }

            for (pane_id, pane_state) in active_panes {
                let _ = batch_processor.unregister_pane(pane_id);

                if let Some(handle) = pane_state.reader_handle {
                    let _ = handle.join();
                }
                if let Ok(mut reg) = pane_registry.lock() {
                    reg.remove(&pane_id);
                }
            }
        })
    }

    /// 启动读取线程
    fn spawn_reader_thread(
        _pane_id: PaneId,
        mut reader: Box<dyn Read + Send>,
        data_sender: Sender<Vec<u8>>,
        buffer_size: usize,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut buffer = vec![0u8; buffer_size];

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        break;
                    }
                    Ok(bytes_read) => {
                        let data = buffer[..bytes_read].to_vec();

                        if let Err(_) = data_sender.send(data) {
                            break;
                        }
                    }
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::Interrupted => {
                            continue;
                        }
                        std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(1));
                            continue;
                        }
                        _ => {
                            break;
                        }
                    },
                }
            }
        })
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
