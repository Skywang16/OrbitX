use std::collections::{HashMap, VecDeque};
use std::sync::RwLock;
use tauri::ipc::Channel;

use super::replay;
use super::types::TerminalChannelMessage;
use crate::completion::output_analyzer::OutputAnalyzer;

const MAX_PENDING_CHUNKS: usize = 64;
const MAX_PENDING_BYTES: usize = 64 * 1024;

#[derive(Default)]
struct PendingQueue {
    total_bytes: usize,
    chunks: VecDeque<Vec<u8>>,
}

impl PendingQueue {
    fn push(&mut self, data: &[u8]) {
        let chunk = data.to_vec();
        self.total_bytes += chunk.len();
        self.chunks.push_back(chunk);

        while self.total_bytes > MAX_PENDING_BYTES || self.chunks.len() > MAX_PENDING_CHUNKS {
            if let Some(removed) = self.chunks.pop_front() {
                self.total_bytes = self.total_bytes.saturating_sub(removed.len());
            } else {
                break;
            }
        }
    }

    fn drain(self) -> Vec<Vec<u8>> {
        self.chunks.into_iter().collect()
    }
}

#[derive(Default)]
pub struct TerminalChannelManager {
    channels: RwLock<HashMap<u32, Channel<TerminalChannelMessage>>>,
    pending: RwLock<HashMap<u32, PendingQueue>>,
}

impl TerminalChannelManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, pane_id: u32, channel: Channel<TerminalChannelMessage>) {
        if let Ok(mut map) = self.channels.write() {
            map.insert(pane_id, channel);
        }

        // 检查缓冲区是否太新（<2秒），如果是则跳过 replay（避免新建终端重复输出）
        if !OutputAnalyzer::global().is_pane_buffer_too_new(pane_id) {
            if let Ok(replay_result) = replay::build_replay(pane_id) {
                if let Ok(map) = self.channels.read() {
                    if let Some(ch) = map.get(&pane_id) {
                        for event in replay_result.events {
                            let _ = ch.send(TerminalChannelMessage::Data {
                                pane_id,
                                data: event.data.into_bytes(),
                            });
                        }
                    }
                }
            }
        }

        let buffered = {
            if let Ok(mut pending) = self.pending.write() {
                pending.remove(&pane_id).map(PendingQueue::drain)
            } else {
                None
            }
        };

        if let Some(chunks) = buffered {
            if let Ok(map) = self.channels.read() {
                if let Some(ch) = map.get(&pane_id) {
                    for chunk in chunks {
                        let _ = ch.send(TerminalChannelMessage::Data {
                            pane_id,
                            data: chunk,
                        });
                    }
                }
            }
        }
    }

    pub fn remove(&self, pane_id: u32) {
        if let Ok(mut map) = self.channels.write() {
            map.remove(&pane_id);
        }
        if let Ok(mut pending) = self.pending.write() {
            pending.remove(&pane_id);
        }
    }

    pub fn send_data(&self, pane_id: u32, data: &[u8]) {
        let mut should_buffer = true;
        let mut should_remove = false;

        if let Ok(map) = self.channels.read() {
            if let Some(ch) = map.get(&pane_id) {
                let payload = TerminalChannelMessage::Data {
                    pane_id,
                    data: data.to_vec(),
                };
                if ch.send(payload).is_ok() {
                    should_buffer = false;
                } else {
                    should_remove = true;
                }
            }
        }

        if should_remove {
            if let Ok(mut map) = self.channels.write() {
                map.remove(&pane_id);
            }
        }

        if should_buffer {
            if let Ok(mut pending) = self.pending.write() {
                pending
                    .entry(pane_id)
                    .or_insert_with(PendingQueue::default)
                    .push(data);
            }
        }
    }

    pub fn send_error(&self, pane_id: u32, error: String) {
        if let Ok(map) = self.channels.read() {
            if let Some(ch) = map.get(&pane_id) {
                let _ = ch.send(TerminalChannelMessage::Error { pane_id, error });
            }
        }
    }

    pub fn close(&self, pane_id: u32) {
        if let Ok(map) = self.channels.read() {
            if let Some(ch) = map.get(&pane_id) {
                let _ = ch.send(TerminalChannelMessage::Close { pane_id });
            }
        }
        self.remove(pane_id);
    }
}
