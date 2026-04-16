use std::sync::Arc;

use crate::agent::rollout::{RolloutRecorder, ThreadRepository};
use crate::storage::database::DatabaseManager;

use super::repositories::{MessageRepository, WorkspaceRepository};

/// Facade that wires all persistence repositories together for the agent backend.
#[derive(Debug)]
pub struct AgentPersistence {
    database: Arc<DatabaseManager>,
    threads: ThreadRepository,
    rollout_recorder: RolloutRecorder,
    workspaces: WorkspaceRepository,
    messages: MessageRepository,
}

impl AgentPersistence {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self {
            threads: ThreadRepository::new(Arc::clone(&database)),
            rollout_recorder: RolloutRecorder::new(Arc::clone(&database)),
            workspaces: WorkspaceRepository::new(Arc::clone(&database)),
            messages: MessageRepository::new(Arc::clone(&database)),
            database,
        }
    }

    pub fn database(&self) -> &DatabaseManager {
        &self.database
    }

    pub fn workspaces(&self) -> &WorkspaceRepository {
        &self.workspaces
    }

    pub fn threads(&self) -> &ThreadRepository {
        &self.threads
    }

    pub fn rollout_recorder(&self) -> &RolloutRecorder {
        &self.rollout_recorder
    }

    pub fn messages(&self) -> &MessageRepository {
        &self.messages
    }
}
