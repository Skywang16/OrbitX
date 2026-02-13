use crate::{
    mux::{
        error::{IoHandlerError, IoHandlerResult},
        MuxNotification, Pane, PaneId,
    },
    shell::ShellIntegrationManager,
};
use bytes::Bytes;
use crossbeam_channel::Sender;
use std::{
    collections::HashMap,
    io::{self, Read},
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};
use tracing::{error, warn};

pub struct IoHandler {
    buffer_size: usize,
    notification_sender: Sender<MuxNotification>,
    shell_integration: Arc<ShellIntegrationManager>,
    /// 存储每个面板的读取线程句柄
    reader_threads: Arc<RwLock<HashMap<PaneId, thread::JoinHandle<()>>>>,
}

impl IoHandler {
    pub fn new(
        notification_sender: Sender<MuxNotification>,
        shell_integration: Arc<ShellIntegrationManager>,
    ) -> Self {
        Self {
            buffer_size: 8192,
            notification_sender,
            shell_integration,
            reader_threads: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_buffer_size(
        notification_sender: Sender<MuxNotification>,
        shell_integration: Arc<ShellIntegrationManager>,
        buffer_size: usize,
    ) -> Self {
        Self {
            buffer_size,
            notification_sender,
            shell_integration,
            reader_threads: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    pub fn spawn_io_threads(&self, pane: Arc<dyn Pane>) -> IoHandlerResult<()> {
        let pane_id = pane.pane_id();
        let reader = pane.reader().map_err(|err| IoHandlerError::PaneReader {
            reason: format!("Failed to acquire reader for {:?}: {err}", pane_id),
        })?;

        let handle = self.spawn_reader_thread(pane_id, reader, pane);

        // 存储线程句柄
        if let Ok(mut threads) = self.reader_threads.write() {
            threads.insert(pane_id, handle);
        } else {
            warn!("无法存储面板 {:?} 的线程句柄", pane_id);
        }

        Ok(())
    }

    pub fn stop_pane_io(&self, pane_id: PaneId) -> IoHandlerResult<()> {
        if let Ok(mut threads) = self.reader_threads.write() {
            if let Some(handle) = threads.remove(&pane_id) {
                // 使用 thread::spawn 在后台 join，避免阻塞
                thread::spawn(move || {
                    if let Err(e) = handle.join() {
                        warn!("面板 {:?} 的I/O线程结束时发生错误: {:?}", pane_id, e);
                    }
                });
            }
        }
        Ok(())
    }

    pub fn shutdown(&self) -> IoHandlerResult<()> {
        if let Ok(mut threads) = self.reader_threads.write() {
            if threads.is_empty() {
                return Ok(());
            }

            // 使用后台线程批量 join，设置超时并记录结果
            let handles: Vec<_> = threads.drain().collect();
            let (tx, rx) = std::sync::mpsc::channel();

            thread::spawn(move || {
                let start = std::time::Instant::now();
                let mut joined = 0;
                let mut panicked = 0;

                for (pane_id, handle) in handles {
                    if start.elapsed() > Duration::from_secs(2) {
                        warn!("I/O线程关闭超时，放弃等待剩余线程");
                        break;
                    }

                    match handle.join() {
                        Ok(_) => joined += 1,
                        Err(e) => {
                            warn!("面板 {:?} 的I/O线程 panic: {:?}", pane_id, e);
                            panicked += 1;
                        }
                    }
                }

                // 发送结果统计
                let _ = tx.send((joined, panicked));
            });

            // 非阻塞地记录结果
            thread::spawn(move || {
                if rx.recv_timeout(Duration::from_secs(3)).is_err() {
                    warn!("等待I/O线程关闭结果超时");
                }
            });
        }

        Ok(())
    }

    fn spawn_reader_thread(
        &self,
        pane_id: PaneId,
        mut reader: Box<dyn Read + Send>,
        pane: Arc<dyn Pane>,
    ) -> thread::JoinHandle<()> {
        let mut buffer = vec![0u8; self.buffer_size];
        let sender = self.notification_sender.clone();
        let integration = self.shell_integration.clone();

        thread::spawn(move || {
            let mut pending = Vec::new();

            loop {
                // 检查面板是否已死亡
                if pane.is_dead() {
                    break;
                }

                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(len) => {
                        for chunk in decode_utf8_stream(&mut pending, &buffer[..len]) {
                            // Shell事件现在通过broadcast channel发送,不再返回
                            integration.process_output(pane_id, &chunk);

                            let cleaned = integration.strip_osc_sequences(&chunk);

                            if cleaned.is_empty() {
                                continue;
                            }

                            let notification = MuxNotification::PaneOutput {
                                pane_id,
                                data: Bytes::from(cleaned.into_bytes()),
                            };

                            if sender.send(notification).is_err() {
                                return;
                            }
                        }
                    }
                    Err(err) => {
                        if err.kind() == io::ErrorKind::Interrupted {
                            continue;
                        }
                        warn!("面板 {:?} 读线程出错: {}", pane_id, err);
                        break;
                    }
                }
            }

            for chunk in decode_utf8_stream(&mut pending, &[]) {
                integration.process_output(pane_id, &chunk);
                let cleaned = integration.strip_osc_sequences(&chunk);

                if cleaned.is_empty() {
                    continue;
                }

                let notification = MuxNotification::PaneOutput {
                    pane_id,
                    data: Bytes::from(cleaned.into_bytes()),
                };

                if sender.send(notification).is_err() {
                    return;
                }
            }

            let exit_notification = MuxNotification::PaneExited {
                pane_id,
                exit_code: None,
            };

            if let Err(err) = sender.send(exit_notification) {
                error!("面板 {:?} 发送退出通知失败（可能已关闭）: {}", pane_id, err);
            }
        })
    }
}

/// 优化的 UTF-8 流解码函数
///
/// 使用更高效的方式处理字节流到字符串的转换：
/// - 减少 Vec 操作，使用 split_off 替代 drain
/// - 预分配字符串容量
/// - 减少中间分配
fn decode_utf8_stream(pending: &mut Vec<u8>, input: &[u8]) -> Vec<String> {
    if input.is_empty() && pending.is_empty() {
        return Vec::new();
    }

    pending.extend_from_slice(input);

    // 预分配结果向量（通常只有1-2个片段）
    let mut frames = Vec::with_capacity(2);

    loop {
        if pending.is_empty() {
            break;
        }

        match std::str::from_utf8(pending) {
            Ok(valid) => {
                // 整个缓冲区都是有效 UTF-8
                if !valid.is_empty() {
                    frames.push(valid.to_string());
                }
                pending.clear();
                break;
            }
            Err(err) => {
                let valid_up_to = err.valid_up_to();

                if valid_up_to > 0 {
                    // 有部分有效的 UTF-8 数据
                    let valid = unsafe { std::str::from_utf8_unchecked(&pending[..valid_up_to]) };
                    if !valid.is_empty() {
                        frames.push(valid.to_string());
                    }

                    // 使用 split_off 代替 drain，更高效
                    *pending = pending.split_off(valid_up_to);
                    continue;
                }

                // 处理无效字节
                if let Some(error_len) = err.error_len() {
                    // 跳过无效字节
                    let drop_len = error_len.max(1).min(pending.len());
                    *pending = pending.split_off(drop_len);
                    continue;
                }

                // 不完整的 UTF-8 序列，保留在缓冲区中等待更多数据
                break;
            }
        }
    }

    frames
}
