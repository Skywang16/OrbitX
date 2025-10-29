pub mod model;

pub use model::*;

use crate::ai::AIService;
use crate::storage::cache::UnifiedCache;
use crate::storage::DatabaseManager;
use crate::terminal::TerminalContextService;
use std::sync::Arc;

pub struct AIManagerState {
    pub ai_service: Arc<AIService>,
    pub database: Arc<DatabaseManager>,
    pub cache: Arc<UnifiedCache>,
    pub terminal_context_service: Arc<TerminalContextService>,
}

impl AIManagerState {
    pub fn new(
        database: Arc<DatabaseManager>,
        cache: Arc<UnifiedCache>,
        terminal_context_service: Arc<TerminalContextService>,
    ) -> Result<Self, String> {
        let ai_service = Arc::new(AIService::new(database.clone()));

        Ok(Self {
            ai_service,
            database,
            cache,
            terminal_context_service,
        })
    }

    pub async fn initialize(&self) -> Result<(), String> {
        self.ai_service
            .initialize()
            .await
            .map_err(|err| err.to_string())
    }

    pub fn database(&self) -> &Arc<DatabaseManager> {
        &self.database
    }

    pub fn get_terminal_context_service(&self) -> &Arc<TerminalContextService> {
        &self.terminal_context_service
    }
}
