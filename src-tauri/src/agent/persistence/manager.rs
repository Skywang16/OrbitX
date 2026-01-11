use std::sync::Arc;

use crate::storage::database::DatabaseManager;

use super::repositories::{
    AgentExecutionRepository, ExecutionEventRepository, ExecutionMessageRepository,
    MessageRepository, SessionRepository, ToolExecutionRepository, WorkspaceFileContextRepository,
    WorkspaceRepository,
};
use super::ToolOutputRepository;

/// Facade that wires all persistence repositories together for the agent backend.
#[derive(Debug)]
pub struct AgentPersistence {
    database: Arc<DatabaseManager>,
    workspaces: WorkspaceRepository,
    sessions: SessionRepository,
    messages: MessageRepository,
    tool_outputs: ToolOutputRepository,
    file_context: WorkspaceFileContextRepository,
    agent_executions: AgentExecutionRepository,
    execution_messages: ExecutionMessageRepository,
    tool_executions: ToolExecutionRepository,
    execution_events: ExecutionEventRepository,
}

impl AgentPersistence {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self {
            workspaces: WorkspaceRepository::new(Arc::clone(&database)),
            sessions: SessionRepository::new(Arc::clone(&database)),
            messages: MessageRepository::new(Arc::clone(&database)),
            tool_outputs: ToolOutputRepository::new(Arc::clone(&database)),
            file_context: WorkspaceFileContextRepository::new(Arc::clone(&database)),
            agent_executions: AgentExecutionRepository::new(Arc::clone(&database)),
            execution_messages: ExecutionMessageRepository::new(Arc::clone(&database)),
            tool_executions: ToolExecutionRepository::new(Arc::clone(&database)),
            execution_events: ExecutionEventRepository::new(Arc::clone(&database)),
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

    pub fn tool_outputs(&self) -> &ToolOutputRepository {
        &self.tool_outputs
    }

    pub fn file_context(&self) -> &WorkspaceFileContextRepository {
        &self.file_context
    }

    pub fn agent_executions(&self) -> &AgentExecutionRepository {
        &self.agent_executions
    }

    pub fn execution_messages(&self) -> &ExecutionMessageRepository {
        &self.execution_messages
    }

    pub fn tool_executions(&self) -> &ToolExecutionRepository {
        &self.tool_executions
    }

    pub fn execution_events(&self) -> &ExecutionEventRepository {
        &self.execution_events
    }
}
