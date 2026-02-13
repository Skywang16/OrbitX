pub mod hybrid_search;
pub mod semantic_search;
mod workspace_index;

use crate::vector_db::core::Language;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchOptions {
    pub top_k: usize,
    pub threshold: f32,
    pub include_snippet: bool,
    pub filter_languages: Vec<Language>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            top_k: 20,
            threshold: 0.3,
            include_snippet: true,
            filter_languages: vec![],
        }
    }
}

pub use hybrid_search::*;
pub use semantic_search::*;
pub(crate) use workspace_index::*;
