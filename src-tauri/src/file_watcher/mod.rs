pub mod commands;
mod config;
mod events;
mod watcher;

pub use config::FileWatcherConfig;
pub use events::{
    now_timestamp_ms, FileWatcherEvent, FileWatcherEventBatch, FsEventType, GitChangeType,
};
pub use watcher::{ObservedFsChange, ObservedFsChangeBatch, UnifiedFileWatcher, WatcherStatus};
