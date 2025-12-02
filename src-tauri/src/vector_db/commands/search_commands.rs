use tauri::State;
use crate::vector_db::commands::VectorDbState;
use crate::vector_db::search::SearchOptions;
use crate::vector_db::core::SearchResult;
use crate::vector_db::storage::IndexStatus;

/// 语义搜索命令
#[tauri::command]
pub async fn semantic_search(
    query: String,
    options: Option<SearchOptions>,
    state: State<'_, VectorDbState>,
) -> Result<Vec<SearchResult>, String> {
    let default_options = SearchOptions::default();
    let search_options = options.unwrap_or(default_options);
    
    state
        .search_engine
        .search(&query, search_options)
        .await
        .map_err(|e| e.to_string())
}

/// 获取索引状态命令
#[tauri::command]
pub async fn get_index_status(
    state: State<'_, VectorDbState>,
) -> Result<IndexStatus, String> {
    Ok(state.index_manager.get_status())
}
