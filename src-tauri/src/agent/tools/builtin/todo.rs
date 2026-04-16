//! TodoWrite tool — incremental task tracking with stable IDs.
//!
//! Supports `merge` mode: when true, only the supplied items are upserted into
//! the existing list (matched by `id`); when false the new list replaces the old
//! one entirely.  This avoids re-sending the full list on every status update
//! and reduces the chance of the LLM accidentally dropping items.
//!
//! State lives in `Arc<RwLock<..>>` inside the tool instance.  Each tool
//! registry (i.e. each agent session) gets its own TodoState — that's correct
//! because TODO lists are per-conversation.  The state is *not* persisted to DB;
//! it only exists in the chat history as tool-result messages.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent::core::context::AgentRunContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority as MetaPriority, ToolResult,
    ToolResultContent, ToolResultStatus,
};

const DESCRIPTION: &str = r#"Create and manage a structured task list for your current coding session.

## When to Use
1. Complex multi-step tasks (3+ distinct steps)
2. Non-trivial tasks requiring careful planning
3. User explicitly requests a todo list
4. User provides multiple tasks (numbered/comma-separated)
5. After receiving new instructions — capture requirements as todos (use merge=false to add new ones)
6. After completing tasks — mark complete with merge=true and add follow-ups
7. When starting new tasks — mark as in_progress (ideally only one at a time)

## When NOT to Use
1. Single, straightforward tasks
2. Trivial tasks with no organizational benefit
3. Tasks completable in < 3 trivial steps
4. Purely conversational/informational requests

## Rules
1. Every item MUST have a unique `id` (short, stable string).
2. Keep IDs stable across updates — change status/content, never churn IDs.
3. At most ONE item can be `in_progress` at a time.
4. Use `merge: true` to update specific items without resending the whole list.
5. Use `merge: false` to replace the entire list.
6. You can leave unchanged properties undefined when merge=true.
7. Keep the list small and actionable (prefer ≤ 10 items).

## Status Values
- **pending**: Not yet started
- **in_progress**: Currently working on (only one allowed)
- **completed**: Finished successfully
- **cancelled**: No longer needed"#;

// ── Domain types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl Status {
    fn parse(s: &str) -> Result<Self, String> {
        match s {
            "pending" => Ok(Self::Pending),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!(
                "Invalid todo status '{s}'. Expected one of: pending, in_progress, completed, cancelled"
            )),
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Pending => "○",
            Self::InProgress => "▶",
            Self::Completed => "✓",
            Self::Cancelled => "✗",
        }
    }
}

#[derive(Debug, Clone)]
struct TodoItem {
    id: String,
    content: String,
    status: Status,
}

// ── State ────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct TodoState {
    items: Vec<TodoItem>,
}

impl TodoState {
    fn apply(&mut self, incoming: Vec<TodoItemInput>, merge: bool) -> Result<(), String> {
        if !merge {
            self.items = incoming
                .into_iter()
                .map(TodoItemInput::into_required_item)
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(());
        }

        // Merge: upsert by id, preserve existing order, append new items.
        let idx_by_id: HashMap<String, usize> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, t)| (t.id.clone(), i))
            .collect();

        for input in incoming {
            if let Some(&i) = idx_by_id.get(&input.id) {
                if let Some(c) = input.content {
                    let content = c.trim();
                    if content.is_empty() {
                        return Err(format!("Todo '{}' content cannot be empty", input.id));
                    }
                    self.items[i].content = content.to_string();
                }
                if let Some(s) = input.status {
                    self.items[i].status = Status::parse(s.trim())?;
                }
            } else {
                self.items.push(input.into_required_item()?);
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), &'static str> {
        let n = self
            .items
            .iter()
            .filter(|t| t.status == Status::InProgress)
            .count();
        if n > 1 {
            return Err("At most one todo can be in_progress at a time");
        }
        Ok(())
    }

    fn format(&self) -> String {
        let done = self
            .items
            .iter()
            .filter(|t| t.status == Status::Completed)
            .count();
        let active = self
            .items
            .iter()
            .filter(|t| t.status != Status::Cancelled)
            .count();

        let mut out = format!("Todo ({done}/{active})\n");
        for t in &self.items {
            out.push_str(&format!("{} {}\n", t.status.icon(), t.content));
        }
        out
    }

    fn summary_json(&self) -> serde_json::Value {
        let done = self
            .items
            .iter()
            .filter(|t| t.status == Status::Completed)
            .count();
        let total = self
            .items
            .iter()
            .filter(|t| t.status != Status::Cancelled)
            .count();
        json!({ "done": done, "total": total })
    }
}

// ── Deserialization ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TodoWriteArgs {
    todos: Vec<TodoItemInput>,
    #[serde(default)]
    merge: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TodoItemInput {
    id: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

impl TodoItemInput {
    fn into_required_item(self) -> Result<TodoItem, String> {
        let content = self
            .content
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("Todo '{}' requires non-empty content", self.id))?;
        let status = self
            .status
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("Todo '{}' requires status", self.id))
            .and_then(Status::parse)?;

        Ok(TodoItem {
            id: self.id,
            content: content.to_string(),
            status,
        })
    }
}

// ── Tool ─────────────────────────────────────────────────────────────────

pub struct TodoWriteTool {
    state: Arc<RwLock<TodoState>>,
}

impl Default for TodoWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoWriteTool {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(TodoState::default())),
        }
    }
}

#[async_trait]
impl RunnableTool for TodoWriteTool {
    fn name(&self) -> &str {
        "todowrite"
    }

    fn description(&self) -> &str {
        DESCRIPTION
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "description": "Array of TODO items to update or create",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id":      { "type": "string", "description": "Unique identifier for this item" },
                            "content": { "type": "string", "description": "Description of the todo item" },
                            "status":  { "type": "string", "enum": ["pending", "in_progress", "completed", "cancelled"] }
                        },
                        "required": ["id"]
                    }
                },
                "merge": {
                    "type": "boolean",
                    "description": "If true, merge into existing todos by id. If false, replace entirely. Default: false."
                }
            },
            "required": ["todos"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::CodeAnalysis, MetaPriority::Standard)
            .with_tags(vec!["todo".into(), "planning".into()])
    }

    async fn run(
        &self,
        _context: &AgentRunContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: TodoWriteArgs = serde_json::from_value(args)?;

        let mut state = self.state.write().await;
        if let Err(msg) = state.apply(args.todos, args.merge) {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Error(msg)],
                status: ToolResultStatus::Error,
                cancel_reason: None,
                execution_time_ms: None,
                ext_info: None,
            });
        }

        if let Err(msg) = state.validate() {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Error(msg.to_string())],
                status: ToolResultStatus::Error,
                cancel_reason: None,
                execution_time_ms: None,
                ext_info: None,
            });
        }

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(state.format())],
            status: ToolResultStatus::Success,
            cancel_reason: None,
            execution_time_ms: None,
            ext_info: Some(state.summary_json()),
        })
    }
}
