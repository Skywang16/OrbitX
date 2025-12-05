use crate::utils::TauriApiResult;
use crate::vector_db::commands::VectorDbState;
use crate::vector_db::core::SearchResult;
use crate::vector_db::search::SearchOptions;
use crate::vector_db::storage::IndexStatus;
use crate::{api_error, api_success};
use std::path::PathBuf;
use tauri::State;
use tracing::warn;

/// 语义搜索命令
#[tauri::command]
pub async fn semantic_search(
    query: String,
    path: String,
    options: Option<SearchOptions>,
    state: State<'_, VectorDbState>,
) -> TauriApiResult<Vec<SearchResult>> {
    let workspace_path = PathBuf::from(&path);
    let config = state.search_engine.config().clone();
    let default_options = SearchOptions::default();
    let search_options = options.unwrap_or(default_options);

    // 为工作区创建 IndexManager 并加载向量索引
    let index_manager =
        match crate::vector_db::storage::IndexManager::new(&workspace_path, config.clone()) {
            Ok(manager) => manager,
            Err(e) => {
                warn!(error = %e, path = %path, "创建索引管理器失败");
                return Ok(api_error!("vector_db.search_failed"));
            }
        };

    // 获取所有 chunk 元数据并加载向量到内存
    let chunk_metadata_vec = index_manager.get_all_chunk_metadata();
    if chunk_metadata_vec.is_empty() {
        return Ok(api_success!(vec![]));
    }

    // 转换为HashMap
    let chunk_metadata: std::collections::HashMap<_, _> = chunk_metadata_vec.into_iter().collect();

    let vector_index = match crate::vector_db::index::VectorIndex::load(
        index_manager.store(),
        &chunk_metadata,
        config.embedding.dimension,
    )
    .await
    {
        Ok(index) => index,
        Err(e) => {
            warn!(error = %e, "加载向量索引失败");
            return Ok(api_error!("vector_db.search_failed"));
        }
    };

    // 生成查询向量
    let embedder = state.search_engine.embedder();
    let query_embedding = match embedder.embed(&[&query]).await {
        Ok(embeddings) if !embeddings.is_empty() => embeddings.into_iter().next().unwrap(),
        Ok(_) => {
            warn!(query = %query, "生成查询向量失败: 空结果");
            return Ok(api_error!("vector_db.search_failed"));
        }
        Err(e) => {
            warn!(error = %e, query = %query, "生成查询向量失败");
            return Ok(api_error!("vector_db.search_failed"));
        }
    };

    // 执行向量搜索
    match vector_index.search(
        &query_embedding,
        search_options.top_k,
        config.similarity_threshold.max(search_options.threshold),
    ) {
        Ok(results) => {
            let search_results: Vec<SearchResult> = results
                .into_iter()
                .filter_map(|(chunk_id, score)| {
                    vector_index.get_chunk_metadata(&chunk_id).map(|metadata| {
                        SearchResult::new(
                            metadata.file_path.clone(),
                            metadata.span.clone(),
                            score,
                            format!("Chunk {:?}", metadata.chunk_type),
                            None,
                            Some(metadata.chunk_type),
                        )
                    })
                })
                .collect();
            Ok(api_success!(search_results))
        }
        Err(e) => {
            warn!(error = %e, query = %query, "向量搜索失败");
            Ok(api_error!("vector_db.search_failed"))
        }
    }
}

/// 获取索引状态命令
#[tauri::command]
pub async fn get_index_status(
    path: String,
    state: State<'_, VectorDbState>,
) -> TauriApiResult<IndexStatus> {
    let workspace_path = PathBuf::from(&path);

    // 索引目录不存在就返回空状态
    if !workspace_path.join(".oxi").exists() {
        return Ok(api_success!(IndexStatus {
            total_files: 0,
            total_chunks: 0,
            embedding_model: String::new(),
            vector_dimension: 0,
        }));
    }

    match crate::vector_db::storage::IndexManager::new(
        &workspace_path,
        state.search_engine.config().clone(),
    ) {
        Ok(manager) => Ok(api_success!(manager.get_status())),
        Err(_) => Ok(api_success!(IndexStatus {
            total_files: 0,
            total_chunks: 0,
            embedding_model: String::new(),
            vector_dimension: 0,
        })),
    }
}
