use std::collections::HashMap;
use std::sync::RwLock;
use tauri::ipc::Channel;

use super::types::TerminalChannelMessage;
use crate::terminal::TerminalScrollback;

#[derive(Default)]
pub struct TerminalChannelManager {
    channels: RwLock<HashMap<u32, Channel<TerminalChannelMessage>>>,
}

impl TerminalChannelManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, pane_id: u32, channel: Channel<TerminalChannelMessage>) {
        if let Ok(mut map) = self.channels.write() {
            map.insert(pane_id, channel);
        }

        // Always replay scrollback on subscribe. The UI terminal is frequently mounted/unmounted
        // when switching tabs, so it needs a durable source of history.
        let replay = TerminalScrollback::global().get_bytes(pane_id);
        if replay.is_empty() {
            return;
        }

        if let Ok(map) = self.channels.read() {
            if let Some(ch) = map.get(&pane_id) {
                let _ = ch.send(TerminalChannelMessage::Data {
                    pane_id,
                    data: replay,
                });
            }
        }
    }

    pub fn remove(&self, pane_id: u32) {
        if let Ok(mut map) = self.channels.write() {
            map.remove(&pane_id);
        }
    }

    pub fn send_data(&self, pane_id: u32, data: &[u8]) {
        let mut should_remove = false;

        if let Ok(map) = self.channels.read() {
            if let Some(ch) = map.get(&pane_id) {
                let payload = TerminalChannelMessage::Data {
                    pane_id,
                    data: data.to_vec(),
                };
                if ch.send(payload).is_ok() {
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
