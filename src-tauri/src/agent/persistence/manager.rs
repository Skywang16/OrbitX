use std::sync::Arc;

use crate::storage::database::DatabaseManager;

use super::repositories::{
    AgentExecutionRepository, ConversationRepository, ConversationSummaryRepository,
    ExecutionEventRepository, ExecutionMessageRepository, FileContextRepository,
    ToolExecutionRepository,
};

/// Facade that wires all persistence repositories together for the agent backend.
#[derive(Debug)]
pub struct AgentPersistence {
    database: Arc<DatabaseManager>,
    conversations: ConversationRepository,
    conversation_summaries: ConversationSummaryRepository,
    file_context: FileContextRepository,
    agent_executions: AgentExecutionRepository,
    execution_messages: ExecutionMessageRepository,
    tool_executions: ToolExecutionRepository,
    execution_events: ExecutionEventRepository,
}

impl AgentPersistence {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self {
            conversations: ConversationRepository::new(Arc::clone(&database)),
            conversation_summaries: ConversationSummaryRepository::new(Arc::clone(&database)),
            file_context: FileContextRepository::new(Arc::clone(&database)),
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

    pub fn conversations(&self) -> &ConversationRepository {
        &self.conversations
    }

    pub fn conversation_summaries(&self) -> &ConversationSummaryRepository {
        &self.conversation_summaries
    }

    pub fn file_context(&self) -> &FileContextRepository {
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

    /// 确保conversation存在于核心轨中，如果不存在则创建
    pub async fn ensure_conversation_exists(
        &self,
        conversation_id: i64,
    ) -> crate::utils::error::AppResult<()> {
        if !self.conversations.exists(conversation_id).await? {
            let now = super::util::now_timestamp();
            self.conversations
                .create_with_id(conversation_id, None, None, now)
                .await?;
        }
        Ok(())
    }
}
