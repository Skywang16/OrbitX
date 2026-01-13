pub mod build;
pub mod index;

pub use build::*;
pub use index::*;

use crate::vector_db::SemanticSearchEngine;
use std::sync::Arc;

/// 向量数据库全局状态（Tauri manage）
pub struct VectorDbState {
    pub search_engine: Arc<SemanticSearchEngine>,
}

impl VectorDbState {
    pub fn new(search_engine: Arc<SemanticSearchEngine>) -> Self {
        Self { search_engine }
    }
}
