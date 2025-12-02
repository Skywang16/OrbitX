pub mod search_commands;
pub mod index_commands;

pub use search_commands::*;
pub use index_commands::*;

use std::sync::{Arc, OnceLock};
use crate::vector_db::{SemanticSearchEngine, IndexManager, VectorIndex};

/// 向量数据库全局状态（Tauri manage）
pub struct VectorDbState {
    pub search_engine: Arc<SemanticSearchEngine>,
    pub index_manager: Arc<IndexManager>,
    pub vector_index: Arc<VectorIndex>,
}

impl VectorDbState {
    pub fn new(
        search_engine: Arc<SemanticSearchEngine>,
        index_manager: Arc<IndexManager>,
        vector_index: Arc<VectorIndex>,
    ) -> Self {
        Self {
            search_engine,
            index_manager,
            vector_index,
        }
    }
}

/// 全局只读访问（供非 Tauri command 场景使用）
pub struct VectorDbGlobal {
    pub search_engine: Arc<SemanticSearchEngine>,
    pub index_manager: Arc<IndexManager>,
    pub vector_index: Arc<VectorIndex>,
}

static VECTOR_DB_GLOBAL: OnceLock<VectorDbGlobal> = OnceLock::new();

pub fn set_global_state(
    search_engine: Arc<SemanticSearchEngine>,
    index_manager: Arc<IndexManager>,
    vector_index: Arc<VectorIndex>,
) {
    let _ = VECTOR_DB_GLOBAL.set(VectorDbGlobal {
        search_engine,
        index_manager,
        vector_index,
    });
}

pub fn get_global_state() -> Option<&'static VectorDbGlobal> {
    VECTOR_DB_GLOBAL.get()
}
