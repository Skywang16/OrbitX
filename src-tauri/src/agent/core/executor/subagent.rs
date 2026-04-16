use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use crate::agent::agents::AgentConfigLoader;
use crate::agent::common::truncate_chars;
use crate::agent::core::context::{
    AgentRunContext, AgentRunContextDeps, SubAgentRequest, SubAgentResponse,
};
use crate::agent::error::{AgentRunError, AgentRunResult};
use crate::agent::permissions::PermissionDecision;
use crate::agent::rollout::ThreadMeta;
use crate::agent::subagent::{CreateSubagentParams, SubagentRepository, UpdateSubagentParams};
use crate::agent::tools::RunnableTool;
use crate::agent::types::{AgentRunEvent, Block, MessageRole, SubagentStatus};
use crate::agent::utils::subagent_name_candidates;
use crate::git::service as git_service;
use crate::storage::repositories::AIModels;
use chrono::Utc;

use super::AgentRunExecutor;

const MAX_ACTIVE_SUBAGENTS_GLOBAL: usize = 8;
const MAX_ACTIVE_SUBAGENTS_PER_PARENT: usize = 3;

struct PreparedSubagent {
    child_thread_id: i64,
    child_display_name: String,
}

#[allow(dead_code)]
fn subagent_error_message(status: &SubagentStatus, summary: Option<&str>) -> Option<String> {
    let _ = status;
    summary
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}

pub async fn run_subagent_task(
    executor: &AgentRunExecutor,
    parent: &AgentRunContext,
    request: SubAgentRequest,
) -> AgentRunResult<SubAgentResponse> {
    let prepared = prepare_subagent(executor, parent, &request).await?;
    let result = execute_prepared_subagent(executor, parent, request, &prepared).await;
    executor.decrement_active_child_executions_for_parent(parent.run_id.as_ref());
    result
}

pub async fn spawn_subagent_detached(
    executor: &AgentRunExecutor,
    parent: &AgentRunContext,
    request: SubAgentRequest,
    collab_call_id: String,
    _collab_tool: String,
) -> AgentRunResult<i64> {
    let prepared = prepare_subagent(executor, parent, &request).await?;
    let child_thread_id = prepared.child_thread_id;
    let child_display_name = prepared.child_display_name.clone();
    let _sender_thread_id = parent.thread_id;
    let parent_message_id = parent.current_assistant_message_id().await.ok_or_else(|| {
        AgentRunError::StatePersistenceFailed(
            "assistant message not initialized for subagent creation".to_string(),
        )
    })?;
    let subagent_repo = SubagentRepository::new(parent.repositories());
    let existing_names = subagent_repo
        .list_by_parent_thread(parent.thread_id)
        .await?
        .into_iter()
        .map(|subagent| subagent.name)
        .collect::<HashSet<_>>();
    let candidates =
        subagent_name_candidates(&existing_names).map_err(AgentRunError::ConfigurationError)?;
    let mut created_subagent = None;
    for candidate in candidates {
        match subagent_repo
            .create(CreateSubagentParams {
                id: &collab_call_id,
                parent_thread_id: parent.thread_id,
                child_thread_id,
                parent_message_id,
                name: &candidate,
                profile: &request.profile,
                task_title: &request.description,
                status: SubagentStatus::Running,
            })
            .await
        {
            Ok(record) => {
                created_subagent = Some(record);
                break;
            }
            Err(crate::agent::error::AgentError::Database(err))
                if err
                    .as_database_error()
                    .is_some_and(|db| db.is_unique_violation()) =>
            {
                continue;
            }
            Err(err) => return Err(AgentRunError::StatePersistenceFailed(err.to_string())),
        }
    }
    let created_subagent = created_subagent.ok_or_else(|| {
        AgentRunError::ConfigurationError("failed to allocate a unique subagent name".to_string())
    })?;
    executor
        .agent_persistence()
        .threads()
        .update_display_name(child_thread_id, &created_subagent.name)
        .await
        .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
    let parent_ctx = executor
        .active_runs()
        .get(parent.run_id.as_ref())
        .map(|entry| Arc::clone(entry.value()))
        .ok_or_else(|| {
            AgentRunError::StatePersistenceFailed(format!(
                "parent run {} not found in active runs",
                parent.run_id
            ))
        })?;
    parent_ctx
        .emit_event(AgentRunEvent::SubagentCreated {
            run_id: parent.run_id.to_string(),
            subagent: created_subagent,
        })
        .await?;
    let executor_clone = executor.clone();
    tokio::spawn(async move {
        let result =
            execute_prepared_subagent(&executor_clone, parent_ctx.as_ref(), request, &prepared)
                .await;
        executor_clone.decrement_active_child_executions_for_parent(parent_ctx.run_id.as_ref());

        let (status, summary, receiver_thread_id, error_message) = match result {
            Ok(response) => (
                match response.status {
                    SubagentStatus::Pending => SubagentStatus::Pending,
                    SubagentStatus::Running => SubagentStatus::Running,
                    SubagentStatus::Completed => SubagentStatus::Completed,
                    SubagentStatus::Cancelled => SubagentStatus::Cancelled,
                    SubagentStatus::Error => SubagentStatus::Error,
                },
                response.summary,
                response.thread_id,
                None,
            ),
            Err(err) => (
                SubagentStatus::Error,
                Some(err.to_string()),
                child_thread_id,
                Some(err.to_string()),
            ),
        };
        let subagent_repo = SubagentRepository::new(parent_ctx.repositories());
        match subagent_repo
            .update(
                &collab_call_id,
                UpdateSubagentParams {
                    status: Some(status),
                    final_summary: Some(summary),
                    error_message: Some(error_message),
                    finished_at: Some(Some(Utc::now())),
                    ..UpdateSubagentParams::default()
                },
            )
            .await
        {
            Ok(updated) => {
                if let Err(err) = parent_ctx
                    .emit_event(AgentRunEvent::SubagentUpdated {
                        run_id: parent_ctx.run_id.to_string(),
                        subagent: updated,
                    })
                    .await
                {
                    tracing::warn!(
                        thread_id = receiver_thread_id,
                        "Failed to emit detached subagent update: {}",
                        err
                    );
                }
            }
            Err(err) => {
                tracing::warn!(
                    thread_id = receiver_thread_id,
                    agent = %child_display_name,
                    "Failed to update detached subagent: {}",
                    err
                );
            }
        }
    });

    Ok(child_thread_id)
}

async fn prepare_subagent(
    executor: &AgentRunExecutor,
    parent: &AgentRunContext,
    request: &SubAgentRequest,
) -> AgentRunResult<PreparedSubagent> {
    let workspace_root = PathBuf::from(parent.cwd.as_ref());

    let parent_thread = executor
        .agent_persistence()
        .threads()
        .get(parent.thread_id)
        .await
        .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?
        .ok_or_else(|| {
            AgentRunError::StatePersistenceFailed(format!(
                "parent thread {} not found",
                parent.thread_id
            ))
        })?;

    let agent_configs = AgentConfigLoader::load_for_workspace(&workspace_root)
        .await
        .map_err(|e| {
            AgentRunError::ConfigurationError(format!("Failed to load agent configs: {e}"))
        })?;
    let Some(sub_cfg) = agent_configs.get(request.profile.as_str()) else {
        return Err(AgentRunError::ConfigurationError(format!(
            "Unknown task profile: {}",
            request.profile
        )));
    };

    let parent_cfg = agent_configs
        .get(parent.agent_type.as_ref())
        .ok_or_else(|| {
            AgentRunError::ConfigurationError(format!(
                "Parent agent config not found: {}",
                parent.agent_type
            ))
        })?;

    match parent_cfg.task_permission_for(&request.profile) {
        PermissionDecision::Deny => {
            return Err(AgentRunError::ConfigurationError(format!(
                "Agent '{}' is not allowed to delegate to task profile '{}'",
                parent.agent_type, request.profile
            )))
        }
        PermissionDecision::Ask | PermissionDecision::Allow => {}
    }

    if !matches!(
        sub_cfg.mode,
        crate::agent::agents::config::AgentMode::TaskProfile
    ) {
        return Err(AgentRunError::ConfigurationError(format!(
            "Agent {} is not a child-execution profile",
            sub_cfg.name
        )));
    }

    let active_subagents_global = executor.active_child_executions_global();
    if active_subagents_global >= MAX_ACTIVE_SUBAGENTS_GLOBAL {
        return Err(AgentRunError::TooManyActiveSubagentsGlobal {
            current: active_subagents_global,
            limit: MAX_ACTIVE_SUBAGENTS_GLOBAL,
        });
    }

    let active_subagents_for_parent =
        executor.active_child_executions_for_parent(parent.run_id.as_ref());
    if active_subagents_for_parent >= MAX_ACTIVE_SUBAGENTS_PER_PARENT {
        return Err(AgentRunError::TooManyActiveSubagentsPerParent {
            parent_task_id: parent.run_id.to_string(),
            current: active_subagents_for_parent,
            limit: MAX_ACTIVE_SUBAGENTS_PER_PARENT,
        });
    }

    let database = executor.database();
    let repo = AIModels::new(database.as_ref());

    let configured_model = sub_cfg.model_id.as_ref().and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let config_file_model = repo
        .get_agent_model_binding(&request.profile, true)
        .await
        .map_err(|e| AgentRunError::ConfigurationError(e.to_string()))?;

    let initial_model = request
        .model_id
        .clone()
        .or(config_file_model.clone())
        .or(configured_model.clone())
        .or(parent_thread.model_id.clone());

    let initial_provider = if let Some(model_id) = initial_model.as_deref() {
        repo.find_by_id(model_id)
            .await
            .map_err(|e| AgentRunError::ConfigurationError(e.to_string()))?
            .map(|model| model.provider)
    } else {
        parent_thread.provider_id.clone()
    };

    let prepared = match request.thread_id {
        Some(id) => PreparedSubagent {
            child_thread_id: id,
            child_display_name: String::new(),
        },
        None => {
            let spawned_by = request.call_id.as_deref();
            let child_rollout_path = executor
                .agent_persistence()
                .rollout_recorder()
                .rollout_path(0)
                .to_string_lossy()
                .to_string();
            let sibling_names = executor
                .agent_persistence()
                .threads()
                .list_children(parent.thread_id)
                .await
                .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?
                .into_iter()
                .filter_map(|thread| thread.display_name)
                .collect::<HashSet<_>>();
            let child_display_name = subagent_name_candidates(&sibling_names)
                .map_err(AgentRunError::ConfigurationError)?
                .into_iter()
                .next()
                .ok_or_else(|| {
                    AgentRunError::ConfigurationError(
                        "No available subagent names configured for current locale".to_string(),
                    )
                })?;
            let created = executor
                .agent_persistence()
                .threads()
                .create(crate::agent::rollout::projection::CreateThreadParams {
                    workspace_path: &parent_thread.workspace_path,
                    title: &request.description,
                    display_name: Some(child_display_name.as_str()),
                    agent_type: &request.profile,
                    parent_thread_id: Some(parent.thread_id),
                    spawned_by_tool_call_id: spawned_by,
                    rollout_path: &child_rollout_path,
                    worktree_path: None,
                    model_id: initial_model.as_deref(),
                    provider_id: initial_provider.as_deref(),
                })
                .await
                .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
            let meta = ThreadMeta {
                thread_id: created.id,
                workspace_path: parent_thread.workspace_path.clone(),
                title: request.description.clone(),
                agent_type: request.profile.clone(),
                parent_thread_id: Some(parent.thread_id),
                spawned_by_tool_call_id: spawned_by.map(ToOwned::to_owned),
                model_id: initial_model.clone(),
                provider_id: initial_provider.clone(),
                created_at: Utc::now(),
            };
            let rollout_path = executor
                .agent_persistence()
                .rollout_recorder()
                .ensure_thread_rollout(&meta)
                .await
                .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
            executor
                .agent_persistence()
                .threads()
                .update_rollout_path(created.id, &rollout_path.to_string_lossy())
                .await
                .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;
            PreparedSubagent {
                child_thread_id: created.id,
                child_display_name,
            }
        }
    };

    let child_thread_id = prepared.child_thread_id;
    let child_thread = executor
        .agent_persistence()
        .threads()
        .get(child_thread_id)
        .await
        .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?
        .ok_or_else(|| {
            AgentRunError::StatePersistenceFailed(format!(
                "child thread {child_thread_id} not found"
            ))
        })?;

    if child_thread.parent_thread_id != Some(parent.thread_id) {
        return Err(AgentRunError::ConfigurationError(
            "Child thread does not belong to parent".to_string(),
        ));
    }

    executor.increment_active_child_executions_for_parent(parent.run_id.as_ref());

    Ok(PreparedSubagent {
        child_thread_id,
        child_display_name: if prepared.child_display_name.is_empty() {
            child_thread.display_name.clone().ok_or_else(|| {
                AgentRunError::ConfigurationError(
                    "Child thread is missing display_name".to_string(),
                )
            })?
        } else {
            prepared.child_display_name
        },
    })
}

async fn execute_prepared_subagent(
    executor: &AgentRunExecutor,
    parent: &AgentRunContext,
    request: SubAgentRequest,
    prepared: &PreparedSubagent,
) -> AgentRunResult<SubAgentResponse> {
    let workspace_root = PathBuf::from(parent.cwd.as_ref());

    let parent_thread = executor
        .agent_persistence()
        .threads()
        .get(parent.thread_id)
        .await
        .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?
        .ok_or_else(|| {
            AgentRunError::StatePersistenceFailed(format!(
                "parent thread {} not found",
                parent.thread_id
            ))
        })?;

    let agent_configs = AgentConfigLoader::load_for_workspace(&workspace_root)
        .await
        .map_err(|e| {
            AgentRunError::ConfigurationError(format!("Failed to load agent configs: {e}"))
        })?;
    let Some(sub_cfg) = agent_configs.get(request.profile.as_str()) else {
        return Err(AgentRunError::ConfigurationError(format!(
            "Unknown task profile: {}",
            request.profile
        )));
    };

    // Model priority: request override > child session > config file binding > profile frontmatter > parent session
    let child_thread_id = prepared.child_thread_id;
    let database = executor.database();
    let repo = AIModels::new(database.as_ref());
    let configured_model = sub_cfg.model_id.as_ref().and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let config_file_model = repo
        .get_agent_model_binding(&request.profile, true)
        .await
        .map_err(|e| AgentRunError::ConfigurationError(e.to_string()))?;
    let child_thread = executor
        .agent_persistence()
        .threads()
        .get(child_thread_id)
        .await
        .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?
        .ok_or_else(|| {
            AgentRunError::StatePersistenceFailed(format!(
                "child thread {child_thread_id} not found"
            ))
        })?;
    let model_id = request
        .model_id
        .clone()
        .or(child_thread.model_id.clone())
        .or(config_file_model.clone())
        .or(configured_model.clone())
        .or(parent_thread.model_id.clone())
        .ok_or_else(|| {
            AgentRunError::ConfigurationError(
                "No model_id set on thread; cannot run sub-agent".to_string(),
            )
        })?;

    let model_provider = repo
        .find_by_id(&model_id)
        .await
        .map_err(|e| AgentRunError::ConfigurationError(e.to_string()))?
        .map(|model| model.provider);

    executor
        .agent_persistence()
        .threads()
        .update_model_selection(child_thread_id, &model_id, model_provider.as_deref())
        .await
        .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

    let effective = executor
        .settings_manager()
        .get_effective_settings(Some(workspace_root.clone()))
        .await
        .map_err(|e| AgentRunError::ConfigurationError(e.to_string()))?;

    let workspace_settings = executor
        .settings_manager()
        .get_workspace_settings(&workspace_root)
        .await
        .map_err(|e| AgentRunError::ConfigurationError(e.to_string()))?;
    if let Err(err) = executor
        .mcp_registry()
        .init_workspace_servers(&workspace_root, &effective, workspace_settings.as_ref())
        .await
    {
        tracing::warn!(
            workspace = %workspace_root.display(),
            "Failed to initialize MCP workspace servers for child execution: {}",
            err
        );
    }

    let mcp_tools = executor
        .mcp_registry()
        .get_tools_for_workspace(parent.cwd.as_ref())
        .into_iter()
        .map(|t| Arc::new(t) as Arc<dyn RunnableTool>)
        .collect::<Vec<_>>();

    // Tool access is controlled entirely by each agent's frontmatter configuration.
    // Level-1 (general) agents may spawn Level-2 functional agents via task permissions.
    // Level-2 functional agents have task excluded from their tool_filter already.
    let merged_tool_filter = sub_cfg.tool_filter.clone();

    // Initialize Skill system (shares the same workspace as parent)
    let skill_manager = {
        let manager = Arc::new(crate::agent::skill::SkillManager::new());
        let global_skills_dir = crate::config::paths::skills_dir();

        if let Err(e) = manager
            .discover_skills(Some(&global_skills_dir), Some(&workspace_root))
            .await
        {
            tracing::warn!("Failed to discover skills for child execution: {}", e);
        }

        Some(manager)
    };

    let tool_registry = crate::agent::tools::create_tool_registry(
        "agent",
        effective.permissions,
        Some(merged_tool_filter),
        executor.tool_confirmations(),
        mcp_tools,
        executor.vector_search_engine(),
        Some(executor.lsp_manager()),
        skill_manager,
    )
    .await;

    let run_id = format!("task_{}", uuid::Uuid::new_v4());

    // Worktree isolation: create a dedicated git worktree branch for this child execution.
    // This allows parallel execution branches to modify files without conflicts.
    let cwd = if request.use_worktree {
        let repo_root = match git_service::find_repo_root(parent.cwd.as_ref()).await {
            Some(repo_root) => repo_root,
            None => {
                tracing::warn!(
                    "Failed to find git repo root for child execution parent cwd '{}', using cwd directly",
                    parent.cwd
                );
                parent.cwd.to_string()
            }
        };
        let branch = format!("orbitx/task-{child_thread_id}");
        let wt_dir = format!("{repo_root}/.git/orbitx-worktrees/{child_thread_id}");
        match crate::git::service::GitService::worktree_add(&repo_root, &branch, &wt_dir).await {
            Ok(wt_path) => {
                if let Err(err) = executor
                    .agent_persistence()
                    .threads()
                    .set_worktree_path(child_thread_id, &wt_path)
                    .await
                {
                    tracing::warn!(
                        thread_id = child_thread_id,
                        worktree = %wt_path,
                        "Failed to persist child execution worktree path: {}",
                        err
                    );
                }
                tracing::info!(
                    thread_id = child_thread_id,
                    branch = %branch,
                    worktree = %wt_path,
                    "Created isolated worktree for child execution"
                );
                wt_path
            }
            Err(e) => {
                tracing::warn!(
                    thread_id = child_thread_id,
                    error = %e.message,
                    "Failed to create worktree, falling back to parent cwd"
                );
                parent.cwd.to_string()
            }
        }
    } else {
        parent.cwd.to_string()
    };

    let progress_channel = parent.progress_channel().await;

    let ctx = AgentRunContext::new(crate::agent::core::context::AgentRunContextInit {
        run_id,
        thread_id: child_thread_id,
        user_prompt: request.prompt.clone(),
        agent_type: request.profile.clone(),
        config: crate::agent::config::AgentRunConfig::default(),
        workspace_path: cwd,
        emit_task_events: false,
        // Mixed-view design: stream child agent tool/message events on the same channel, but
        // persist them to the child session. The UI merges sessions into one timeline.
        progress_channel,
        deps: AgentRunContextDeps {
            tool_registry: Arc::clone(&tool_registry),
            repositories: executor.database(),
            agent_persistence: executor.agent_persistence(),
            checkpoint_service: executor.checkpoint_service(),
            workspace_changes: executor.workspace_changes(),
            subagent_runner: Arc::new(executor.clone()),
            settings_manager: executor.settings_manager(),
        },
    })
    .await?;

    let ctx = Arc::new(ctx);

    if let Some((checkpoint_id, workspace_root)) = parent.active_checkpoint_handle().await {
        ctx.inherit_checkpoint(checkpoint_id, workspace_root).await;
    }

    let parent_cancel = parent.create_stream_cancel_token();
    let ctx_for_cancel = Arc::clone(&ctx);
    tokio::spawn(async move {
        parent_cancel.cancelled().await;
        ctx_for_cancel.abort();
    });

    executor
        .restore_thread_history(&ctx, child_thread_id, None)
        .await?;

    ctx.file_tracker().take_recent_agent_edits().await;

    if !request.resume_existing {
        ctx.initialize_message_track(&request.prompt, None, true)
            .await?;
    }

    let prompt_seed = if request.resume_existing {
        child_thread
            .first_user_message
            .clone()
            .or_else(|| (!request.prompt.trim().is_empty()).then_some(request.prompt.clone()))
            .ok_or_else(|| {
                AgentRunError::ConfigurationError(format!(
                    "Cannot resume child thread {child_thread_id} without prompt seed"
                ))
            })?
    } else {
        request.prompt.clone()
    };

    let prompts = executor
        .prompt_orchestrator()
        .build_task_prompts(
            ctx.thread_id,
            ctx.run_id.to_string(),
            &prompt_seed,
            ctx.agent_type.as_ref(),
            &ctx.cwd,
            &tool_registry,
            Some(&model_id),
        )
        .await?;
    ctx.set_system_prompt(prompts.instructions).await?;
    ctx.set_developer_context(prompts.developer_context).await?;
    if !request.resume_existing {
        ctx.add_user_message(prompts.user_prompt).await?;
    }
    ctx.set_status(crate::agent::core::status::AgentTaskStatus::Running)
        .await?;

    // Child executions share the parent's event stream; they must also be reachable for tool confirmation
    // resolution (agent_tool_confirm looks up by run_id).
    executor
        .active_runs()
        .insert(ctx.run_id.to_string(), Arc::clone(&ctx));

    let task_key = ctx.run_id.to_string();
    let run_result = executor.run_task_loop(Arc::clone(&ctx), model_id).await;
    executor.active_runs().remove(&task_key);

    let (status, runtime_error) = match run_result {
        Ok(()) => (SubagentStatus::Completed, None),
        Err(AgentRunError::TaskInterrupted) | Err(AgentRunError::TaskCancelled(_)) => {
            (SubagentStatus::Cancelled, None)
        }
        Err(e) => (SubagentStatus::Error, Some(e)),
    };

    let messages = executor
        .agent_persistence()
        .messages()
        .list_by_thread(child_thread_id)
        .await
        .map_err(|e| AgentRunError::StatePersistenceFailed(e.to_string()))?;

    // Do NOT write partial output back into the parent context on cancellation.
    // Cancellation is an incomplete run; the parent task will backfill a real summary on next turn.
    let summary = match status {
        SubagentStatus::Cancelled => None,
        SubagentStatus::Error => match runtime_error {
            Some(error) => Some(error.to_string()),
            None => extract_last_assistant_text(&messages).map(|text| truncate_chars(&text, 1200)),
        },
        SubagentStatus::Completed => {
            extract_last_assistant_text(&messages).map(|text| truncate_chars(&text, 2000))
        }
        SubagentStatus::Pending | SubagentStatus::Running => None,
    };

    Ok(SubAgentResponse {
        thread_id: child_thread_id,
        status,
        summary,
    })
}

fn extract_last_assistant_text(messages: &[crate::agent::types::Message]) -> Option<String> {
    for msg in messages.iter().rev() {
        if !matches!(msg.role, MessageRole::Assistant) {
            continue;
        }
        let mut parts = Vec::new();
        for block in &msg.blocks {
            match block {
                Block::Text(b) => {
                    let cleaned = clean_subagent_text(&b.content);
                    if !cleaned.trim().is_empty() {
                        parts.push(cleaned);
                    }
                }
                Block::Error(b) => {
                    let cleaned = clean_subagent_text(&b.message);
                    if !cleaned.trim().is_empty() {
                        parts.push(cleaned);
                    }
                }
                _ => {}
            }
        }
        let out = parts.join("\n").trim().to_string();
        if !out.is_empty() {
            return Some(out);
        }
    }
    None
}

fn clean_subagent_text(input: &str) -> String {
    input
        .lines()
        .filter(|line| {
            let t = line.trim();
            if t.is_empty() {
                return false;
            }
            // Drop pseudo tool tags that some models emit as plain text.
            if t.starts_with('<') && t.ends_with('>') {
                return false;
            }
            true
        })
        .collect::<Vec<_>>()
        .join("\n")
}
