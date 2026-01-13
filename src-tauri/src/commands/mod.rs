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

pub fn register_all_commands<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        // 文件拖拽命令
        file_handle_open,
        // Dock 菜单命令
        crate::dock::commands::dock_update_tabs,
        crate::dock::commands::dock_get_tabs,
        crate::dock::commands::dock_clear_tabs,
        // 工作区管理命令（来自 workspace 模块）
        crate::workspace::commands::workspace_get_recent,
        crate::workspace::commands::workspace_add_recent,
        crate::workspace::commands::workspace_remove_recent,
        crate::workspace::commands::workspace_maintain,
        crate::workspace::commands::workspace_get_or_create,
        crate::workspace::commands::workspace_list_sessions,
        crate::workspace::commands::workspace_get_messages,
        crate::workspace::commands::workspace_get_active_session,
        crate::workspace::commands::workspace_create_session,
        crate::workspace::commands::workspace_set_active_session,
        crate::workspace::commands::workspace_get_project_rules,
        crate::workspace::commands::workspace_set_project_rules,
        crate::workspace::commands::workspace_list_rules_files,
        // 窗口管理命令
        crate::window::commands::window_state_get,
        crate::window::commands::window_state_update,
        // 终端管理命令
        crate::ai::tool::shell::terminal_create,
        crate::ai::tool::shell::terminal_write,
        crate::ai::tool::shell::terminal_resize,
        crate::ai::tool::shell::terminal_close,
        crate::ai::tool::shell::terminal_list,
        crate::ai::tool::shell::terminal_get_available_shells,
        crate::ai::tool::shell::terminal_get_default_shell,
        crate::ai::tool::shell::terminal_validate_shell_path,
        crate::ai::tool::shell::terminal_create_with_shell,
        // 终端上下文管理命令
        crate::terminal::commands::pane::terminal_context_set_active_pane,
        crate::terminal::commands::pane::terminal_context_get_active_pane,
        crate::terminal::commands::context::terminal_context_get,
        crate::terminal::commands::context::terminal_context_get_active,
        // 终端 Channel 流命令
        crate::terminal::commands::stream::terminal_subscribe_output,
        crate::terminal::commands::stream::terminal_subscribe_output_cancel,
        // Shell 集成命令
        crate::shell::commands::shell_pane_setup_integration,
        crate::shell::commands::shell_pane_get_state,
        crate::shell::commands::shell_execute_background_program,
        // 补全功能命令
        crate::completion::commands::completion_init_engine,
        crate::completion::commands::completion_get,
        crate::completion::commands::completion_clear_cache,
        crate::completion::commands::completion_get_stats,
        // Git 集成命令
        crate::git::commands::git_check_repository,
        crate::git::commands::git_get_status,
        crate::git::commands::git_get_branches,
        crate::git::commands::git_get_commits,
        crate::git::commands::git_get_commit_files,
        crate::git::commands::git_get_diff,
        crate::git::commands::git_watch_start,
        crate::git::commands::git_watch_stop,
        crate::git::commands::git_watch_status,
        // 配置管理命令
        crate::config::commands::config_get,
        crate::config::commands::config_set,
        crate::config::commands::config_reset_to_defaults,
        crate::config::commands::config_open_folder,
        // AI 设置（settings.json / workspace .orbitx/settings.json）
        crate::settings::commands::get_global_settings,
        crate::settings::commands::update_global_settings,
        crate::settings::commands::get_workspace_settings,
        crate::settings::commands::update_workspace_settings,
        crate::settings::commands::get_effective_settings,
        // MCP 管理命令
        crate::agent::mcp::commands::list_mcp_servers,
        crate::agent::mcp::commands::test_mcp_server,
        crate::agent::mcp::commands::reload_mcp_servers,
        // 终端配置命令
        crate::config::terminal_commands::terminal_config_get,
        crate::config::terminal_commands::terminal_config_set,
        crate::config::terminal_commands::terminal_config_validate,
        crate::config::terminal_commands::terminal_config_reset_to_defaults,
        // 主题系统命令
        crate::config::theme::commands::theme_get_config_status,
        crate::config::theme::commands::theme_get_current,
        crate::config::theme::commands::theme_get_available,
        crate::config::theme::commands::theme_set_terminal,
        crate::config::theme::commands::theme_set_follow_system,
        // 快捷键系统命令
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
        // 语言设置命令
        crate::utils::i18n::commands::language_set_app_language,
        crate::utils::i18n::commands::language_get_app_language,
        // AI 模型管理命令
        crate::ai::commands::ai_models_get,
        crate::ai::commands::ai_models_add,
        crate::ai::commands::ai_models_update,
        crate::ai::commands::ai_models_remove,
        crate::ai::commands::ai_models_test_connection,
        // 新Agent双轨上下文命令由 agent::core::commands 提供
        // LLM 调用命令
        crate::llm::commands::llm_call,
        crate::llm::commands::llm_call_stream,
        crate::llm::commands::llm_get_available_models,
        crate::llm::commands::llm_test_model_connection,
        crate::llm::commands::llm_get_providers,
        // Agent 执行器命令（注册以供前端调用）
        crate::agent::core::commands::agent_execute_task,
        crate::agent::core::commands::agent_cancel_task,
        crate::agent::core::commands::agent_tool_confirm,
        crate::agent::core::commands::agent_list_tasks,
        // 项目规则命令已迁移到 workspace 模块
        // 存储系统命令（State/Runtime）
        crate::ai::tool::storage::storage_save_session_state,
        crate::ai::tool::storage::storage_load_session_state,
        crate::ai::tool::storage::storage_get_terminals_state,
        crate::ai::tool::storage::storage_get_terminal_cwd,
        // 双轨制任务老命令已废弃，由新的Agent UI持久化替代
        // Node.js 版本管理命令
        crate::node::commands::node_check_project,
        crate::node::commands::node_get_version_manager,
        crate::node::commands::node_list_versions,
        crate::node::commands::node_get_switch_command,
        // 向量数据库命令
        crate::vector_db::commands::get_index_status,
        crate::vector_db::commands::delete_workspace_index,
        crate::vector_db::commands::vector_build_index_start,
        crate::vector_db::commands::vector_build_index_status,
        crate::vector_db::commands::vector_build_index_subscribe,
        crate::vector_db::commands::vector_build_index_cancel,
        // Checkpoint 系统命令
        crate::checkpoint::commands::checkpoint_list,
        crate::checkpoint::commands::checkpoint_rollback,
        crate::checkpoint::commands::checkpoint_diff,
        crate::checkpoint::commands::checkpoint_diff_with_workspace,
        crate::checkpoint::commands::checkpoint_get_file_content,
        // 文件系统命令
        crate::filesystem::commands::fs_read_dir,
    ])
}
