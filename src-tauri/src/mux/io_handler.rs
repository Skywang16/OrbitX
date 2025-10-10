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
    io::{self, Read},
    sync::Arc,
    thread,
};
use tracing::{debug, error, warn};

pub struct IoHandler {
    buffer_size: usize,
    notification_sender: Sender<MuxNotification>,
    shell_integration: Arc<ShellIntegrationManager>,
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

        self.spawn_reader_thread(pane_id, reader);
        Ok(())
    }

    pub fn stop_pane_io(&self, _pane_id: PaneId) -> IoHandlerResult<()> {
        Ok(())
    }

    pub fn shutdown(&self) -> IoHandlerResult<()> {
        Ok(())
    }

    fn spawn_reader_thread(&self, pane_id: PaneId, mut reader: Box<dyn Read + Send>) {
        let mut buffer = vec![0u8; self.buffer_size];
        let sender = self.notification_sender.clone();
        let integration = self.shell_integration.clone();

        thread::spawn(move || {
            let mut pending = Vec::new();

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(len) => {
                        for chunk in decode_utf8_stream(&mut pending, &buffer[..len]) {
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
                                debug!("面板 {:?} 输出通知发送失败", pane_id);
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

                if let Err(err) = sender.send(notification) {
                    debug!("面板 {:?} 输出通知发送失败（终止前刷新）: {}", pane_id, err);
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
        });
    }
}

fn decode_utf8_stream(pending: &mut Vec<u8>, input: &[u8]) -> Vec<String> {
    pending.extend_from_slice(input);
    let mut frames = Vec::new();

    loop {
        match std::str::from_utf8(pending) {
            Ok(valid) => {
                if !valid.is_empty() {
                    frames.push(valid.to_string());
                }
                pending.clear();
                break;
            }
            Err(err) => {
                let valid_up_to = err.valid_up_to();

                if valid_up_to > 0 {
                    let valid = unsafe { std::str::from_utf8_unchecked(&pending[..valid_up_to]) };
                    if !valid.is_empty() {
                        frames.push(valid.to_string());
                    }
                    pending.drain(0..valid_up_to);
                    continue;
                }

                if let Some(error_len) = err.error_len() {
                    let drop_len = error_len.max(1);
                    pending.drain(0..drop_len);
                    continue;
                }

                break;
            }
        }
    }

    frames
}
