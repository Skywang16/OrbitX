use crate::utils::{EmptyData, TauriApiResult};
use crate::vector_db::commands::VectorDbState;
use crate::{api_error, api_success};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tracing::{error, warn};

#[tauri::command]
pub async fn get_index_status(
    path: String,
    state: State<'_, VectorDbState>,
) -> TauriApiResult<crate::vector_db::storage::IndexStatus> {
    let workspace_path = PathBuf::from(&path);

    if !workspace_path.join(".orbitx").join("index").exists() {
        return Ok(api_success!(crate::vector_db::storage::IndexStatus {
            total_files: 0,
            total_chunks: 0,
            embedding_model: String::new(),
            vector_dimension: 0,
            size_bytes: 0,
        }));
    }

    let engine = match state.current_search_engine() {
        Some(e) => e,
        None => return Ok(api_error!("vector_db.engine_not_configured")),
    };
    let config = engine.config().clone();
    match crate::vector_db::storage::IndexManager::new(&workspace_path, config) {
        Ok(manager) => Ok(api_success!(manager.get_status_with_size_bytes())),
        Err(e) => {
            warn!(error = %e, path = %path, "Failed to get index status");
            Ok(api_error!("vector_db.status_failed"))
        }
    }
}

#[tauri::command]
pub async fn delete_workspace_index(
    path: String,
    state: State<'_, VectorDbState>,
) -> TauriApiResult<EmptyData> {
    let root = PathBuf::from(&path);
    let index_dir = root.join(".orbitx").join("index");

    if let Some(engine) = state.current_search_engine() {
        engine.invalidate_workspace_index(&root);
    }

    if index_dir.exists() {
        let dir = index_dir.clone();
        match tokio::task::spawn_blocking(move || std::fs::remove_dir_all(dir)).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                warn!(error = %e, path = %path, "Failed to delete index");
                return Ok(api_error!("vector_db.delete_failed"));
            }
            Err(e) => {
                error!("Failed to join delete index task: {}", e);
                return Ok(api_error!("vector_db.delete_failed"));
            }
        }
    }
    Ok(api_success!())
}

#[tauri::command]
pub async fn vector_reload_embedding_config(
    state: State<'_, VectorDbState>,
    database: State<'_, Arc<crate::storage::DatabaseManager>>,
) -> TauriApiResult<EmptyData> {
    match crate::vector_db::build_search_engine_from_database(database.inner().clone()).await {
        Ok(search_engine) => {
            state.replace_search_engine(search_engine);
            Ok(api_success!())
        }
        Err(e) => {
            warn!(error = %e, "Failed to reload vector embedding config");
            Ok(api_error!("vector_db.status_failed"))
        }
    }
}
