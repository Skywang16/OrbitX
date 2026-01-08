use super::SearchOptions;
use crate::vector_db::core::{Result, SearchResult, VectorDbConfig};
use crate::vector_db::embedding::Embedder;
use crate::vector_db::search::WorkspaceIndexCache;
use crate::vector_db::storage::IndexManager;
use std::path::Path;
use std::sync::Arc;

pub struct SemanticSearchEngine {
    embedder: Arc<dyn Embedder>,
    config: VectorDbConfig,
    index_cache: WorkspaceIndexCache,
}

impl SemanticSearchEngine {
    pub fn new(embedder: Arc<dyn Embedder>, config: VectorDbConfig) -> Self {
        // Keep memory bounded: cache only a few workspaces and cap total vector bytes.
        let index_cache = WorkspaceIndexCache::new(3, 256 * 1024 * 1024);
        Self {
            embedder,
            config,
            index_cache,
        }
    }

    pub fn embedder(&self) -> Arc<dyn Embedder> {
        self.embedder.clone()
    }

    pub fn config(&self) -> &VectorDbConfig {
        &self.config
    }

    pub fn invalidate_workspace_index(&self, workspace_root: &Path) {
        self.index_cache.invalidate(workspace_root);
    }

    pub async fn search_in_workspace(
        &self,
        workspace_root: &Path,
        query: &str,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let index_manager = IndexManager::new(workspace_root, self.config.clone())?;
        if index_manager.get_status().total_chunks == 0 {
            return Ok(Vec::new());
        }

        let cached = self
            .index_cache
            .get_or_build(workspace_root, &self.config)
            .await?;

        let query_embedding = self.embedder.embed(&[query]).await?;
        let query_vec = &query_embedding[0];

        let threshold = self.config.similarity_threshold.max(options.threshold);
        let hits = cached.search(query_vec, options.top_k, threshold)?;

        let mut search_results = Vec::with_capacity(hits.len());
        for (internal_idx, score) in hits {
            if let Some((_chunk_id, metadata)) = cached.chunk_meta_by_internal(internal_idx) {
                search_results.push(SearchResult::new(
                    metadata.file_path.clone(),
                    metadata.span.clone(),
                    score,
                    format!("Chunk {:?}", metadata.chunk_type),
                    None,
                    Some(metadata.chunk_type.clone()),
                ));
            }
        }

        Ok(search_results)
    }
}
