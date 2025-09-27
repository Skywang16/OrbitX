// Memory & context compaction utilities for Agent module
// Phase 1: placeholder; future: extract message compaction strategies from executor into here.

pub mod message_compaction;
pub use message_compaction::compress_messages;
