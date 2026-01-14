use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::agent::error::AgentResult;
use crate::agent::persistence::AgentPersistence;
use crate::agent::types::{Message, MessageStatus};
use crate::storage::DatabaseManager;

use super::config::CompactionConfig;

#[derive(Debug, Clone, Copy)]
pub enum CompactionTrigger {
    Auto,
    Manual,
}

pub struct PreparedCompaction {
    pub summary_job: Option<SummaryJob>,
}

pub struct SummaryJob {
    pub summary_message: Message,
}

pub struct SummaryCompletion {
    pub message_id: i64,
    pub status: MessageStatus,
    pub finished_at: DateTime<Utc>,
    pub duration_ms: i64,
}

pub struct CompactionService {
    _database: Arc<DatabaseManager>,
    _persistence: Arc<AgentPersistence>,
    _config: CompactionConfig,
}

impl CompactionService {
    pub fn new(
        database: Arc<DatabaseManager>,
        persistence: Arc<AgentPersistence>,
        config: CompactionConfig,
    ) -> Self {
        Self {
            _database: database,
            _persistence: persistence,
            _config: config,
        }
    }

    pub async fn prepare_compaction(
        &self,
        _session_id: i64,
        _context_window: u32,
        _trigger: CompactionTrigger,
    ) -> AgentResult<PreparedCompaction> {
        Ok(PreparedCompaction { summary_job: None })
    }

    pub async fn complete_summary_job(
        &self,
        _job: SummaryJob,
        _model_id: &str,
    ) -> AgentResult<SummaryCompletion> {
        Ok(SummaryCompletion {
            message_id: 0,
            status: MessageStatus::Completed,
            finished_at: Utc::now(),
            duration_ms: 0,
        })
    }
}

