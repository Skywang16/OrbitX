pub mod index_commands;
pub mod search_commands;

pub use index_commands::*;
pub use search_commands::*;

use crate::vector_db::SemanticSearchEngine;
use std::sync::{Arc, OnceLock};

/// 向量数据库全局状态（Tauri manage）
pub struct VectorDbState {
    pub search_engine: Arc<SemanticSearchEngine>,
}

impl VectorDbState {
    pub fn new(search_engine: Arc<SemanticSearchEngine>) -> Self {
        Self { search_engine }
    }
}

/// 全局只读访问（供非 Tauri command 场景使用）
pub struct VectorDbGlobal {
    pub search_engine: Arc<SemanticSearchEngine>,
}

static VECTOR_DB_GLOBAL: OnceLock<VectorDbGlobal> = OnceLock::new();

pub fn set_global_state(search_engine: Arc<SemanticSearchEngine>) {
    let _ = VECTOR_DB_GLOBAL.set(VectorDbGlobal { search_engine });
}

pub fn get_global_state() -> Option<&'static VectorDbGlobal> {
    VECTOR_DB_GLOBAL.get()
}
