//! Checkpoint 系统模块
//!
//! 提供类似 Git 的文件状态快照功能，支持：
//! - 自动创建 checkpoint（用户发消息时）
//! - 查看 checkpoint 历史
//! - 回滚到任意历史状态
//! - 文件差异对比

mod blob_store;
pub mod commands;
mod models;
mod service;
mod storage;

pub use blob_store::BlobStore;
pub use commands::CheckpointState;
pub use models::{
    Checkpoint, CheckpointError, CheckpointResult, CheckpointSummary, FileChangeType, FileDiff,
    FileSnapshot, NewCheckpoint, NewFileSnapshot, RollbackResult,
};
pub use service::CheckpointService;
pub use storage::CheckpointStorage;
