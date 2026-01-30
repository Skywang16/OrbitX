//! Terminal scrollback buffer.
//!
//! This buffer is used for UI replay when a terminal tab is mounted/unmounted.
//! It is intentionally separate from the completion `OutputAnalyzer`, which may
//! clear buffers based on command boundaries.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::mux::ConfigManager;

const TOO_NEW_WINDOW: Duration = Duration::from_secs(2);

struct ScrollbackEntry {
    bytes: Vec<u8>,
    created_at: Instant,
}

impl ScrollbackEntry {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            created_at: Instant::now(),
        }
    }

    fn is_too_new(&self) -> bool {
        self.created_at.elapsed() < TOO_NEW_WINDOW
    }

    fn append(&mut self, data: &[u8], max_size: usize, keep_size: usize) {
        if data.is_empty() {
            return;
        }

        self.bytes.extend_from_slice(data);

        if self.bytes.len() <= max_size {
            return;
        }

        let keep = keep_size.min(max_size).max(1);
        if self.bytes.len() <= keep {
            return;
        }

        let start = self.bytes.len().saturating_sub(keep);
        self.bytes.drain(..start);
    }
}

/// A process-wide, in-memory scrollback buffer keyed by `pane_id`.
pub struct TerminalScrollback {
    inner: Mutex<HashMap<u32, ScrollbackEntry>>,
}

static GLOBAL_SCROLLBACK: OnceLock<Arc<TerminalScrollback>> = OnceLock::new();

impl TerminalScrollback {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn global() -> &'static Arc<TerminalScrollback> {
        GLOBAL_SCROLLBACK.get_or_init(|| Arc::new(TerminalScrollback::new()))
    }

    pub fn append(&self, pane_id: u32, data: &[u8]) {
        let config = ConfigManager::config_get();
        let max_size = config.buffer.max_size;
        let keep_size = config.buffer.keep_size;

        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let entry = inner.entry(pane_id).or_insert_with(ScrollbackEntry::new);
        entry.append(data, max_size, keep_size);
    }

    pub fn get_bytes(&self, pane_id: u32) -> Vec<u8> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner
            .get(&pane_id)
            .map(|entry| entry.bytes.clone())
            .unwrap_or_default()
    }

    pub fn get_text_lossy(&self, pane_id: u32) -> String {
        String::from_utf8_lossy(&self.get_bytes(pane_id)).to_string()
    }

    pub fn is_too_new(&self, pane_id: u32) -> bool {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.get(&pane_id).is_some_and(|entry| entry.is_too_new())
    }

    pub fn remove(&self, pane_id: u32) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.remove(&pane_id);
    }
}

impl Default for TerminalScrollback {
    fn default() -> Self {
        Self::new()
    }
}

