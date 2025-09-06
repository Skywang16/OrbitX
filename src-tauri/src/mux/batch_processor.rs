//! 独立的批处理调度器
//!
//! 将批处理逻辑从工作线程中分离，实现真正的多终端并发支持

use anyhow::anyhow;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tracing::debug;

use crate::mux::{MuxNotification, Pane, PaneId};
use crate::shell::ShellIntegrationManager;
use crate::utils::error::AppResult;
use bytes::Bytes;

/// 批处理任务类型
#[derive(Debug)]
pub enum BatchTask {
    /// 注册新的面板进行批处理
    RegisterPane {
        pane: Weak<dyn Pane>,
        data_receiver: Receiver<Vec<u8>>,
    },
    /// 注销面板的批处理
    UnregisterPane { pane_id: PaneId },
    /// 关闭批处理器
    Shutdown,
}

/// 面板批处理状态
#[derive(Debug)]
struct PaneBatchState {
    pane: Weak<dyn Pane>,
    data_receiver: Receiver<Vec<u8>>,
    batch_data: Vec<u8>,
    last_flush: Instant,
}

/// 批处理器配置
#[derive(Debug, Clone)]
pub struct BatchProcessorConfig {
    /// 批处理线程数量
    pub processor_threads: usize,
    /// 批处理大小阈值（字节）
    pub batch_size: usize,
    /// 批处理时间阈值
    pub flush_interval: Duration,
    /// 任务队列容量
    pub task_queue_capacity: usize,
}

impl Default for BatchProcessorConfig {
    fn default() -> Self {
        Self {
            processor_threads: (num_cpus::get() / 2).clamp(2, 4), // 2-4个批处理线程
            batch_size: 1024,
            flush_interval: Duration::from_millis(16), // ~60 FPS
            task_queue_capacity: 500,
        }
    }
}

/// 独立的批处理调度器
pub struct BatchProcessor {
    config: BatchProcessorConfig,
    task_sender: Sender<BatchTask>,
    processor_handles: Arc<std::sync::Mutex<Option<Vec<JoinHandle<()>>>>>,
    #[allow(dead_code)]
    notification_sender: Sender<MuxNotification>,
    #[allow(dead_code)]
    shell_integration: Arc<ShellIntegrationManager>,
}

impl BatchProcessor {
    /// 创建新的批处理器
    pub fn new(
        notification_sender: Sender<MuxNotification>,
        shell_integration: Arc<ShellIntegrationManager>,
        config: BatchProcessorConfig,
    ) -> Self {
        let (task_sender, task_receiver) = bounded(config.task_queue_capacity);

        let mut processor_handles = Vec::new();
        for processor_id in 0..config.processor_threads {
            let handle = Self::spawn_processor_thread(
                processor_id,
                task_receiver.clone(),
                notification_sender.clone(),
                config.clone(),
                shell_integration.clone(),
            );
            processor_handles.push(handle);
        }

        Self {
            config,
            task_sender,
            processor_handles: Arc::new(std::sync::Mutex::new(Some(processor_handles))),
            notification_sender,
            shell_integration,
        }
    }

    /// 注册面板进行批处理
    pub fn register_pane(
        &self,
        pane: Weak<dyn Pane>,
        data_receiver: Receiver<Vec<u8>>,
    ) -> AppResult<()> {
        let _pane_id = if let Some(p) = pane.upgrade() {
            p.pane_id()
        } else {
            return Err(anyhow!("面板已被释放"));
        };

        let task = BatchTask::RegisterPane {
            pane,
            data_receiver,
        };
        self.task_sender
            .send(task)
            .map_err(|e| anyhow!("发送注册任务失败: {}", e))?;

        Ok(())
    }

    /// 注销面板的批处理
    pub fn unregister_pane(&self, pane_id: PaneId) -> AppResult<()> {
        let task = BatchTask::UnregisterPane { pane_id };
        self.task_sender
            .send(task)
            .map_err(|e| anyhow!("发送注销任务失败: {}", e))?;

        Ok(())
    }

    /// 关闭批处理器
    pub fn shutdown(&self) -> AppResult<()> {
        for _ in 0..self.config.processor_threads {
            let _ = self.task_sender.send(BatchTask::Shutdown);
        }

        if let Ok(mut handles_guard) = self.processor_handles.lock() {
            if let Some(handles) = handles_guard.take() {
                for handle in handles {
                    let _ = handle.join();
                }
            }
        }
        Ok(())
    }

    /// 启动批处理线程
    fn spawn_processor_thread(
        processor_id: usize,
        task_receiver: Receiver<BatchTask>,
        notification_sender: Sender<MuxNotification>,
        config: BatchProcessorConfig,
        shell_integration: Arc<ShellIntegrationManager>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut active_panes: HashMap<PaneId, PaneBatchState> = HashMap::new();

            loop {
                // 处理新的任务（非阻塞）
                match task_receiver.try_recv() {
                    Ok(BatchTask::RegisterPane {
                        pane,
                        data_receiver,
                    }) => {
                        if let Some(pane_arc) = pane.upgrade() {
                            let pane_id = pane_arc.pane_id();

                            let state = PaneBatchState {
                                pane,
                                data_receiver,
                                batch_data: Vec::new(),
                                last_flush: Instant::now(),
                            };
                            active_panes.insert(pane_id, state);
                        }
                    }
                    Ok(BatchTask::UnregisterPane { pane_id }) => {
                        if let Some(state) = active_panes.remove(&pane_id) {
                            Self::flush_pane_data(
                                pane_id,
                                state.batch_data,
                                &notification_sender,
                                &shell_integration,
                            );
                        }
                    }
                    Ok(BatchTask::Shutdown) => {
                        break;
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => {
                        // 没有新任务，继续处理现有面板
                    }
                    Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        // 正常关闭路径：任务通道断开，使用 debug 降低噪音
                        debug!("批处理线程 {} 任务通道已断开", processor_id);
                        break;
                    }
                }

                // 处理所有活跃面板的数据
                let mut panes_to_remove = Vec::new();
                for (pane_id, state) in active_panes.iter_mut() {
                    // 检查面板是否还存活
                    let pane_alive = if let Some(pane) = state.pane.upgrade() {
                        !pane.is_dead()
                    } else {
                        false
                    };

                    if !pane_alive {
                        panes_to_remove.push(*pane_id);
                        continue;
                    }

                    // 尝试接收数据（非阻塞）
                    match state.data_receiver.try_recv() {
                        Ok(data) => {
                            state.batch_data.extend_from_slice(&data);
                        }
                        Err(crossbeam_channel::TryRecvError::Empty) => {
                            // 没有新数据
                        }
                        Err(crossbeam_channel::TryRecvError::Disconnected) => {
                            // 数据通道已断开，标记为移除
                            panes_to_remove.push(*pane_id);
                            continue;
                        }
                    }

                    // 检查是否需要刷新
                    let should_flush = !state.batch_data.is_empty()
                        && (state.batch_data.len() >= config.batch_size
                            || state.last_flush.elapsed() >= config.flush_interval);

                    if should_flush {
                        let data_to_send = std::mem::take(&mut state.batch_data);
                        Self::flush_pane_data(
                            *pane_id,
                            data_to_send,
                            &notification_sender,
                            &shell_integration,
                        );
                        state.last_flush = Instant::now();
                    }
                }

                for pane_id in panes_to_remove {
                    if let Some(state) = active_panes.remove(&pane_id) {
                        Self::flush_pane_data(
                            pane_id,
                            state.batch_data,
                            &notification_sender,
                            &shell_integration,
                        );
                        let exit_notification = MuxNotification::PaneExited {
                            pane_id,
                            exit_code: None,
                        };
                        let _ = notification_sender.send(exit_notification);
                    }
                }

                // 短暂休眠，避免忙等待
                if active_panes.is_empty() {
                    thread::sleep(Duration::from_millis(10));
                } else {
                    thread::sleep(Duration::from_millis(1));
                }
            }

            for (pane_id, state) in active_panes {
                Self::flush_pane_data(
                    pane_id,
                    state.batch_data,
                    &notification_sender,
                    &shell_integration,
                );
            }
        })
    }

    /// 刷新面板数据
    fn flush_pane_data(
        pane_id: PaneId,
        data: Vec<u8>,
        notification_sender: &Sender<MuxNotification>,
        shell_integration: &Arc<ShellIntegrationManager>,
    ) {
        if data.is_empty() {
            return;
        }

        let data_str = String::from_utf8_lossy(&data);
        // 将输出交给 Shell Integration 进行处理（用于解析 CWD/命令等上下文）
        shell_integration.process_output(pane_id, &data_str);
        let notification = MuxNotification::PaneOutput {
            pane_id,
            data: Bytes::from(data),
        };
        if let Err(e) = notification_sender.send(notification) {
            tracing::error!(
                "BatchProcessor发送PaneOutput通知失败: pane_id={:?}, error={}",
                pane_id,
                e
            );
        }
    }
}

// 添加num_cpus依赖的简单实现
mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    }
}
