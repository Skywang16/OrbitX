use std::sync::Arc;

use crate::storage::database::DatabaseManager;

use super::repositories::{
    MessageRepository, SessionRepository, ToolExecutionRepository, WorkspaceRepository,
};

/// Facade that wires all persistence repositories together for the agent backend.
#[derive(Debug)]
pub struct AgentPersistence {
    database: Arc<DatabaseManager>,
    workspaces: WorkspaceRepository,
    sessions: SessionRepository,
    messages: MessageRepository,
    tool_executions: ToolExecutionRepository,
}

impl AgentPersistence {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self {
            tool_executions: ToolExecutionRepository::new(Arc::clone(&database)),
            workspaces: WorkspaceRepository::new(Arc::clone(&database)),
            sessions: SessionRepository::new(Arc::clone(&database)),
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

    pub fn sessions(&self) -> &SessionRepository {
        &self.sessions
    }

    pub fn messages(&self) -> &MessageRepository {
        &self.messages
    }

    pub fn tool_executions(&self) -> &ToolExecutionRepository {
        &self.tool_executions
    }
}
