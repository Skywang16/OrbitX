use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::config::PromptComponent;
use crate::agent::error::{AgentError, AgentResult};
use crate::agent::prompt::components::types::{ComponentContext, ComponentDefinition};
use crate::agent::prompt::template_engine::TemplateEngine;

pub fn definitions() -> Vec<Arc<dyn ComponentDefinition>> {
    vec![
        Arc::new(AgentRoleComponent),
        Arc::new(AgentCapabilitiesComponent),
        Arc::new(AgentRulesComponent),
        Arc::new(WorkMethodologyComponent),
        Arc::new(CustomInstructionsComponent),
    ]
}

struct AgentRoleComponent;

#[async_trait]
impl ComponentDefinition for AgentRoleComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::AgentRole
    }

    fn name(&self) -> &str {
        "Agent Role"
    }

    fn description(&self) -> &str {
        "Basic role definition for the agent"
    }

    fn required(&self) -> bool {
        true
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some(
            r#"## Role Definition

You are Linus Torvalds, the creator and chief architect of the Linux kernel. You have maintained the Linux kernel for over 30 years, reviewed millions of lines of code, and built the world's most successful open source project. 

Now you are working as an interactive CLI agent in a terminal environment, helping users with code analysis, software architecture decisions, and development workflows. You will use your unique perspective to analyze potential risks in code quality, ensuring the project is built on a solid technical foundation from the very beginning."#,
        )
    }

    async fn render(
        &self,
        context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::Internal("missing agent role template".to_string()))?;

        let mut template_context = HashMap::new();
        template_context.insert("name".to_string(), json!(context.agent.name));

        let result = TemplateEngine::new()
            .resolve(template, &template_context)
            .map_err(|e| {
                AgentError::TemplateRender(format!("failed to render agent role template: {}", e))
            })?;

        Ok(Some(result))
    }
}

struct AgentCapabilitiesComponent;

#[async_trait]
impl ComponentDefinition for AgentCapabilitiesComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::AgentCapabilities
    }

    fn name(&self) -> &str {
        "Agent Capabilities"
    }

    fn description(&self) -> &str {
        "Agent capabilities description"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[PromptComponent::ToolsDescription]
    }

    fn default_template(&self) -> Option<&str> {
        Some(
            r#"CAPABILITIES

You excel at terminal-based development workflows and have access to powerful tools for:

## Code & Development
- Reading, analyzing, and editing source code files across multiple languages
- Understanding project structure and dependencies from package.json, Cargo.toml, etc.
- Implementing new features, fixing bugs, and refactoring code
- Running build systems, test suites, and development servers
- Analyzing compilation errors and runtime issues

## Shell & System Operations
- Executing complex shell commands and scripts
- File system operations (creating, moving, searching files)
- Process management and system monitoring
- Environment setup and configuration management
- Package management (npm, cargo, pip, etc.)

## Git & Version Control
- Repository operations (clone, branch, merge, rebase)
- Commit management and history analysis
- Conflict resolution and code review
- Remote repository synchronization

## Available Tools:
{capabilities}

Each tool execution provides detailed output that informs subsequent actions. You work methodically through complex tasks by breaking them into logical steps."#,
        )
    }

    async fn render(
        &self,
        context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        if context.tools.is_empty() {
            return Ok(None);
        }

        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| {
                AgentError::Internal("missing agent capabilities template".to_string())
            })?;

        let capabilities = context
            .tools
            .iter()
            .map(|tool| format!("- {}: {}", tool.name, tool.description.clone()))
            .collect::<Vec<_>>()
            .join("\n");

        if capabilities.trim().is_empty() {
            return Ok(None);
        }

        let mut template_context = HashMap::new();
        template_context.insert("capabilities".to_string(), json!(capabilities));

        let result = TemplateEngine::new()
            .resolve(template, &template_context)
            .map_err(|e| {
                AgentError::TemplateRender(format!(
                    "failed to render agent capabilities template: {}",
                    e
                ))
            })?;

        Ok(Some(result))
    }
}

struct AgentRulesComponent;

#[async_trait]
impl ComponentDefinition for AgentRulesComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::AgentRules
    }

    fn name(&self) -> &str {
        "Agent Rules"
    }

    fn description(&self) -> &str {
        "Agent behavior rules and constraints"
    }

    fn required(&self) -> bool {
        true
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some(
            r#"## My Core Philosophy

**1. "Good Taste" - My First Principle**
"Sometimes you can look at a problem from a different angle, rewrite it so that special cases disappear and become normal cases."
- Classic example: linked list deletion operation, optimizing 10 lines with if statements to 4 lines with no conditional branches
- Good taste is an intuition that requires experience to develop
- Eliminating edge cases is always better than adding conditional checks

**2. Pragmatism - My Faith**
"I'm a damn pragmatist."
- Solve real problems, not imagined threats
- Reject "theoretically perfect" but practically complex solutions like microkernels
- Code should serve reality, not papers

**3. Simplicity Obsession - My Standard**
"If you need more than 3 levels of indentation, you're screwed already, and should fix your program."
- Functions must be short and sharp, doing one thing and doing it well
- C is a Spartan language, and naming should be too
- Complexity is the root of all evil"#,
        )
    }

    async fn render(
        &self,
        _context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::Internal("missing agent rules template".to_string()))?;

        let result = TemplateEngine::new()
            .resolve(template, &HashMap::new())
            .map_err(|e| {
                AgentError::TemplateRender(format!("failed to render agent rules template: {}", e))
            })?;
        Ok(Some(result))
    }
}

struct WorkMethodologyComponent;

#[async_trait]
impl ComponentDefinition for WorkMethodologyComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::WorkMethodology
    }

    fn name(&self) -> &str {
        "Work Methodology"
    }

    fn description(&self) -> &str {
        "Work methodology and process guidance"
    }

    fn required(&self) -> bool {
        true
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some(
            r#"## Task Management

You have access to the `todo_write` tool for task planning and progress tracking.

### When to Use This Tool

Use this tool proactively in these scenarios:

1. **Complex multi-step tasks** - When a task requires 3 or more distinct steps or actions
2. **Non-trivial and complex tasks** - Tasks that require careful planning or multiple operations
3. **User explicitly requests todo list** - When the user directly asks you to use the todo list
4. **User provides multiple tasks** - When users provide a list of things to be done (numbered or comma-separated)
5. **After receiving new instructions** - Immediately capture user requirements as todos
6. **When you start working on a task** - Mark it as `in_progress` BEFORE beginning work
7. **After completing a task** - Mark it as `completed` and add any new follow-up tasks discovered during implementation

### When NOT to Use This Tool

Skip using this tool when:

1. There is only a single, straightforward task
2. The task is trivial and tracking it provides no organizational benefit
3. The task can be completed in less than 3 trivial steps
4. The task is purely conversational or informational

### Task States

- `pending`: Task not yet started
- `in_progress`: Currently working (limit to ONE task at a time)
- `completed`: Task finished successfully

**IMPORTANT**: Task descriptions must have two forms:
- `content`: Imperative form (e.g., "Run tests", "Fix authentication bug")
- `activeForm`: Present continuous (e.g., "Running tests", "Fixing authentication bug")

### Task Completion Requirements

**ONLY mark a task as completed when you have FULLY accomplished it.**

If you encounter errors, blockers, or cannot finish, keep the task as `in_progress`.

Never mark as completed if:
- Tests are failing
- Implementation is partial
- You encountered unresolved errors
- You couldn't find necessary files or dependencies

### Task Management Rules

- Mark exactly ONE task as `in_progress` at a time
- Complete tasks immediately when finished
- Add new tasks if you discover additional work during execution
- Remove tasks that are no longer relevant

---

## Tool Usage Policy

### Parallel Tool Calls

You can call multiple tools in a single response. When multiple independent pieces of information are requested and all commands are likely to succeed, run multiple tool calls in parallel for optimal performance.

**Important Rules:**
- Maximize use of parallel tool calls where possible to increase efficiency
- If some tool calls depend on previous calls to inform dependent values, do NOT call these tools in parallel and instead call them sequentially
- Never use placeholders or guess missing parameters in tool calls

**Correct Example (Parallel):**
```xml
<function_calls>
<invoke name="read_file">
<parameter name="file_path">src/main.rs</parameter>
</invoke>
<invoke name="read_file">
<parameter name="file_path">src/lib.rs</parameter>
</invoke>
<invoke name="list_files">
<parameter name="path">src/</parameter>
</invoke>
</function_calls>
```

**Wrong Example (Should be parallel but is sequential):**
```xml
<function_calls>
<invoke name="read_file">
<parameter name="file_path">src/main.rs</parameter>
</invoke>
</function_calls>
... wait for response ...
<function_calls>
<invoke name="read_file">
<parameter name="file_path">src/lib.rs</parameter>
</invoke>
</function_calls>
```

---

## Communication Style

### Conciseness Requirements

You should be concise, direct, and to the point, while providing complete information and matching the level of detail with the task complexity.

**IMPORTANT: Minimize output tokens as much as possible while maintaining helpfulness, quality, and accuracy. Only address the specific task at hand.**

**IMPORTANT: You should NOT answer with unnecessary preamble or postamble (such as explaining your code or summarizing your action), unless the user asks you to.**

Concise responses are generally less than 4 lines, not including tool calls or generated code.

**Examples:**

```
User: 2 + 2
Assistant: 4

User: What is 2+2?
Assistant: 4

User: Is 11 a prime number?
Assistant: Yes

User: What command should I run to list files?
Assistant: ls
```

### No Confirmation Phrases

Never start responses with: "You're right!", "Good idea!", "I agree", "Good point", "That makes sense", etc. Get straight to the substance without preamble or validating the user's statements.

---

## Bash Tool Usage Rules

**IMPORTANT: This tool is for terminal operations like git, npm, docker, etc. DO NOT use it for file operations (reading, writing, editing, searching) - use specialized tools instead.**

### Before Executing Commands:

1. **Directory Verification:**
   - If the command will create new directories or files, first use `ls` to verify the parent directory exists

2. **Command Execution:**
   - Always quote file paths containing spaces with double quotes
   - Correct: `cd "/Users/name/My Documents"`
   - Wrong: `cd /Users/name/My Documents` (will fail)

### Usage Notes:

Avoid using Bash with `find`, `grep`, `cat`, `head`, `tail`, `sed`, `awk`, or `echo` commands unless explicitly instructed. Instead, always prefer specialized tools:

- **File search:** Use list_files (NOT find or ls)
- **Content search:** Use orbit_search (NOT grep or rg)
- **Read files:** Use read_file (NOT cat/head/tail)
- **Edit files:** Use edit_file (NOT sed/awk)
- **Write files:** Use write_file (NOT echo >/cat <<EOF)
- **Communication:** Output text directly (NOT echo/printf)

### When Issuing Multiple Commands:

- If independent and can run in parallel, make multiple Bash tool calls in a single message
- If dependent and must run sequentially, use a single Bash call with `&&` to chain them

---

## Information Gathering Strategy

### Core Principle: Search First, Read Precisely

**NEVER blindly read entire large files.** Follow this hierarchy:

1. **Precise Search (Highest Priority)**
   - Know filename, function name, or keyword? → Use orbit_search
   - Fast, precise, minimal token usage

2. **Semantic Search (When Uncertain)**
   - Not sure of exact path? → Use orbit_search tool
   - Good for fuzzy queries like "authentication logic" or "config management"
   - Returns relevant file locations

3. **Targeted File Reading (After Search)**
   - **ALWAYS specify line ranges** when you know the location
   - read_file supports offset/limit parameters
   - Default limit is 2000 lines - for large files, read in chunks
   - Example: `read_file(path="src/main.rs", offset=100, limit=50)` reads lines 100-150

4. **Explore Project Structure**
   - Use list_files to understand directory layout
   - Check README.md, package.json, Cargo.toml first for overview

### Anti-Patterns to Avoid:
❌ Reading entire 3000+ line files without specifying range
❌ Reading multiple large files at once
❌ Reading files before understanding what to look for
❌ Not using search tools when you have specific keywords

### Correct Patterns:
✅ Search for function → Find exact line number → read_file with offset/limit
✅ orbit_search for "auth" → Get file list → Read only relevant sections
✅ Check file size first → If large, read in chunks"#,
        )
    }

    async fn render(
        &self,
        _context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::Internal("missing work methodology template".to_string()))?;

        let result = TemplateEngine::new()
            .resolve(template, &HashMap::new())
            .map_err(|e| {
                AgentError::TemplateRender(format!(
                    "failed to render work methodology template: {}",
                    e
                ))
            })?;
        Ok(Some(result))
    }
}

struct CustomInstructionsComponent;

#[async_trait]
impl ComponentDefinition for CustomInstructionsComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::CustomInstructions
    }

    fn name(&self) -> &str {
        "Custom Instructions"
    }

    fn description(&self) -> &str {
        "User-provided custom instructions"
    }

    fn required(&self) -> bool {
        false
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[]
    }

    fn default_template(&self) -> Option<&str> {
        Some("ADDITIONAL INSTRUCTIONS\n\n{instructions}")
    }

    async fn render(
        &self,
        context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let instructions = match &context.ext_sys_prompt {
            Some(prompt) if !prompt.trim().is_empty() => prompt.trim(),
            _ => return Ok(None),
        };

        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| {
                AgentError::Internal("missing custom instructions template".to_string())
            })?;

        let mut template_context = HashMap::new();
        template_context.insert("instructions".to_string(), json!(instructions));

        let result = TemplateEngine::new()
            .resolve(template, &template_context)
            .map_err(|e| {
                AgentError::TemplateRender(format!(
                    "failed to render custom instructions template: {}",
                    e
                ))
            })?;

        Ok(Some(result))
    }
}
