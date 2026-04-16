use crate::utils::TauriApiResult;
use crate::{api_error, api_success};
use std::path::PathBuf;
use tracing::warn;

#[tauri::command]
pub async fn file_handle_open(path: String) -> TauriApiResult<String> {
    if path.trim().is_empty() {
        return Ok(api_error!("common.path_empty"));
    }

    let path_buf = PathBuf::from(&path);

    if path_buf.exists() {
        let dir = if path_buf.is_file() {
            match path_buf.parent() {
                Some(parent) => parent,
                None => {
                    warn!("File has no parent directory: {}", path);
                    &path_buf
                }
            }
        } else {
            &path_buf
        };

        let dir_str = dir.to_string_lossy().to_string();
        Ok(api_success!(dir_str))
    } else {
        warn!("Path does not exist: {}", path);
        Ok(api_error!("common.not_found"))
    }
}

#[tauri::command]
pub async fn open_in_editor(path: String) -> TauriApiResult<()> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        warn!("Path does not exist: {}", path);
        return Ok(api_error!("common.not_found"));
    }

    // Detect system default code editor via file extension association
    let extensions = ["rs", "ts", "js", "py", "md", "txt"];
    for ext in &extensions {
        if let Some(editor) = open_in_editor::Editor::new_for_file_extension(ext) {
            if editor.open_paths([&path_buf]).is_ok() {
                return Ok(api_success!(()));
            }
        }
    }

    // Fallback: open with system default handler
    #[cfg(target_os = "macos")]
    {
        if let Err(err) = std::process::Command::new("open").arg(&path).spawn() {
            warn!(
                "Failed to open path with macOS default handler '{}': {}",
                path, err
            );
            return Ok(api_error!("common.system_error"));
        }
    }

    Ok(api_success!(()))
}

#[tauri::command]
pub async fn fetch_registry_skills() -> TauriApiResult<String> {
    match reqwest::get("https://skillregistry.io/api/skills").await {
        Ok(resp) => match resp.text().await {
            Ok(text) => Ok(api_success!(text)),
            Err(e) => {
                warn!("Failed to read registry skills response: {}", e);
                Ok(api_error!("Failed to read registry skills response"))
            }
        },
        Err(e) => {
            warn!("Failed to fetch registry skills: {}", e);
            Ok(api_error!("Failed to fetch registry skills"))
        }
    }
}

pub fn register_all_commands<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        // File drag and drop command
        file_handle_open,
        // Open directory in editor
        open_in_editor,
        // Fetch skills from registry
        fetch_registry_skills,
        // Workspace management commands (from workspace module)
        crate::workspace::commands::workspace_get_recent,
        crate::workspace::commands::workspace_add_recent,
        crate::workspace::commands::workspace_remove_recent,
        crate::workspace::commands::workspace_maintain,
        crate::workspace::commands::workspace_get_or_create,
        crate::workspace::commands::workspace_list_thread_views,
        crate::workspace::commands::workspace_get_thread_messages,
        crate::workspace::commands::workspace_get_active_thread,
        crate::workspace::commands::workspace_get_thread,
        crate::workspace::commands::workspace_create_thread,
        crate::workspace::commands::workspace_set_active_thread,
        crate::workspace::commands::workspace_clear_active_thread,
        crate::workspace::commands::workspace_delete_thread,
        crate::workspace::commands::workspace_get_project_rules,
        crate::workspace::commands::workspace_set_project_rules,
        crate::workspace::commands::workspace_list_rules_files,
        // Run Actions commands
        crate::workspace::commands::workspace_list_run_actions,
        crate::workspace::commands::workspace_create_run_action,
        crate::workspace::commands::workspace_update_run_action,
        crate::workspace::commands::workspace_delete_run_action,
        crate::workspace::commands::workspace_set_selected_run_action,
        crate::workspace::commands::workspace_get_selected_run_action,
        crate::workspace::commands::workspace_get_run_actions_by_paths,
        // Preferences commands
        crate::workspace::commands::preferences_get_batch,
        crate::workspace::commands::preferences_set,
        // Window management commands
        crate::window::commands::window_state_get,
        crate::window::commands::window_state_update,
        // Terminal management commands
        crate::ai::tool::shell::terminal_create,
        crate::ai::tool::shell::terminal_write,
        crate::ai::tool::shell::terminal_resize,
        crate::ai::tool::shell::terminal_close,
        crate::ai::tool::shell::terminal_list,
        crate::ai::tool::shell::terminal_get_available_shells,
        crate::ai::tool::shell::terminal_get_default_shell,
        crate::ai::tool::shell::terminal_validate_shell_path,
        crate::ai::tool::shell::terminal_create_with_shell,
        // Terminal context management commands
        crate::terminal::commands::pane::terminal_context_set_active_pane,
        crate::terminal::commands::pane::terminal_context_get_active_pane,
        crate::terminal::commands::context::terminal_context_get,
        crate::terminal::commands::context::terminal_context_get_active,
        // Terminal Channel stream commands
        crate::terminal::commands::stream::terminal_subscribe_output,
        crate::terminal::commands::stream::terminal_subscribe_output_cancel,
        // Shell integration commands
        crate::shell::commands::shell_pane_setup_integration,
        crate::shell::commands::shell_pane_get_state,
        crate::shell::commands::shell_execute_background_program,
        // Git integration commands
        crate::git::commands::git_check_repository,
        crate::git::commands::git_get_status,
        crate::git::commands::git_get_branches,
        crate::git::commands::git_get_commits,
        crate::git::commands::git_get_commit_files,
        crate::git::commands::git_get_diff,
        crate::git::commands::git_stage_paths,
        crate::git::commands::git_stage_all,
        crate::git::commands::git_unstage_paths,
        crate::git::commands::git_unstage_all,
        crate::git::commands::git_discard_worktree_paths,
        crate::git::commands::git_discard_worktree_all,
        crate::git::commands::git_clean_paths,
        crate::git::commands::git_clean_all,
        crate::git::commands::git_commit,
        crate::git::commands::git_push,
        crate::git::commands::git_pull,
        crate::git::commands::git_fetch,
        crate::git::commands::git_checkout_branch,
        crate::git::commands::git_init_repo,
        crate::git::commands::git_get_diff_stat,
        // Unified file watcher commands
        crate::file_watcher::commands::file_watcher_start,
        crate::file_watcher::commands::file_watcher_stop,
        crate::file_watcher::commands::file_watcher_status,
        // Configuration management commands
        crate::config::commands::config_get,
        crate::config::commands::config_set,
        crate::config::commands::config_reset_to_defaults,
        crate::config::commands::config_open_folder,
        // AI settings (settings.json / workspace .orbitx/settings.json)
        crate::settings::commands::get_global_settings,
        crate::settings::commands::update_global_settings,
        crate::settings::commands::get_workspace_settings,
        crate::settings::commands::update_workspace_settings,
        crate::settings::commands::get_effective_settings,
        // MCP management commands
        crate::agent::mcp::commands::list_mcp_servers,
        crate::agent::mcp::commands::test_mcp_server,
        crate::lsp::commands::lsp_status,
        crate::lsp::commands::lsp_document_symbols,
        crate::lsp::commands::lsp_workspace_symbols,
        crate::lsp::commands::lsp_hover,
        crate::lsp::commands::lsp_definition,
        crate::lsp::commands::lsp_references,
        crate::lsp::commands::lsp_diagnostics,
        crate::agent::mcp::commands::reload_mcp_servers,
        // Terminal configuration commands
        crate::config::terminal_commands::terminal_config_get,
        crate::config::terminal_commands::terminal_config_set,
        crate::config::terminal_commands::terminal_config_validate,
        crate::config::terminal_commands::terminal_config_reset_to_defaults,
        // Theme system commands
        crate::config::theme::commands::theme_get_config_status,
        crate::config::theme::commands::theme_get_current,
        crate::config::theme::commands::theme_get_available,
        crate::config::theme::commands::theme_set_terminal,
        crate::config::theme::commands::theme_set_follow_system,
        // Shortcut system commands
        crate::config::shortcuts::shortcuts_get_config,
        crate::config::shortcuts::shortcuts_update_config,
        crate::config::shortcuts::shortcuts_validate_config,
        crate::config::shortcuts::shortcuts_detect_conflicts,
        crate::config::shortcuts::shortcuts_add,
        crate::config::shortcuts::shortcuts_remove,
        crate::config::shortcuts::shortcuts_update,
        crate::config::shortcuts::shortcuts_reset_to_defaults,
        crate::config::shortcuts::shortcuts_get_statistics,
        crate::config::shortcuts::shortcuts_execute_action,
        crate::config::shortcuts::shortcuts_get_current_platform,
        // Language setting commands
        crate::utils::i18n::commands::language_set_app_language,
        crate::utils::i18n::commands::language_get_app_language,
        // AI model management commands
        crate::ai::commands::ai_models_get,
        crate::ai::commands::ai_models_get_config,
        crate::ai::commands::ai_models_get_config_path,
        crate::ai::commands::ai_models_add,
        crate::ai::commands::ai_models_update,
        crate::ai::commands::ai_models_remove,
        crate::ai::commands::ai_models_test_connection,
        // New Agent dual-track context commands provided by agent::core::commands
        // LLM call commands
        crate::llm::commands::llm_call,
        crate::llm::commands::llm_call_stream,
        crate::llm::commands::llm_get_available_models,
        crate::llm::commands::llm_test_model_connection,
        crate::llm::commands::llm_get_providers,
        crate::llm::commands::llm_get_models_dev_providers,
        crate::llm::commands::llm_refresh_models_dev,
        crate::llm::commands::llm_get_model_info,
        // OAuth authentication commands
        crate::llm::oauth::commands::start_oauth_flow,
        crate::llm::oauth::commands::wait_oauth_callback,
        crate::llm::oauth::commands::cancel_oauth_flow,
        crate::llm::oauth::commands::refresh_oauth_token,
        crate::llm::oauth::commands::check_oauth_status,
        // Agent executor commands (registered for frontend calls)
        crate::agent::core::commands::agent_execute_run,
        crate::agent::core::commands::agent_cancel_run,
        crate::agent::core::commands::agent_tool_confirm,
        crate::agent::core::commands::agent_list_runs,
        crate::agent::core::commands::agent_list_commands,
        crate::agent::core::commands::agent_render_command,
        crate::agent::core::commands::agent_list_skills,
        crate::agent::core::commands::agent_validate_skill,
        crate::agent::core::commands::agent_switch_session_agent,
        // Storage system commands (Runtime)
        crate::ai::tool::storage::commands::storage_get_terminals_state,
        crate::ai::tool::storage::commands::storage_get_terminal_state,
        crate::ai::tool::storage::commands::storage_get_terminal_cwd,
        // Node.js version management commands
        crate::node::commands::node_check_project,
        crate::node::commands::node_get_version_manager,
        crate::node::commands::node_get_current_version,
        crate::node::commands::node_list_versions,
        crate::node::commands::node_get_switch_command,
        // Vector database commands
        crate::vector_db::commands::get_index_status,
        crate::vector_db::commands::delete_workspace_index,
        crate::vector_db::commands::vector_build_index_start,
        crate::vector_db::commands::vector_build_index_status,
        crate::vector_db::commands::vector_build_index_subscribe,
        crate::vector_db::commands::vector_build_index_cancel,
        crate::vector_db::commands::vector_reload_embedding_config,
        // Checkpoint system commands
        crate::checkpoint::commands::checkpoint_list,
        crate::checkpoint::commands::checkpoint_rollback,
        crate::checkpoint::commands::checkpoint_diff,
        crate::checkpoint::commands::checkpoint_diff_with_workspace,
        crate::checkpoint::commands::checkpoint_get_file_content,
        crate::workspace::commands::workspace_list_subagents,
        // File system commands
        crate::filesystem::commands::fs_read_dir,
    ])
}
