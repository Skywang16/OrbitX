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
        crate::workspace::commands::workspace_get_project_rules,
        crate::workspace::commands::workspace_set_project_rules,
        crate::workspace::commands::workspace_list_rules_files,
        // 窗口管理命令
        crate::window::commands::window_manage_state,
        crate::window::commands::window_get_current_directory,
        crate::window::commands::window_get_home_directory,
        crate::window::commands::window_clear_directory_cache,
        crate::window::commands::window_normalize_path,
        crate::window::commands::window_join_paths,
        crate::window::commands::window_path_exists,
        crate::window::commands::window_get_platform_info,
        crate::window::commands::window_set_opacity,
        crate::window::commands::window_get_opacity,
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
        crate::ai::tool::shell::terminal_find_shell_by_name,
        crate::ai::tool::shell::terminal_find_shell_by_path,
        crate::ai::tool::shell::terminal_get_shell_stats,
        crate::ai::tool::shell::terminal_initialize_shell_manager,
        crate::ai::tool::shell::terminal_validate_shell_manager,
        // 终端上下文管理命令
        crate::terminal::commands::pane::terminal_context_set_active_pane,
        crate::terminal::commands::pane::terminal_context_get_active_pane,
        crate::terminal::commands::pane::terminal_context_clear_active_pane,
        crate::terminal::commands::pane::terminal_context_is_pane_active,
        crate::terminal::commands::context::terminal_context_get,
        crate::terminal::commands::context::terminal_context_get_active,
        crate::terminal::commands::cache::terminal_context_invalidate_cache,
        crate::terminal::commands::cache::terminal_context_clear_all_cache,
        crate::terminal::commands::stats::terminal_context_get_cache_stats,
        crate::terminal::commands::stats::terminal_context_get_registry_stats,
        // 终端 Channel 流命令
        crate::terminal::commands::stream::terminal_subscribe_output,
        crate::terminal::commands::stream::terminal_subscribe_output_cancel,
        // Shell 集成命令
        crate::shell::commands::shell_execute_background_command,
        crate::shell::commands::shell_setup_integration,
        crate::shell::commands::shell_check_integration_status,
        crate::shell::commands::shell_update_pane_cwd,
        crate::shell::commands::get_pane_shell_state,
        crate::shell::commands::set_pane_shell_type,
        crate::shell::commands::generate_shell_integration_script,
        crate::shell::commands::generate_shell_env_vars,
        crate::shell::commands::enable_pane_integration,
        crate::shell::commands::disable_pane_integration,
        crate::shell::commands::get_pane_current_command,
        crate::shell::commands::get_pane_command_history,
        crate::shell::commands::detect_shell_type,
        crate::shell::commands::check_shell_integration_support,
        // 补全功能命令
        crate::completion::commands::completion_init_engine,
        crate::completion::commands::completion_get,
        crate::completion::commands::completion_clear_cache,
        crate::completion::commands::completion_get_stats,
        // 配置管理命令
        crate::config::commands::config_get,
        crate::config::commands::config_update,
        crate::config::commands::config_save,
        crate::config::commands::config_validate,
        crate::config::commands::config_reset_to_defaults,
        crate::config::commands::config_get_file_path,
        crate::config::commands::config_get_file_info,
        crate::config::commands::config_open_file,
        crate::config::commands::config_subscribe_events,
        crate::config::commands::config_get_folder_path,
        crate::config::commands::config_open_folder,
        // 终端配置命令
        crate::config::terminal_commands::config_terminal_get,
        crate::config::terminal_commands::config_terminal_update,
        crate::config::terminal_commands::config_terminal_validate,
        crate::config::terminal_commands::config_terminal_reset_to_defaults,
        crate::config::terminal_commands::config_terminal_detect_system_shells,
        crate::config::terminal_commands::config_terminal_validate_shell_path,
        crate::config::terminal_commands::config_terminal_get_shell_info,
        crate::config::terminal_commands::config_terminal_update_cursor,
        crate::config::terminal_commands::config_terminal_update_behavior,
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
        crate::config::shortcuts::shortcuts_search,
        crate::config::shortcuts::shortcuts_execute_action,
        crate::config::shortcuts::shortcuts_get_current_platform,
        crate::config::shortcuts::shortcuts_export_config,
        crate::config::shortcuts::shortcuts_import_config,
        crate::config::shortcuts::shortcuts_get_registered_actions,
        crate::config::shortcuts::shortcuts_get_action_metadata,
        crate::config::shortcuts::shortcuts_validate_key_combination,
        // 语言设置命令
        crate::utils::i18n::commands::language_set_app_language,
        crate::utils::i18n::commands::language_get_app_language,
        crate::utils::i18n::commands::language_get_supported_languages,
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
        crate::agent::core::commands::agent_pause_task,
        crate::agent::core::commands::agent_cancel_task,
        crate::agent::core::commands::agent_list_tasks,
        crate::agent::core::commands::agent_get_file_context_status,
        crate::agent::core::commands::agent_get_user_rules,
        crate::agent::core::commands::agent_set_user_rules,
        // 项目规则命令已迁移到 workspace 模块
        // 双轨架构命令
        crate::agent::core::commands::agent_create_conversation,
        crate::agent::core::commands::agent_delete_conversation,
        crate::agent::core::commands::agent_update_conversation_title,
        crate::agent::core::commands::agent_ui_get_conversations,
        crate::agent::core::commands::agent_ui_get_messages,
        crate::agent::core::commands::agent_ui_delete_messages_from,
        // crate::agent::core::commands::agent_trigger_context_summary, // 暂时注释：类型问题待修复
        // 存储系统命令
        crate::ai::tool::storage::storage_get_config,
        crate::ai::tool::storage::storage_update_config,
        crate::ai::tool::storage::storage_save_session_state,
        crate::ai::tool::storage::storage_load_session_state,
        crate::ai::tool::storage::storage_get_terminals_state,
        crate::ai::tool::storage::storage_get_terminal_cwd,
        // 双轨制任务老命令已废弃，由新的Agent UI持久化替代
        // 网络请求命令
        crate::ai::tool::network::network_web_fetch_headless,
        crate::ai::tool::network::network_simple_web_fetch,
        // Node.js 版本管理命令
        crate::node::commands::node_check_project,
        crate::node::commands::node_get_version_manager,
        crate::node::commands::node_list_versions,
        crate::node::commands::node_get_switch_command,
        // 向量数据库命令
        crate::vector_db::commands::semantic_search,
        crate::vector_db::commands::get_index_status,
        crate::vector_db::commands::index_files,
        crate::vector_db::commands::update_file_index,
        crate::vector_db::commands::remove_file_index,
        crate::vector_db::commands::delete_workspace_index,
        crate::vector_db::commands::vector_build_index,
        crate::vector_db::commands::vector_get_build_progress,
        crate::vector_db::commands::vector_cancel_build,
        // Checkpoint 系统命令
        crate::checkpoint::commands::checkpoint_create,
        crate::checkpoint::commands::checkpoint_list,
        crate::checkpoint::commands::checkpoint_rollback,
        crate::checkpoint::commands::checkpoint_diff,
        crate::checkpoint::commands::checkpoint_diff_with_current,
        crate::checkpoint::commands::checkpoint_get_file_content,
        crate::checkpoint::commands::checkpoint_delete,
    ])
}
