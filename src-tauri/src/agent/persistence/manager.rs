use std::sync::Arc;

use crate::storage::database::DatabaseManager;

use super::repositories::{
    AgentExecutionRepository, ExecutionEventRepository, ExecutionMessageRepository,
    MessageRepository, SessionRepository, SessionSummaryRepository, ToolExecutionRepository,
    WorkspaceFileContextRepository, WorkspaceRepository,
};

/// Facade that wires all persistence repositories together for the agent backend.
#[derive(Debug)]
pub struct AgentPersistence {
    database: Arc<DatabaseManager>,
    workspaces: WorkspaceRepository,
    sessions: SessionRepository,
    messages: MessageRepository,
    session_summaries: SessionSummaryRepository,
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
            session_summaries: SessionSummaryRepository::new(Arc::clone(&database)),
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

    pub fn session_summaries(&self) -> &SessionSummaryRepository {
        &self.session_summaries
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
