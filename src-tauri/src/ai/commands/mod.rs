/*!
 * AI功能的Tauri命令接口
 *
 * 统一管理AI相关的命令模块，包括模型管理、对话管理和上下文管理
 */

// 子模块声明
pub mod chat;
pub mod context;
pub mod model;

// 重新导出所有命令函数
pub use chat::*;
pub use context::*;
pub use model::*;

use crate::ai::{AIService, ContextManager};
use crate::storage::cache::UnifiedCache;
use crate::storage::repositories::RepositoryManager;
use crate::terminal::TerminalContextService;
use crate::utils::error::ToTauriResult;

use std::sync::Arc;

/// AI管理器状态
pub struct AIManagerState {
    pub ai_service: Arc<AIService>,
    pub repositories: Arc<RepositoryManager>,
    pub cache: Arc<UnifiedCache>,
    pub terminal_context_service: Arc<TerminalContextService>,
    pub context_manager: Arc<ContextManager>,
}

impl AIManagerState {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        cache: Arc<UnifiedCache>,
        terminal_context_service: Arc<TerminalContextService>,
    ) -> Result<Self, String> {
        let ai_service = Arc::new(AIService::new(repositories.clone(), cache.clone()));
        let context_manager = Arc::new(crate::ai::create_context_manager());

        Ok(Self {
            ai_service,
            repositories,
            cache,
            terminal_context_service,
            context_manager,
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

    pub fn get_context_manager(&self) -> &Arc<ContextManager> {
        &self.context_manager
    }
}
