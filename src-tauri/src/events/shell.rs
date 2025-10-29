use std::sync::Arc;
use crate::shell::CommandInfo;

#[derive(Debug, Clone)]
pub enum ShellEvent {
    CwdChanged { new_cwd: String },
    CommandEvent { command: Arc<CommandInfo> },
    TitleChanged { new_title: String },
    NodeVersionChanged { version: String },
}
