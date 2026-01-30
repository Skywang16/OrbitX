use async_trait::async_trait;
use serde_json::{json, Value};
use std::time::Duration;

use crate::agent::core::context::{SubtaskRequest, TaskContext};
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::tools::metadata::{ToolCategory, ToolMetadata, ToolPriority};
use crate::agent::tools::{RunnableTool, ToolResult, ToolResultContent, ToolResultStatus};
use crate::agent::types::{Block, SubtaskBlock, SubtaskStatus};

#[derive(Default)]
pub struct TaskTool;

impl TaskTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for TaskTool {
    fn name(&self) -> &str {
        "task"
    }

    fn description(&self) -> &str {
        "Run a subtask in a child session using a subagent, returning a short summary."
    }

    fn parameters_schema(&self) -> Value {
        json!({
          "type": "object",
          "properties": {
            "description": { "type": "string", "description": "Short label (3-5 words)" },
            "prompt": { "type": "string", "description": "Full instructions for the subagent" },
            "subagent_type": { "type": "string", "description": "Subagent type (explore/general/research)" },
            "session_id": { "type": "number", "description": "Optional existing child session id" },
            "call_id": { "type": "string", "description": "Tool call id (injected by runtime)" }
          },
          "required": ["description", "prompt", "subagent_type"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::CodeAnalysis, ToolPriority::Expensive)
            .with_timeout(Duration::from_secs(30 * 60))
            .with_summary_key_arg("description")
            .with_tags(vec!["ui:hidden".into(), "orchestration".into()])
    }

    async fn run(&self, context: &TaskContext, args: Value) -> ToolExecutorResult<ToolResult> {
        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let prompt = args
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let subagent_type = args
            .get("subagent_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let session_id = args.get("session_id").and_then(|v| v.as_i64());
        let call_id = args
            .get("call_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        if description.is_empty() || prompt.is_empty() || subagent_type.is_empty() {
            return Err(ToolExecutorError::InvalidArguments {
                tool_name: "task".to_string(),
                error: "task requires description, prompt, subagent_type".to_string(),
            });
        }

        let persistence = context.agent_persistence();
        let parent_session = persistence
            .sessions()
            .get(context.session_id)
            .await
            .map_err(|e| ToolExecutorError::ExecutionFailed {
                tool_name: "task".to_string(),
                error: e.to_string(),
            })?
            .ok_or_else(|| ToolExecutorError::ExecutionFailed {
                tool_name: "task".to_string(),
                error: format!("parent session {} not found", context.session_id),
            })?;

        let child_session_id = match session_id {
            Some(id) => id,
            None => {
                let created = persistence
                    .sessions()
                    .create(
                        &parent_session.workspace_path,
                        Some(&description),
                        &subagent_type,
                        Some(parent_session.id),
                        Some(&call_id),
                        parent_session.model_id.as_deref(),
                        parent_session.provider_id.as_deref(),
                    )
                    .await
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "task".to_string(),
                        error: e.to_string(),
                    })?;
                created.id
            }
        };

        context
            .assistant_append_block(Block::Subtask(SubtaskBlock {
                id: call_id.clone(),
                child_session_id,
                agent_type: subagent_type.clone(),
                description: description.clone(),
                status: SubtaskStatus::Running,
                summary: None,
            }))
            .await
            .map_err(|e| ToolExecutorError::ExecutionFailed {
                tool_name: "task".to_string(),
                error: e.to_string(),
            })?;

        let response = match context
            .subtask_runner()
            .run_subtask(
                context,
                SubtaskRequest {
                    description: description.clone(),
                    prompt: prompt.clone(),
                    subagent_type: subagent_type.clone(),
                    session_id: Some(child_session_id),
                    call_id: Some(call_id.clone()),
                },
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let _ = context
                    .assistant_update_block(
                        &call_id,
                        Block::Subtask(SubtaskBlock {
                            id: call_id.clone(),
                            child_session_id,
                            agent_type: subagent_type.clone(),
                            description: description.clone(),
                            status: SubtaskStatus::Error,
                            summary: Some(e.to_string()),
                        }),
                    )
                    .await;

                return Err(ToolExecutorError::ExecutionFailed {
                    tool_name: "task".to_string(),
                    error: e.to_string(),
                });
            }
        };

        context
            .assistant_update_block(
                &call_id,
                Block::Subtask(SubtaskBlock {
                    id: call_id.clone(),
                    child_session_id: response.session_id,
                    agent_type: subagent_type.clone(),
                    description: description.clone(),
                    status: response.status.clone(),
                    summary: response.summary.clone(),
                }),
            )
            .await
            .map_err(|e| ToolExecutorError::ExecutionFailed {
                tool_name: "task".to_string(),
                error: e.to_string(),
            })?;

        let (status, cancel_reason, content) = match response.status {
            SubtaskStatus::Completed => (
                ToolResultStatus::Success,
                None,
                ToolResultContent::Success(
                    response
                        .summary
                        .clone()
                        .unwrap_or_else(|| "Subtask completed.".to_string()),
                ),
            ),
            SubtaskStatus::Cancelled => (
                ToolResultStatus::Cancelled,
                Some("cancelled".to_string()),
                ToolResultContent::Error(
                    response
                        .summary
                        .clone()
                        .unwrap_or_else(|| "Subtask cancelled (summary pending).".to_string()),
                ),
            ),
            SubtaskStatus::Pending | SubtaskStatus::Running | SubtaskStatus::Error => (
                ToolResultStatus::Error,
                None,
                ToolResultContent::Error(
                    response
                        .summary
                        .clone()
                        .unwrap_or_else(|| "Subtask failed.".to_string()),
                ),
            ),
        };

        Ok(ToolResult {
            content: vec![content],
            status,
            cancel_reason,
            execution_time_ms: None,
            ext_info: Some(json!({
                "child_session_id": response.session_id,
                "subagent_type": subagent_type,
            })),
        })
    }
}
