use std::sync::Arc;

use super::channel_manager::TerminalChannelManager;

pub struct TerminalChannelState {
    pub manager: Arc<TerminalChannelManager>,
}

impl Default for TerminalChannelState {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalChannelState {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(TerminalChannelManager::new()),
        }
    }
}
