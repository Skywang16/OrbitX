use crate::utils::TauriApiResult;
use crate::vector_db::commands::VectorDbState;
use crate::vector_db::core::SearchResult;
use crate::vector_db::search::SearchOptions;
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
    let default_options = SearchOptions::default();
    let search_options = options.unwrap_or(default_options);

    match state
        .search_engine
        .search_in_workspace(&workspace_path, &query, search_options)
        .await
    {
        Ok(results) => Ok(api_success!(results)),
        Err(e) => {
            warn!(error = %e, path = %path, "语义搜索失败");
            Ok(api_error!("vector_db.search_failed"))
        }
    }
}
