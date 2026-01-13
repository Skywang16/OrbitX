//! TodoWrite tool - outputs todo list to chat history only
//!
//! Simple task tracking - just pending/completed states.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority as MetaPriority, ToolResult,
    ToolResultContent, ToolResultStatus,
};

const DESCRIPTION: &str = r#"Use this tool to create and manage a structured task list for your current coding session. This helps you track progress, organize complex tasks, and demonstrate thoroughness to the user.

## When to Use This Tool
Use this tool proactively in these scenarios:

1. **Complex multistep tasks** - When a task requires 3 or more distinct steps or actions
2. **Non-trivial and complex tasks** - Tasks that require careful planning or multiple operations  
3. **User explicitly requests todo list** - When the user directly asks you to use the todo list
4. **User provides multiple tasks** - When users provide a list of things to be done (numbered or comma-separated)
5. **After receiving new instructions** - Immediately capture user requirements as todos. Feel free to edit the todo list based on new information
6. **After completing a task** - Mark it complete and add any new follow-up tasks
7. **When starting work** - Mark the todo as in_progress. Ideally you should only have one todo as in_progress at a time

## When NOT to Use This Tool
Skip using this tool when:
1. **Single, straightforward task** - There is only one simple task to complete
2. **Trivial task** - The task provides no organizational benefit to track
3. **Less than 3 trivial steps** - The task can be completed in less than 3 simple steps
4. **Purely conversational** - The interaction is informational only

## Rules (follow strictly):
1. Submit the FULL list every time (this tool replaces the previous state)
2. Keep IDs stable across updates (edit status/content, do not churn IDs)
3. At most ONE item can be status=in_progress at a time
4. Keep the list small and actionable (prefer <= 7 items)

## Status Values:
- **pending**: Task not yet started
- **in_progress**: Currently working on this task (only one allowed)
- **completed**: Task finished successfully

## Examples of When to Use:

**Example 1: Multi-step Feature**
User: "I want to add a dark mode toggle to the application settings. Make sure you run the tests when you're done!"

Response: I'll help add a dark mode toggle. Let me create a todo list to track this implementation:
- Create dark mode toggle component in Settings page
- Add dark mode state management (context/store)
- Implement CSS styles for dark theme
- Update existing components to support theme switching
- Run tests and build process, addressing any failures

*Reasoning: Multi-step feature with explicit test requirement - perfect for todo tracking*

**Example 2: Multiple Related Tasks**
User: "Help me rename the function getCwd to getCurrentWorkingDirectory across my project"

Response: I'll help rename that function across your project. Let me search for all occurrences first and create a plan:
- Search for all occurrences of 'getCwd' in the codebase
- Update function definition in main file
- Update all function calls in dependent files
- Update any documentation or comments referencing the old name
- Run tests to ensure no breaking changes

*Reasoning: Systematic refactoring requiring multiple coordinated changes*

## Examples of When NOT to Use:

**Example 1: Simple Question**
User: "How do I print 'Hello World' in Python?"

Response: You can print 'Hello World' in Python using: `print("Hello World")`

*Reasoning: Single trivial task, no organizational benefit to track*

**Example 2: Single File Read**
User: "Can you read the main.py file?"

Response: I'll read the main.py file for you.

*Reasoning: Single straightforward operation, no planning needed*"#;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TodoWriteArgs {
    todos: Vec<TodoItemInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TodoItemInput {
    content: String,
    status: String,
}

pub struct TodoWriteTool;

impl Default for TodoWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoWriteTool {
    pub fn new() -> Self {
        Self
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
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": { "type": "string" },
                            "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] }
                        },
                        "required": ["content", "status"]
                    }
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
        _context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: TodoWriteArgs = serde_json::from_value(args)?;

        // Validate: at most one in_progress
        let in_progress_count = args
            .todos
            .iter()
            .filter(|t| t.status == "in_progress")
            .count();

        if in_progress_count > 1 {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Error(
                    "At most one todo can be in_progress".to_string(),
                )],
                status: ToolResultStatus::Error,
                cancel_reason: None,
                execution_time_ms: None,
                ext_info: None,
            });
        }

        // Format output
        let done = args
            .todos
            .iter()
            .filter(|t| t.status == "completed")
            .count();
        let total = args.todos.len();

        let mut output = format!("Todo ({done}/{total})\n");
        for t in &args.todos {
            let icon = match t.status.as_str() {
                "in_progress" => "▶",
                "completed" => "✓",
                _ => "○",
            };
            output.push_str(&format!("{} {}\n", icon, t.content));
        }

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(output)],
            status: ToolResultStatus::Success,
            cancel_reason: None,
            execution_time_ms: None,
            ext_info: Some(json!({ "done": done, "total": total })),
        })
    }
}
