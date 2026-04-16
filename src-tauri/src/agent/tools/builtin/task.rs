use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::agent::core::context::{AgentRunContext, SubAgentRequest};
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::subagent::{CreateSubagentParams, SubagentRepository, UpdateSubagentParams};
use crate::agent::tools::metadata::{ExecutionMode, ToolCategory, ToolMetadata, ToolPriority};
use crate::agent::tools::{
    RunnableTool, ToolDescriptionContext, ToolResult, ToolResultContent, ToolResultStatus,
};
use crate::agent::types::{AgentRunEvent, SubagentStatus};
use crate::agent::utils::subagent_name_candidates;
use chrono::Utc;

#[derive(Default)]
pub struct TaskTool;

impl TaskTool {
    pub fn new() -> Self {
        Self
    }
}

fn subagent_error_message(status: &SubagentStatus, summary: Option<&str>) -> Option<String> {
    let _ = status;
    summary
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}

#[async_trait]
impl RunnableTool for TaskTool {
    fn name(&self) -> &str {
        "task"
    }

    fn description(&self) -> &str {
        "Run a delegated task with an authorized agent profile. Blocks until completion and returns the result."
    }

    fn description_with_context(&self, context: &ToolDescriptionContext) -> Option<String> {
        if context.allowed_subagent_types.is_empty() {
            return Some(
                "Task delegation is unavailable because no helper profiles are authorized for this agent."
                    .to_string(),
            );
        }
        Some(format!(
            "Run a delegated task using an authorized child-agent profile. Blocks until the child finishes and returns its output. Allowed profiles: {}.\n\nTo run multiple tasks in parallel, emit multiple task tool calls in a single response.",
            context.allowed_subagent_types.join(", ")
        ))
    }

    fn parameters_schema(&self) -> Value {
        json!({
          "type": "object",
          "properties": {
            "description": { "type": "string", "description": "Short label (3-5 words)" },
            "prompt": { "type": "string", "description": "Full instructions for the delegated task" },
            "profile": { "type": "string", "description": "Execution profile to use" },
            "model": { "type": "string", "description": "Optional model id override" },
            "use_worktree": { "type": "boolean", "description": "Create an isolated git worktree. Defaults to false." }
          },
          "required": ["description", "prompt", "profile"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Delegation, ToolPriority::Expensive)
            .with_execution_mode(ExecutionMode::Parallel)
            .with_timeout(Duration::from_secs(30 * 60))
            .with_summary_key_arg("description")
            .with_tags(vec!["ui:hidden".into(), "orchestration".into()])
    }

    async fn run(&self, context: &AgentRunContext, args: Value) -> ToolExecutorResult<ToolResult> {
        let description = required_string_arg(&args, "description")?;
        let prompt = required_string_arg(&args, "prompt")?;
        let profile = required_string_arg(&args, "profile")?;
        let model_id = args
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let use_worktree = args
            .get("use_worktree")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let call_id = uuid::Uuid::new_v4().to_string();

        let persistence = context.agent_persistence();
        let parent_thread = persistence
            .threads()
            .get(context.thread_id)
            .await
            .map_err(as_exec_error(self.name()))?
            .ok_or_else(|| ToolExecutorError::ExecutionFailed {
                tool_name: self.name().to_string(),
                error: format!("parent thread {} not found", context.thread_id),
            })?;

        let child_rollout_path = persistence
            .rollout_recorder()
            .rollout_path(0)
            .to_string_lossy()
            .to_string();
        let child_thread = persistence
            .threads()
            .create(crate::agent::rollout::projection::CreateThreadParams {
                workspace_path: &parent_thread.workspace_path,
                title: &description,
                display_name: None,
                thread_type: "agent",
                agent_type: &profile,
                parent_thread_id: Some(context.thread_id),
                spawned_by_tool_call_id: Some(&call_id),
                rollout_path: &child_rollout_path,
                worktree_path: None,
                model_id: parent_thread.model_id.as_deref(),
                provider_id: parent_thread.provider_id.as_deref(),
            })
            .await
            .map_err(as_exec_error(self.name()))?;
        let child_thread_id = child_thread.id;

        let parent_message_id = context
            .current_assistant_message_id()
            .await
            .ok_or_else(|| ToolExecutorError::ExecutionFailed {
                tool_name: self.name().to_string(),
                error: "assistant message not initialized for subagent creation".to_string(),
            })?;
        let subagent_repo = SubagentRepository::new(context.repositories());
        let existing_names = subagent_repo
            .list_by_parent_thread(context.thread_id)
            .await
            .map_err(as_exec_error(self.name()))?
            .into_iter()
            .map(|subagent| subagent.name)
            .collect::<std::collections::HashSet<_>>();
        let candidates = subagent_name_candidates(&existing_names).map_err(|error| {
            ToolExecutorError::ExecutionFailed {
                tool_name: self.name().to_string(),
                error,
            }
        })?;
        let mut created_subagent = None;
        for candidate in candidates {
            match subagent_repo
                .create(CreateSubagentParams {
                    id: &call_id,
                    parent_thread_id: context.thread_id,
                    child_thread_id,
                    parent_message_id,
                    name: &candidate,
                    profile: &profile,
                    task_title: &description,
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
                Err(err) => {
                    return Err(ToolExecutorError::ExecutionFailed {
                        tool_name: self.name().to_string(),
                        error: err.to_string(),
                    });
                }
            }
        }
        let created_subagent =
            created_subagent.ok_or_else(|| ToolExecutorError::ExecutionFailed {
                tool_name: self.name().to_string(),
                error: "failed to allocate a unique subagent name".to_string(),
            })?;
        persistence
            .threads()
            .update_display_name(child_thread_id, &created_subagent.name)
            .await
            .map_err(as_exec_error(self.name()))?;
        context
            .emit_event(AgentRunEvent::SubagentCreated {
                run_id: context.run_id.to_string(),
                subagent: created_subagent.clone(),
            })
            .await
            .map_err(|e| ToolExecutorError::ExecutionFailed {
                tool_name: self.name().to_string(),
                error: e.to_string(),
            })?;

        let request = SubAgentRequest {
            description: description.clone(),
            prompt: prompt.clone(),
            profile: profile.clone(),
            thread_id: Some(child_thread_id),
            call_id: Some(call_id.clone()),
            resume_existing: false,
            model_id,
            use_worktree,
        };

        let response = context
            .subagent_runner()
            .run_subagent(context, request)
            .await
            .map_err(|e| ToolExecutorError::ExecutionFailed {
                tool_name: self.name().to_string(),
                error: e.to_string(),
            })?;

        let updated_subagent = subagent_repo
            .update(
                &call_id,
                UpdateSubagentParams {
                    status: Some(match response.status {
                        SubagentStatus::Pending => SubagentStatus::Pending,
                        SubagentStatus::Running => SubagentStatus::Running,
                        SubagentStatus::Completed => SubagentStatus::Completed,
                        SubagentStatus::Cancelled => SubagentStatus::Cancelled,
                        SubagentStatus::Error => SubagentStatus::Error,
                    }),
                    final_summary: Some(response.summary.clone()),
                    error_message: Some(
                        (response.status == SubagentStatus::Error)
                            .then(|| {
                                subagent_error_message(
                                    &response.status,
                                    response.summary.as_deref(),
                                )
                            })
                            .flatten(),
                    ),
                    finished_at: Some(Some(Utc::now())),
                    ..UpdateSubagentParams::default()
                },
            )
            .await
            .map_err(|e| ToolExecutorError::ExecutionFailed {
                tool_name: self.name().to_string(),
                error: e.to_string(),
            })?;
        context
            .emit_event(AgentRunEvent::SubagentUpdated {
                run_id: context.run_id.to_string(),
                subagent: updated_subagent,
            })
            .await
            .map_err(|e| ToolExecutorError::ExecutionFailed {
                tool_name: self.name().to_string(),
                error: e.to_string(),
            })?;

        let response_status = response.status;
        let summary = response.summary.filter(|s| !s.trim().is_empty());

        let (status, content) = match (response_status, summary) {
            (SubagentStatus::Completed, Some(summary)) => (
                ToolResultStatus::Success,
                ToolResultContent::Success(summary),
            ),
            (SubagentStatus::Completed, None) => (
                ToolResultStatus::Error,
                ToolResultContent::Error("Task finished without returning a summary.".to_string()),
            ),
            (SubagentStatus::Cancelled, s) => (
                ToolResultStatus::Cancelled,
                ToolResultContent::Error(s.unwrap_or_else(|| "Task was cancelled.".to_string())),
            ),
            (status, s) => (
                ToolResultStatus::Error,
                ToolResultContent::Error(
                    s.unwrap_or_else(|| format!("Task returned status {status:?}.")),
                ),
            ),
        };

        Ok(ToolResult {
            content: vec![content],
            status,
            cancel_reason: None,
            execution_time_ms: None,
            ext_info: Some(json!({
                "child_thread_id": response.thread_id,
                "profile": profile,
            })),
        })
    }
}

fn required_string_arg(args: &Value, key: &str) -> ToolExecutorResult<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| ToolExecutorError::InvalidArguments {
            tool_name: "task".to_string(),
            error: format!("task requires non-empty `{key}`"),
        })
}

fn as_exec_error(
    tool_name: &str,
) -> impl Fn(crate::agent::error::AgentError) -> ToolExecutorError + '_ {
    move |e| ToolExecutorError::ExecutionFailed {
        tool_name: tool_name.to_string(),
        error: e.to_string(),
    }
}
