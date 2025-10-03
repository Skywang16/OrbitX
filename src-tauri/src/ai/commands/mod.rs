pub mod model;

pub use model::*;

use crate::ai::AIService;
use crate::storage::cache::UnifiedCache;
use crate::storage::repositories::RepositoryManager;
use crate::terminal::TerminalContextService;
use crate::utils::error::ToTauriResult;

use std::sync::Arc;

pub struct AIManagerState {
    pub ai_service: Arc<AIService>,
    pub repositories: Arc<RepositoryManager>,
    pub cache: Arc<UnifiedCache>,
    pub terminal_context_service: Arc<TerminalContextService>,
}

impl AIManagerState {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        cache: Arc<UnifiedCache>,
        terminal_context_service: Arc<TerminalContextService>,
    ) -> Result<Self, String> {
        let ai_service = Arc::new(AIService::new(repositories.clone()));

        Ok(Self {
            ai_service,
            repositories,
            cache,
            terminal_context_service,
        })
    }

    pub async fn initialize(&self) -> Result<(), String> {
        self.ai_service.initialize().await.to_tauri()
    }

    pub fn repositories(&self) -> &Arc<RepositoryManager> {
        &self.repositories
    }

    pub fn get_terminal_context_service(&self) -> &Arc<TerminalContextService> {
        &self.terminal_context_service
    }
}
