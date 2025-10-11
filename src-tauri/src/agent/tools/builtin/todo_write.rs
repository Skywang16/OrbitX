use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, warn};

use crate::agent::core::context::TaskContext;
use crate::agent::error::{AgentError, ToolExecutorResult};
use crate::agent::tools::{RunnableTool, ToolPermission, ToolResult, ToolResultContent};

/// TodoWrite工具 - 用于任务规划和进度跟踪
///

/// - 强制多步骤任务先规划再执行
/// - 提供用户可见的进度反馈
/// - 一次只能有一个in_progress任务（避免并发混乱）
pub struct TodoWriteTool;

impl TodoWriteTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

impl TodoStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TodoStatus::Pending => "pending",
            TodoStatus::InProgress => "in_progress",
            TodoStatus::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoItem {
    /// 任务描述（祈使句形式，如"Run tests"）
    pub content: String,
    /// 任务进行时描述（现在进行时，如"Running tests"）
    pub active_form: String,
    /// 任务状态
    pub status: TodoStatus,
}

#[derive(Debug, Deserialize)]
struct TodoWriteInput {
    todos: Vec<TodoItem>,
}

#[async_trait]
impl RunnableTool for TodoWriteTool {
    fn name(&self) -> &str {
        "todo_write"
    }

    fn description(&self) -> &str {
        r#"Create and manage a structured task list for the current execution.

## When to Use This Tool

Use this tool proactively in these scenarios:

1. Complex multi-step tasks - When a task requires 3 or more distinct steps or actions
2. Non-trivial and complex tasks - Tasks that require careful planning or multiple operations
3. User explicitly requests todo list - When the user directly asks you to use the todo list
4. User provides multiple tasks - When users provide a list of things to be done (numbered or comma-separated)
5. After receiving new instructions - Immediately capture user requirements as todos
6. When you start working on a task - Mark it as in_progress BEFORE beginning work
7. After completing a task - Mark it as completed and add any new follow-up tasks discovered during implementation

## When NOT to Use This Tool

Skip using this tool when:
1. There is only a single, straightforward task
2. The task is trivial and tracking it provides no organizational benefit
3. The task can be completed in less than 3 trivial steps
4. The task is purely conversational or informational

## Task States

- pending: Task not yet started
- in_progress: Currently working (limit to ONE task at a time)
- completed: Task finished successfully

IMPORTANT: Task descriptions must have two forms:
- content: Imperative form (e.g., "Run tests", "Fix authentication bug")
- activeForm: Present continuous (e.g., "Running tests", "Fixing authentication bug")

## Task Completion Requirements

ONLY mark a task as completed when you have FULLY accomplished it.
If you encounter errors, blockers, or cannot finish, keep the task as in_progress.

Never mark as completed if:
- Tests are failing
- Implementation is partial
- You encountered unresolved errors
- You couldn't find necessary files or dependencies

## Task Management Rules

- Mark exactly ONE task as in_progress at a time
- Complete tasks immediately when finished
- Add new tasks if you discover additional work during execution
- Remove tasks that are no longer relevant"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "description": "List of todo items with their current status",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "Task description in imperative form (e.g., 'Run tests')"
                            },
                            "activeForm": {
                                "type": "string",
                                "description": "Task description in present continuous form (e.g., 'Running tests')"
                            },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "in_progress", "completed"],
                                "description": "Current status of the task"
                            }
                        },
                        "required": ["content", "activeForm", "status"]
                    }
                }
            },
            "required": ["todos"]
        })
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem] // 使用FileSystem权限（最接近的选项）
    }

    async fn run(
        &self,
        _context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let input: TodoWriteInput = serde_json::from_value(args).map_err(|e| {
            AgentError::Internal(format!("Failed to parse TodoWrite parameters: {}", e))
        })?;

        // 验证：只能有一个in_progress任务
        let in_progress_count = input
            .todos
            .iter()
            .filter(|t| matches!(t.status, TodoStatus::InProgress))
            .count();

        if in_progress_count > 1 {
            warn!(
                "TodoWrite validation failed: {} tasks marked as in_progress (only 1 allowed)",
                in_progress_count
            );
            return Ok(ToolResult {
                content: vec![ToolResultContent::Error(format!(
                    "Only one task can be in_progress at a time. You marked {} tasks as in_progress.",
                    in_progress_count
                ))],
                is_error: true,
                execution_time_ms: Some(0),
                ext_info: None,
            });
        }

        // 验证：content 和 activeForm 不能为空
        for (idx, item) in input.todos.iter().enumerate() {
            if item.content.trim().is_empty() {
                return Ok(ToolResult {
                    content: vec![ToolResultContent::Error(format!(
                        "Task {} has empty content field",
                        idx + 1
                    ))],
                    is_error: true,
                    execution_time_ms: Some(0),
                    ext_info: None,
                });
            }
            if item.active_form.trim().is_empty() {
                return Ok(ToolResult {
                    content: vec![ToolResultContent::Error(format!(
                        "Task {} has empty activeForm field",
                        idx + 1
                    ))],
                    is_error: true,
                    execution_time_ms: Some(0),
                    ext_info: None,
                });
            }
        }

        debug!(
            "TodoWrite: updating {} todos, {} pending, {} in_progress, {} completed",
            input.todos.len(),
            input
                .todos
                .iter()
                .filter(|t| matches!(t.status, TodoStatus::Pending))
                .count(),
            in_progress_count,
            input
                .todos
                .iter()
                .filter(|t| matches!(t.status, TodoStatus::Completed))
                .count()
        );

        // TODO: 持久化到数据库
        // 当前简化实现：只返回成功，实际存储由上层TaskContext处理

        // 构建用户可见的摘要
        let summary = input
            .todos
            .iter()
            .map(|t| {
                let status_icon = match t.status {
                    TodoStatus::Pending => "⏳",
                    TodoStatus::InProgress => "🔄",
                    TodoStatus::Completed => "✅",
                };
                format!("{} {}", status_icon, t.content)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let result_message = format!(
            "Todo list updated ({} tasks):\n{}",
            input.todos.len(),
            summary
        );

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(result_message)],
            is_error: false,
            execution_time_ms: Some(0),
            ext_info: Some(json!({
                "todos": input.todos,
                "in_progress_count": in_progress_count,
            })),
        })
    }
}
