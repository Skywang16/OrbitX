use super::SearchOptions;
use crate::vector_db::core::{Result, SearchResult, VectorDbConfig};
use crate::vector_db::embedding::Embedder;
use crate::vector_db::index::VectorIndex;
use crate::vector_db::storage::IndexManager;
use std::path::Path;
use std::sync::Arc;

pub struct SemanticSearchEngine {
    embedder: Arc<dyn Embedder>,
    config: VectorDbConfig,
}

impl SemanticSearchEngine {
    pub fn new(embedder: Arc<dyn Embedder>, config: VectorDbConfig) -> Self {
        Self { embedder, config }
    }

    pub fn embedder(&self) -> Arc<dyn Embedder> {
        self.embedder.clone()
    }

    pub fn config(&self) -> &VectorDbConfig {
        &self.config
    }

    pub async fn search_in_workspace(
        &self,
        workspace_root: &Path,
        query: &str,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let index_manager = IndexManager::new(workspace_root, self.config.clone())?;
        let chunk_metadata_vec = index_manager.get_all_chunk_metadata();
        if chunk_metadata_vec.is_empty() {
            return Ok(Vec::new());
        }

        let chunk_metadata: std::collections::HashMap<_, _> =
            chunk_metadata_vec.into_iter().collect();
        let vector_index = VectorIndex::load(
            index_manager.store(),
            &chunk_metadata,
            self.config.embedding.dimension,
        )
        .await?;

        let query_embedding = self.embedder.embed(&[query]).await?;
        let query_vec = &query_embedding[0];

        let results = vector_index.search(
            query_vec,
            options.top_k,
            self.config.similarity_threshold.max(options.threshold),
        )?;

        let mut search_results = Vec::with_capacity(results.len());
        for (chunk_id, score) in results {
            if let Some(metadata) = vector_index.get_chunk_metadata(&chunk_id) {
                search_results.push(SearchResult::new(
                    metadata.file_path.clone(),
                    metadata.span.clone(),
                    score,
                    format!("Chunk {:?}", metadata.chunk_type),
                    None,
                    Some(metadata.chunk_type),
                ));
            }
        }

        Ok(search_results)
    }
}
