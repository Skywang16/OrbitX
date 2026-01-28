/*!
 * TaskExecutor Tauri命令接口
 */

use crate::agent::agents::AgentConfigLoader;
use crate::agent::command_system::{CommandConfigLoader, CommandRenderResult, CommandSummary};
use crate::agent::core::executor::{ExecuteTaskParams, TaskExecutor, TaskSummary};
use crate::agent::skill::SkillSummary;
use crate::agent::tools::registry::ToolConfirmationDecision;
use crate::agent::types::{AgentSwitchBlock, Block, MessageRole, MessageStatus, TaskEvent};
use crate::agent::workspace_changes::{ChangeKind, PendingChange, WorkspaceChangeJournal};
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use serde::Deserialize;
use std::sync::Arc;
use tauri::{ipc::Channel, State};

/// TaskExecutor状态管理
pub struct TaskExecutorState {
    pub executor: Arc<TaskExecutor>,
}

impl TaskExecutorState {
    pub fn new(executor: Arc<TaskExecutor>) -> Self {
        Self { executor }
    }
}

/// 执行Agent任务
#[tauri::command]
pub async fn agent_execute_task(
    state: State<'_, TaskExecutorState>,
    changes: State<'_, Arc<WorkspaceChangeJournal>>,
    params: ExecuteTaskParams,
    channel: Channel<TaskEvent>,
) -> TauriApiResult<EmptyData> {
    let mut params = params;

    // Collect workspace file changes as a system reminder (not persisted to UI messages)
    if let Ok(workspace_root) = std::path::PathBuf::from(&params.workspace_path).canonicalize() {
        let workspace_key: std::sync::Arc<str> =
            std::sync::Arc::from(workspace_root.to_string_lossy().to_string());
        let pending = changes.take_pending_by_key(workspace_key).await;
        let notice = build_file_change_notice(&pending);
        if !notice.is_empty() {
            params.system_reminders.push(notice);
        }
    }

    match state.executor.execute_task(params, channel).await {
        Ok(_context) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("❌ Task execution failed: {}", e);
            Ok(api_error!("agent.execute_failed"))
        }
    }
}

fn build_file_change_notice(changes: &[PendingChange]) -> String {
    if changes.is_empty() {
        return String::new();
    }

    let now = crate::file_watcher::now_timestamp_ms();
    let mut latest: std::collections::BTreeMap<String, &PendingChange> =
        std::collections::BTreeMap::new();
    for change in changes {
        latest.insert(change.relative_path.clone(), change);
    }
    let total_files = latest.len();

    let mut lines = vec![
        "WORKSPACE FILE CHANGES (since your last message)".to_string(),
        "These changes were made by the user or external tools. Treat them as authoritative."
            .to_string(),
        "Do NOT revert them. If you need to edit any of these files, re-read first with read_file to avoid overwriting user changes.".to_string(),
        String::new(),
    ];

    const MAX_FILES: usize = 20;
    let mut shown = 0usize;
    for (path, change) in latest.iter() {
        if shown >= MAX_FILES {
            break;
        }
        shown += 1;

        let age_ms = now.saturating_sub(change.observed_at_ms);
        let age = format_age(age_ms);

        let kind = match change.kind {
            ChangeKind::Created => "created",
            ChangeKind::Modified => "modified",
            ChangeKind::Deleted => "deleted",
            ChangeKind::Renamed => "renamed",
        };

        if change.large_change || change.patch.is_none() {
            let suffix = if change.large_change {
                "Large change detected; re-read before editing."
            } else {
                "Changed; re-read before editing if needed."
            };
            lines.push(format!("- {path} ({kind}, {age} ago): {suffix}"));
            continue;
        }

        lines.push(format!("- {path} ({kind}, {age} ago):"));
        lines.push("```diff".to_string());
        lines.push(change.patch.clone().unwrap_or_default());
        lines.push("```".to_string());
    }

    let omitted = total_files.saturating_sub(shown);
    if omitted > 0 {
        lines.push(String::new());
        lines.push(format!(
            "(Plus {omitted} more changed files omitted to keep context small.)"
        ));
    }

    lines.join("\n").trim().to_string()
}

fn format_age(age_ms: u64) -> String {
    if age_ms < 1_000 {
        return "just now".to_string();
    }
    let secs = age_ms / 1_000;
    if secs < 60 {
        return format!("{secs}s");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m");
    }
    let hours = mins / 60;
    if hours < 48 {
        return format!("{hours}h");
    }
    let days = hours / 24;
    format!("{days}d")
}

/// 取消任务
#[tauri::command]
pub async fn agent_cancel_task(
    state: State<'_, TaskExecutorState>,
    task_id: String,
    reason: Option<String>,
) -> TauriApiResult<EmptyData> {
    match state.executor.cancel_task(&task_id, reason).await {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("❌ Cancel task failed: {}", e);
            Ok(api_error!("agent.cancel_failed"))
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfirmationParams {
    pub request_id: String,
    pub decision: ToolConfirmationDecision,
}

/// 回传工具确认结果
#[tauri::command]
pub async fn agent_tool_confirm(
    state: State<'_, TaskExecutorState>,
    params: ToolConfirmationParams,
) -> TauriApiResult<EmptyData> {
    let task_id = state
        .executor
        .tool_confirmations()
        .lookup_task_id(&params.request_id);
    let ctx = task_id.and_then(|task_id| {
        state
            .executor
            .active_tasks()
            .get(&task_id)
            .map(|entry| Arc::clone(entry.value()))
    });

    let ctx = match ctx {
        Some(ctx) => ctx,
        None => return Ok(api_error!("agent.tool_confirm_not_found")),
    };

    let ok = ctx
        .tool_registry()
        .resolve_confirmation(&ctx, &params.request_id, params.decision)
        .await;

    if ok {
        Ok(api_success!())
    } else {
        Ok(api_error!("agent.tool_confirm_not_found"))
    }
}

/// 列出任务
#[tauri::command]
pub async fn agent_list_tasks(
    state: State<'_, TaskExecutorState>,
    session_id: Option<i64>,
    status_filter: Option<String>,
) -> TauriApiResult<Vec<TaskSummary>> {
    match state.executor.list_tasks(session_id, status_filter).await {
        Ok(tasks) => Ok(api_success!(tasks)),
        Err(e) => {
            tracing::error!("❌ List tasks failed: {}", e);
            Ok(api_error!("agent.list_failed"))
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCommandsParams {
    pub workspace_path: String,
}

/// 列出 `.orbitx/commands/*.md` 中定义的命令（不负责输入框 `/xxx` 触发）
#[tauri::command]
pub async fn agent_list_commands(
    _state: State<'_, TaskExecutorState>,
    params: ListCommandsParams,
) -> TauriApiResult<Vec<CommandSummary>> {
    let workspace_root = std::path::PathBuf::from(params.workspace_path);
    let configs = CommandConfigLoader::load_for_workspace(&workspace_root)
        .await
        .unwrap_or_default();

    let mut out = configs
        .values()
        .map(CommandConfigLoader::summarize)
        .collect::<Vec<_>>();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(api_success!(out))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderCommandParams {
    pub workspace_path: String,
    pub name: String,
    pub input: String,
}

/// 渲染命令模板（只做 `{{input}}` 替换）
#[tauri::command]
pub async fn agent_render_command(
    _state: State<'_, TaskExecutorState>,
    params: RenderCommandParams,
) -> TauriApiResult<CommandRenderResult> {
    let workspace_root = std::path::PathBuf::from(params.workspace_path);
    let configs = CommandConfigLoader::load_for_workspace(&workspace_root)
        .await
        .unwrap_or_default();

    let Some(cfg) = configs.get(params.name.as_str()) else {
        return Ok(api_error!("agent.command_not_found"));
    };

    // 加载 skills 使用新 API (暂时只用工作区, 全局skills在TaskExecutor中会统一加载)
    let skill_manager = crate::agent::skill::SkillManager::new();
    let skills = if let Ok(_) = skill_manager.discover_skills(None, Some(&workspace_root)).await {
        skill_manager
            .activate_skills(
                &params.input,
                crate::agent::skill::SkillMatchingMode::Hybrid,
                None,
            )
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    Ok(api_success!(CommandConfigLoader::render_with_skills(
        cfg,
        &params.input,
        &skills
    )))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSkillsParams {
    pub workspace_path: String,
}

/// 列出全局 + 工作区 skills（工作区优先级更高）
#[tauri::command]
pub async fn agent_list_skills(
    _state: State<'_, TaskExecutorState>,
    config_paths: State<'_, crate::config::paths::ConfigPaths>,
    params: ListSkillsParams,
) -> TauriApiResult<Vec<SkillSummary>> {
    let workspace_root = std::path::PathBuf::from(params.workspace_path);
    let global_skills_dir = config_paths.skills_dir();

    // 使用新的 SkillManager API (全局 + 工作区)
    let skill_manager = crate::agent::skill::SkillManager::new();
    let metadata_list = skill_manager
        .discover_skills(Some(global_skills_dir), Some(&workspace_root))
        .await
        .unwrap_or_default();

    let mut out: Vec<SkillSummary> = metadata_list
        .iter()
        .map(|m| m.into())
        .collect();

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(api_success!(out))
}

/// 验证 Skill 格式
#[tauri::command]
pub async fn agent_validate_skill(
    _state: State<'_, TaskExecutorState>,
    skill_path: String,
) -> TauriApiResult<crate::agent::skill::ValidationResult> {
    let skill_dir = std::path::PathBuf::from(skill_path);

    match crate::agent::skill::SkillValidator::validate(&skill_dir).await {
        Ok(result) => Ok(api_success!(result)),
        Err(e) => {
            tracing::error!("❌ Validate skill failed: {}", e);
            Ok(api_error!("agent.validate_skill_failed"))
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchSessionAgentParams {
    pub session_id: i64,
    pub agent_type: String,
    pub reason: Option<String>,
}

/// 切换会话的当前 Agent（不做任何向后兼容）
#[tauri::command]
pub async fn agent_switch_session_agent(
    state: State<'_, TaskExecutorState>,
    params: SwitchSessionAgentParams,
) -> TauriApiResult<EmptyData> {
    let persistence = state.executor.agent_persistence();
    let Some(session) = persistence
        .sessions()
        .get(params.session_id)
        .await
        .ok()
        .flatten()
    else {
        return Ok(api_error!("workspace.session_not_found"));
    };

    let workspace_root = std::path::PathBuf::from(session.workspace_path.clone());
    let agent_configs = AgentConfigLoader::load_for_workspace(&workspace_root)
        .await
        .unwrap_or_default();
    if !agent_configs.contains_key(params.agent_type.as_str()) {
        return Ok(api_error!("agent.unknown_agent_type"));
    }

    let from_agent = session.agent_type.clone();
    if from_agent == params.agent_type {
        return Ok(api_success!());
    }

    if let Err(err) = persistence
        .sessions()
        .update_agent_type(params.session_id, &params.agent_type)
        .await
    {
        tracing::error!("❌ Switch agent failed: {}", err);
        return Ok(api_error!("agent.switch_failed"));
    }

    let mut message = match persistence
        .messages()
        .create(
            params.session_id,
            MessageRole::Assistant,
            MessageStatus::Completed,
            vec![Block::AgentSwitch(AgentSwitchBlock {
                from_agent,
                to_agent: params.agent_type.clone(),
                reason: params.reason.clone(),
            })],
            false,
            false,
            &params.agent_type,
            None,
            None,
            None,
        )
        .await
    {
        Ok(m) => m,
        Err(err) => {
            tracing::error!("❌ Create switch message failed: {}", err);
            return Ok(api_error!("agent.switch_failed"));
        }
    };

    let now = chrono::Utc::now();
    message.finished_at = Some(now);
    message.duration_ms = Some(0);
    if let Err(err) = persistence.messages().update(&message).await {
        tracing::warn!("⚠️  Update switch message failed: {}", err);
    }

    Ok(api_success!())
}
