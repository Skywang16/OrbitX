use std::sync::Arc;

use super::channel_manager::TerminalChannelManager;

pub struct TerminalChannelState {
  pub manager: Arc<TerminalChannelManager>,
}

impl TerminalChannelState {
  pub fn new() -> Self {
    Self {
      manager: Arc::new(TerminalChannelManager::new()),
    }
  }
}
