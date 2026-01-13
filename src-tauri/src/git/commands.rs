use crate::git::GitService;
use crate::utils::{ApiResponse, EmptyData, TauriApiResult};
use crate::{api_error, api_success};

fn map_git_error<T>(e: crate::git::GitError) -> ApiResponse<T> {
    match e.code {
        crate::git::GitErrorCode::GitNotInstalled
        | crate::git::GitErrorCode::NotARepository
        | crate::git::GitErrorCode::ParseError => api_error!(e.message.as_str()),
        _ => ApiResponse::error(e.message),
    }
}

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
pub async fn git_get_commits(
    path: String,
    limit: Option<u32>,
    skip: Option<u32>,
) -> TauriApiResult<Vec<crate::git::CommitInfo>> {
    let limit = limit.unwrap_or(50);
    let skip = skip.unwrap_or(0);
    match GitService::get_commits(&path, limit, skip).await {
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
pub async fn git_stage_paths(path: String, paths: Vec<String>) -> TauriApiResult<EmptyData> {
    match GitService::stage_paths(&path, &paths).await {
        Ok(()) => Ok(api_success!()),
        Err(e) => Ok(map_git_error(e)),
    }
}

#[tauri::command]
pub async fn git_stage_all(path: String) -> TauriApiResult<EmptyData> {
    match GitService::stage_all(&path).await {
        Ok(()) => Ok(api_success!()),
        Err(e) => Ok(map_git_error(e)),
    }
}

#[tauri::command]
pub async fn git_unstage_paths(path: String, paths: Vec<String>) -> TauriApiResult<EmptyData> {
    match GitService::unstage_paths(&path, &paths).await {
        Ok(()) => Ok(api_success!()),
        Err(e) => Ok(map_git_error(e)),
    }
}

#[tauri::command]
pub async fn git_unstage_all(path: String) -> TauriApiResult<EmptyData> {
    match GitService::unstage_all(&path).await {
        Ok(()) => Ok(api_success!()),
        Err(e) => Ok(map_git_error(e)),
    }
}

#[tauri::command]
pub async fn git_discard_worktree_paths(
    path: String,
    paths: Vec<String>,
) -> TauriApiResult<EmptyData> {
    match GitService::discard_worktree_paths(&path, &paths).await {
        Ok(()) => Ok(api_success!()),
        Err(e) => Ok(map_git_error(e)),
    }
}

#[tauri::command]
pub async fn git_discard_worktree_all(path: String) -> TauriApiResult<EmptyData> {
    match GitService::discard_worktree_all(&path).await {
        Ok(()) => Ok(api_success!()),
        Err(e) => Ok(map_git_error(e)),
    }
}

#[tauri::command]
pub async fn git_clean_paths(path: String, paths: Vec<String>) -> TauriApiResult<EmptyData> {
    match GitService::clean_paths(&path, &paths).await {
        Ok(()) => Ok(api_success!()),
        Err(e) => Ok(map_git_error(e)),
    }
}

#[tauri::command]
pub async fn git_clean_all(path: String) -> TauriApiResult<EmptyData> {
    match GitService::clean_all(&path).await {
        Ok(()) => Ok(api_success!()),
        Err(e) => Ok(map_git_error(e)),
    }
}

#[tauri::command]
pub async fn git_commit(path: String, message: String) -> TauriApiResult<EmptyData> {
    match GitService::commit(&path, &message).await {
        Ok(()) => Ok(api_success!()),
        Err(e) => Ok(map_git_error(e)),
    }
}

#[tauri::command]
pub async fn git_push(path: String) -> TauriApiResult<EmptyData> {
    match GitService::push(&path).await {
        Ok(()) => Ok(api_success!()),
        Err(e) => Ok(map_git_error(e)),
    }
}

#[tauri::command]
pub async fn git_pull(path: String) -> TauriApiResult<EmptyData> {
    match GitService::pull(&path).await {
        Ok(()) => Ok(api_success!()),
        Err(e) => Ok(map_git_error(e)),
    }
}

#[tauri::command]
pub async fn git_fetch(path: String) -> TauriApiResult<EmptyData> {
    match GitService::fetch(&path).await {
        Ok(()) => Ok(api_success!()),
        Err(e) => Ok(map_git_error(e)),
    }
}

#[tauri::command]
pub async fn git_checkout_branch(path: String, branch: String) -> TauriApiResult<EmptyData> {
    match GitService::checkout_branch(&path, &branch).await {
        Ok(()) => Ok(api_success!()),
        Err(e) => Ok(map_git_error(e)),
    }
}

#[tauri::command]
pub async fn git_init_repo(path: String) -> TauriApiResult<EmptyData> {
    match GitService::init_repo(&path).await {
        Ok(()) => Ok(api_success!()),
        Err(e) => Ok(map_git_error(e)),
    }
}

// git watch 已被统一文件监听器取代
