use super::SearchOptions;
use crate::vector_db::core::{Result, SearchResult, VectorDbConfig};
use crate::vector_db::embedding::Embedder;
use crate::vector_db::index::VectorIndex;
use crate::vector_db::storage::IndexManager;
use std::sync::Arc;

pub struct SemanticSearchEngine {
    _index_manager: Arc<IndexManager>,
    vector_index: Arc<VectorIndex>,
    embedder: Arc<dyn Embedder>,
    config: VectorDbConfig,
}

impl SemanticSearchEngine {
    pub fn new(
        index_manager: Arc<IndexManager>,
        vector_index: Arc<VectorIndex>,
        embedder: Arc<dyn Embedder>,
        config: VectorDbConfig,
    ) -> Self {
        Self {
            _index_manager: index_manager,
            vector_index,
            embedder,
            config,
        }
    }

    pub fn embedder(&self) -> Arc<dyn Embedder> {
        self.embedder.clone()
    }

    pub fn config(&self) -> &VectorDbConfig {
        &self.config
    }

    pub async fn search(&self, query: &str, options: SearchOptions) -> Result<Vec<SearchResult>> {
        // 1. 生成查询向量（使用引用，零克隆）
        let query_embedding = self.embedder.embed(&[query]).await?;
        let query_vec = &query_embedding[0];

        // 2. 向量搜索
        let results = self.vector_index.search(
            query_vec,
            options.top_k,
            self.config.similarity_threshold.max(options.threshold),
        )?;

        // 3. 构建搜索结果（预分配容量）
        let mut search_results = Vec::with_capacity(results.len());
        for (chunk_id, score) in results {
            if let Some(metadata) = self.vector_index.get_chunk_metadata(&chunk_id) {
                // 这里仅构造一个简单的预览占位符。后续可从存储读取真实片段预览。
                let preview = format!("Preview of chunk {:?}", metadata.chunk_type);
                search_results.push(SearchResult::new(
                    metadata.file_path.clone(),
                    metadata.span.clone(),
                    score,
                    preview,
                    None,
                    Some(metadata.chunk_type),
                ));
            }
        }

        Ok(search_results)
    }
}
