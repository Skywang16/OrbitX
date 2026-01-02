use crate::git::GitService;
use crate::utils::{ApiResponse, TauriApiResult};
use crate::{api_error, api_success};
use tauri::{AppHandle, Runtime, State};

use super::GitWatcher;

#[tauri::command]
pub async fn git_check_repository(path: String) -> TauriApiResult<Option<String>> {
    match GitService::is_repository(&path).await {
        Ok(root) => Ok(api_success!(root)),
        Err(e) => match e.code {
            crate::git::GitErrorCode::GitNotInstalled => Ok(api_error!(e.message.as_str())),
            _ => Ok(ApiResponse::error(e.message)),
        },
    }
}

#[tauri::command]
pub async fn git_get_status(path: String) -> TauriApiResult<crate::git::RepositoryStatus> {
    match GitService::get_status(&path).await {
        Ok(status) => Ok(api_success!(status)),
        Err(e) => match e.code {
            crate::git::GitErrorCode::GitNotInstalled => Ok(api_error!(e.message.as_str())),
            crate::git::GitErrorCode::ParseError => Ok(api_error!(e.message.as_str())),
            _ => Ok(ApiResponse::error(e.message)),
        },
    }
}

#[tauri::command]
pub async fn git_get_branches(path: String) -> TauriApiResult<Vec<crate::git::BranchInfo>> {
    match GitService::get_branches(&path).await {
        Ok(branches) => Ok(api_success!(branches)),
        Err(e) => match e.code {
            crate::git::GitErrorCode::GitNotInstalled => Ok(api_error!(e.message.as_str())),
            crate::git::GitErrorCode::NotARepository => Ok(api_error!(e.message.as_str())),
            _ => Ok(ApiResponse::error(e.message)),
        },
    }
}

#[tauri::command]
pub async fn git_get_commits(path: String, limit: Option<u32>) -> TauriApiResult<Vec<crate::git::CommitInfo>> {
    let limit = limit.unwrap_or(50);
    match GitService::get_commits(&path, limit).await {
        Ok(commits) => Ok(api_success!(commits)),
        Err(e) => match e.code {
            crate::git::GitErrorCode::GitNotInstalled => Ok(api_error!(e.message.as_str())),
            crate::git::GitErrorCode::NotARepository => Ok(api_error!(e.message.as_str())),
            _ => Ok(ApiResponse::error(e.message)),
        },
    }
}

#[tauri::command]
pub async fn git_get_commit_files(
    path: String,
    commit_hash: String,
) -> TauriApiResult<Vec<crate::git::CommitFileChange>> {
    match GitService::get_commit_files(&path, &commit_hash).await {
        Ok(files) => Ok(api_success!(files)),
        Err(e) => match e.code {
            crate::git::GitErrorCode::GitNotInstalled => Ok(api_error!(e.message.as_str())),
            crate::git::GitErrorCode::NotARepository => Ok(api_error!(e.message.as_str())),
            _ => Ok(ApiResponse::error(e.message)),
        },
    }
}

#[tauri::command]
pub async fn git_get_diff(
    path: String,
    file_path: String,
    staged: Option<bool>,
    commit_hash: Option<String>,
) -> TauriApiResult<crate::git::DiffContent> {
    let result = match commit_hash {
        Some(hash) => GitService::get_commit_file_diff(&path, &hash, &file_path).await,
        None => GitService::get_diff(&path, &file_path, staged.unwrap_or(false)).await,
    };

    match result {
        Ok(diff) => Ok(api_success!(diff)),
        Err(e) => match e.code {
            crate::git::GitErrorCode::GitNotInstalled => Ok(api_error!(e.message.as_str())),
            crate::git::GitErrorCode::NotARepository => Ok(api_error!(e.message.as_str())),
            _ => Ok(ApiResponse::error(e.message)),
        },
    }
}

#[tauri::command]
pub async fn git_watch_start<R: Runtime>(
    app_handle: AppHandle<R>,
    watcher: State<'_, GitWatcher>,
    path: String,
) -> TauriApiResult<()> {
    match watcher.start(app_handle, path).await {
        Ok(()) => Ok(api_success!(())),
        Err(e) => Ok(api_error!(e.as_str())),
    }
}

#[tauri::command]
pub async fn git_watch_stop(watcher: State<'_, GitWatcher>) -> TauriApiResult<()> {
    watcher.stop().await;
    Ok(api_success!(()))
}

#[tauri::command]
pub async fn git_watch_status(watcher: State<'_, GitWatcher>) -> TauriApiResult<Option<String>> {
    let path = watcher.watched_path().await;
    Ok(api_success!(path))
}
