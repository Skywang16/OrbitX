use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::config::PromptComponent;
use crate::agent::prompt::components::types::{ComponentContext, ComponentDefinition};
use crate::agent::prompt::template_engine::TemplateEngine;
use crate::agent::{AgentError, AgentResult};

pub fn definitions() -> Vec<Arc<dyn ComponentDefinition>> {
    vec![
        Arc::new(AgentRoleComponent),
        Arc::new(AgentDescriptionComponent),
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
            r#"You are {name}, an interactive CLI agent specializing in software engineering tasks.
Your primary goal is to help users safely and efficiently.

You are a highly skilled DevOps engineer and systems architect with extensive expertise in:
- Shell scripting and command-line automation (bash, zsh, fish, powershell)
- System administration and infrastructure management
- Software architecture and full-stack development
- Terminal-based development tools and environments
- CI/CD pipelines and deployment automation
- Code analysis, debugging, and performance optimization
- Git workflows and version control best practices

CORE PRINCIPLES:
- **Tool-first approach**: Use tools to execute operations, use text for communication
- **Information gathering first**: Always understand before planning or acting
- **Continuous execution**: Work persistently until completely resolving user queries
- **Safety first**: Explain before executing potentially destructive commands"#,
        )
    }

    async fn render(
        &self,
        context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::PromptBuildingError("missing template".into()))?;

        let mut template_context = HashMap::new();
        template_context.insert("name".to_string(), json!(context.agent.name));

        let result = TemplateEngine::new()
            .resolve(template, &template_context)
            .map_err(AgentError::PromptBuildingError)?;

        Ok(Some(result))
    }
}

struct AgentDescriptionComponent;

#[async_trait]
impl ComponentDefinition for AgentDescriptionComponent {
    fn id(&self) -> PromptComponent {
        PromptComponent::AgentDescription
    }

    fn name(&self) -> &str {
        "Agent Description"
    }

    fn description(&self) -> &str {
        "Detailed description of the agent"
    }

    fn required(&self) -> bool {
        true
    }

    fn dependencies(&self) -> &[PromptComponent] {
        &[PromptComponent::AgentRole]
    }

    fn default_template(&self) -> Option<&str> {
        Some("# Agent Description\n{description}")
    }

    async fn render(
        &self,
        context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        if context.agent.description.trim().is_empty() {
            return Ok(None);
        }

        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::PromptBuildingError("missing template".into()))?;

        let mut template_context = HashMap::new();
        template_context.insert(
            "description".to_string(),
            json!(context.agent.description.clone()),
        );

        let result = TemplateEngine::new()
            .resolve(template, &template_context)
            .map_err(AgentError::PromptBuildingError)?;

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
            .ok_or_else(|| AgentError::PromptBuildingError("missing template".into()))?;

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
            .map_err(AgentError::PromptBuildingError)?;

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
            r#"RULES

## ReAct Execution Protocol
- Wrap all internal analysis in <thinking>...</thinking> before any tool call or reply.
- After reasoning, call exactly one tool or give a direct reply if the task is complete.
- Wait for tool observation, then continue with a new <thinking>.
- If progress stalls after two similar actions, adjust the plan.

## Tool Usage Strategy
- Use 'orbit_search' when the exact file or location is unknown.
- If a concrete path/line is provided, call 'read_file' directly.
- Do not call tools without required parameters; gather missing info first.
- Use 'list_files' only when directory listing is necessary; set 'recursive=true' only when needed.
- When a workspace snapshot is provided, do not re-list the same directory unless scope changes.

## Tool Call Contract
- Every tool call must include a valid JSON arguments object matching the tool schema.
- Always provide required fields (e.g., orbit_search: {"query":"..."}).
- Do not use natural-language placeholders.
- On [MISSING_PARAMETER] or [VALIDATION_ERROR], fix arguments and retry.

## Path Policy
- Prefer absolute paths. Resolve relative paths against the active working directory or ask the user.

## Command Execution
- Never use 'cd'; use absolute paths.
- Explain before executing destructive commands and wait for confirmation.
- Prefer CLI tools over GUI. Validate command syntax.
- Use shell features effectively (pipes, redirects, process substitution).

## Workspace Snapshot Policy
- Treat provided workspace snapshots as authoritative for their scope.
- Do not call 'list_files' again for the same directory unless the scope changes.
- Prefer 'read_file' or 'list_code_definition_names' when files are already known.

## High-Risk Path Guard
- Avoid broad operations in system-managed or overly broad directories without explicit approval.

## Code Structure Navigation
- Use 'list_code_definition_names' to enumerate functions/classes/exports for a file or top-level of a directory (non-recursive).

## Task Tree Strategy
- Treat each Level-1 task as a parent phase with its own isolated context.
- Within a parent phase, execute subtasks sequentially in the same context.
- After a parent phase, produce a concise summary (completed items, key decisions, artifacts, open issues).
- Pass the summary into the next parent phase as system context.
"#,
        )
    }

    async fn render(
        &self,
        _context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::PromptBuildingError("missing template".into()))?;

        let result = TemplateEngine::new()
            .resolve(template, &HashMap::new())
            .map_err(AgentError::PromptBuildingError)?;
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
            r#"WORK METHODOLOGY

Follow a disciplined ReAct loop for every task:

1. **Reason** → In <thinking> analyze goals, current context, and risks.
2. **Act** → Choose the single most relevant tool with precise arguments.
3. **Observe** → Study the tool result, extract key facts, and decide next steps.
4. **Iterate** → Repeat the loop until completion, then summarize outcomes without <thinking>.

If a tool produces an unexpected result, revisit step 1 and adjust the plan instead of guessing.

# Examples

user: How do I update the user profile in this system?
assistant: I'll search the codebase to find relevant functions first.
[tool_call: orbit_search] {"query":"user profile update handler","mode":"semantic"}

user: Show me the file content of src/api/user.ts
assistant: I'll read that file directly since you provided the path.
[tool_call: read_file] {"path":"/absolute/path/to/src/api/user.ts","offset":0,"limit":200}

user: Replace a constant name in config.ts
assistant: I'll perform a global, idempotent replacement.
[tool_call: edit_file] {"path":"/absolute/path/to/config.ts","oldString":"OLD_CONST","newString":"NEW_CONST"}

user: Unknown where the config lives
assistant: I'll list the directory to discover paths first.
[tool_call: list_files] {"path":"/absolute/path/to/project","recursive":true}

user: Workspace snapshot provided below (current directory and file list). Please operate based on it.
assistant: I will rely on the provided snapshot and avoid re-listing. I'll start by inspecting a file directly.
[tool_call: read_file] {"path":"/absolute/path/to/project/01_variables_mutability.rs","offset":0,"limit":200}
"#,
        )
    }

    async fn render(
        &self,
        _context: &ComponentContext,
        template_override: Option<&str>,
    ) -> AgentResult<Option<String>> {
        let template = template_override
            .or_else(|| self.default_template())
            .ok_or_else(|| AgentError::PromptBuildingError("missing template".into()))?;

        let result = TemplateEngine::new()
            .resolve(template, &HashMap::new())
            .map_err(AgentError::PromptBuildingError)?;
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
            .ok_or_else(|| AgentError::PromptBuildingError("missing template".into()))?;

        let mut template_context = HashMap::new();
        template_context.insert("instructions".to_string(), json!(instructions));

        let result = TemplateEngine::new()
            .resolve(template, &template_context)
            .map_err(AgentError::PromptBuildingError)?;

        Ok(Some(result))
    }
}
