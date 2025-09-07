/*
 * Copyright (C) 2025 OrbitX Contributors
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

//! 命令注册模块
//!
//! 负责统一管理和注册所有 Tauri 命令接口

// 文件拖拽处理命令
use crate::utils::error::TauriResult;
use std::path::PathBuf;
use tracing::warn;

/// 处理文件打开事件，返回文件所在的目录路径
#[tauri::command]
pub async fn handle_file_open(path: String) -> TauriResult<String> {
    // 确保路径字符串正确处理中文字符
    let path_buf = PathBuf::from(&path);

    if path_buf.exists() {
        let dir = if path_buf.is_file() {
            // 如果是文件，返回其父目录
            match path_buf.parent() {
                Some(parent) => parent,
                None => {
                    warn!("文件没有父目录: {}", path);
                    &path_buf
                }
            }
        } else {
            // 如果是目录，直接返回
            &path_buf
        };

        // 使用 to_string_lossy() 确保中文字符正确转换
        let dir_str = dir.to_string_lossy().to_string();
        Ok(dir_str)
    } else {
        let error_msg = format!("路径不存在: {}", path);
        warn!("{}", error_msg);
        Err(error_msg)
    }
}

/// 注册所有 Tauri 命令
pub fn register_all_commands<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        // 窗口管理命令
        crate::window::commands::manage_window_state,
        crate::window::commands::get_current_directory,
        crate::window::commands::get_home_directory,
        crate::window::commands::clear_directory_cache,
        crate::window::commands::normalize_path,
        crate::window::commands::join_paths,
        crate::window::commands::path_exists,
        crate::window::commands::get_platform_info,
        // 窗口透明度命令
        crate::window::commands::set_window_opacity,
        crate::window::commands::get_window_opacity,
        // 文件拖拽命令
        handle_file_open,
        // 终端管理命令
        crate::ai::tool::shell::create_terminal,
        crate::ai::tool::shell::write_to_terminal,
        crate::ai::tool::shell::resize_terminal,
        crate::ai::tool::shell::close_terminal,
        crate::ai::tool::shell::list_terminals,
        crate::ai::tool::shell::get_available_shells,
        crate::ai::tool::shell::get_default_shell,
        crate::ai::tool::shell::validate_shell_path,
        crate::ai::tool::shell::create_terminal_with_shell,
        crate::ai::tool::shell::find_shell_by_name,
        crate::ai::tool::shell::find_shell_by_path,
        crate::ai::tool::shell::get_shell_stats,
        crate::ai::tool::shell::initialize_shell_manager,
        crate::ai::tool::shell::validate_shell_manager,
        // 终端缓冲区命令
        crate::ai::tool::shell::get_terminal_buffer,
        crate::ai::tool::shell::set_terminal_buffer,
        // 补全功能命令
        crate::completion::commands::init_completion_engine,
        crate::completion::commands::get_completions,
        crate::completion::commands::get_enhanced_completions,
        crate::completion::commands::clear_completion_cache,
        crate::completion::commands::get_completion_stats,
        // 配置管理命令
        crate::config::commands::get_config,
        // 后台命令执行
        crate::shell::commands::execute_background_command,
        crate::config::commands::update_config,
        crate::config::commands::save_config,
        crate::config::commands::validate_config,
        crate::config::commands::reset_config_to_defaults,
        crate::config::commands::get_config_file_path,
        crate::config::commands::get_config_file_info,
        crate::config::commands::open_config_file,
        crate::config::commands::subscribe_config_events,
        crate::config::commands::get_config_folder_path,
        crate::config::commands::open_config_folder,
        // 主题系统命令
        crate::config::theme::commands::get_theme_config_status,
        crate::config::theme::commands::get_current_theme,
        crate::config::theme::commands::get_available_themes,
        // 注意：set_terminal_theme 和 set_follow_system_theme 需要特殊处理 AppHandle
        // 终端配置命令
        crate::config::terminal_commands::get_terminal_config,
        crate::config::terminal_commands::update_terminal_config,
        crate::config::terminal_commands::validate_terminal_config,
        crate::config::terminal_commands::reset_terminal_config_to_defaults,
        crate::config::terminal_commands::detect_system_shells,
        crate::config::terminal_commands::validate_terminal_shell_path,
        crate::config::terminal_commands::get_shell_info,
        crate::config::terminal_commands::update_cursor_config,
        crate::config::terminal_commands::update_terminal_behavior_config,
        // Shell integration命令
        crate::shell::commands::setup_shell_integration,
        crate::shell::commands::check_shell_integration_status,
        crate::shell::commands::get_pane_cwd,
        crate::shell::commands::update_pane_cwd,
        // AI模型管理命令
        crate::ai::commands::get_ai_models,
        crate::ai::commands::add_ai_model,
        crate::ai::commands::update_ai_model,
        crate::ai::commands::remove_ai_model,
        crate::ai::commands::test_ai_connection_with_config,
        // LLM调用命令
        crate::llm::commands::llm_call,
        crate::llm::commands::llm_call_stream,
        crate::llm::commands::llm_get_available_models,
        crate::llm::commands::llm_test_model_connection,
        // LLM供应商和模型信息命令
        crate::llm::commands::llm_get_providers,
        crate::llm::commands::llm_get_provider_models,
        crate::llm::commands::llm_get_model_info,
        crate::llm::commands::llm_check_model_feature,
        // AI会话上下文管理命令
        crate::ai::commands::create_conversation,
        crate::ai::commands::get_conversations,
        crate::ai::commands::get_conversation,
        crate::ai::commands::update_conversation_title,
        crate::ai::commands::delete_conversation,
        crate::ai::commands::get_compressed_context,
        crate::ai::commands::build_prompt_with_context,
        crate::ai::commands::get_user_prefix_prompt,
        crate::ai::commands::set_user_prefix_prompt,
        crate::ai::commands::save_message,
        crate::ai::commands::update_message_content,
        crate::ai::commands::update_message_steps,
        crate::ai::commands::update_message_status,
        crate::ai::commands::truncate_conversation,
        // 终端上下文管理命令 - 面板管理
        crate::terminal::commands::pane::set_active_pane,
        crate::terminal::commands::pane::get_active_pane,
        crate::terminal::commands::pane::clear_active_pane,
        crate::terminal::commands::pane::is_pane_active,
        // 终端上下文管理命令 - 上下文获取
        crate::terminal::commands::context::get_terminal_context,
        crate::terminal::commands::context::get_active_terminal_context,
        // 终端上下文管理命令 - 缓存管理
        crate::terminal::commands::cache::invalidate_context_cache,
        crate::terminal::commands::cache::clear_all_context_cache,
        // 终端上下文管理命令 - 统计信息
        crate::terminal::commands::stats::get_context_cache_stats,
        crate::terminal::commands::stats::get_registry_stats,
        // 全新快捷键系统命令
        crate::config::shortcuts::get_shortcuts_config,
        crate::config::shortcuts::update_shortcuts_config,
        crate::config::shortcuts::validate_shortcuts_config,
        crate::config::shortcuts::detect_shortcuts_conflicts,
        crate::config::shortcuts::add_shortcut,
        crate::config::shortcuts::remove_shortcut,
        crate::config::shortcuts::update_shortcut,
        crate::config::shortcuts::reset_shortcuts_to_defaults,
        crate::config::shortcuts::get_shortcuts_statistics,
        crate::config::shortcuts::search_shortcuts,
        crate::config::shortcuts::execute_shortcut_action,
        crate::config::shortcuts::get_current_platform,
        crate::config::shortcuts::export_shortcuts_config,
        crate::config::shortcuts::import_shortcuts_config,
        crate::config::shortcuts::get_registered_actions,
        crate::config::shortcuts::get_action_metadata,
        crate::config::shortcuts::validate_key_combination,
        // 存储系统命令
        crate::ai::tool::storage::storage_get_config,
        crate::ai::tool::storage::storage_update_config,
        crate::ai::tool::storage::storage_save_session_state,
        crate::ai::tool::storage::storage_load_session_state,
        // 网络请求命令
        crate::ai::tool::network::web_fetch_headless,
        crate::ai::tool::network::simple_web_fetch,
        // AST代码分析命令
        crate::ai::tool::ast::commands::analyze_code,
        // 向量索引系统命令
        crate::vector_index::commands::init_vector_index,
        crate::vector_index::commands::build_code_index,
        crate::vector_index::commands::search_code_vectors,
        crate::vector_index::commands::test_qdrant_connection,
        crate::vector_index::commands::get_vector_index_status,
        // 向量索引配置命令
        crate::vector_index::commands::get_vector_index_config,
        crate::vector_index::commands::save_vector_index_config,
        crate::vector_index::commands::get_current_workspace_path,
        crate::vector_index::commands::cancel_build_index,
        crate::vector_index::commands::clear_vector_index,
        // 文件监控相关命令
        crate::vector_index::commands::start_file_monitoring,
        crate::vector_index::commands::stop_file_monitoring,
        crate::vector_index::commands::get_file_monitoring_status
    ])
}
