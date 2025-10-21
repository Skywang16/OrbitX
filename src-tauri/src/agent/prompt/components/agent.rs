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
        Some("You are an interactive CLI tool that helps users with software engineering tasks. Use the instructions and tools available to assist the user.")
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
            r#"# Tone and Style

You MUST answer concisely with fewer than 4 lines (not including tool use or code generation), unless user asks for detail.
IMPORTANT: Minimize output tokens while maintaining helpfulness. Only address the specific query, avoiding tangential information.
Do NOT add unnecessary preamble/postamble. Answer directly without elaboration:

<example>
user: 2 + 2
assistant: 4
</example>

<example>
user: Is 11 prime?
assistant: Yes
</example>

<example>
user: Which file contains foo?
assistant: src/foo.c
</example>

When you run non-trivial bash commands, explain what it does and why.
Your output displays on CLI. Use Github-flavored markdown, rendered in monospace font.
If you cannot help, keep response to 1-2 sentences. Offer alternatives if possible.
Only use emojis if explicitly requested.

# Proactiveness

Be proactive only when user asks you to do something. Balance:
- ✅ Do the right thing when asked, including follow-up actions
- ❌ Don't surprise user with actions without asking

Example: If user asks "how to implement XX?", answer the question first, don't immediately start implementing."#,
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
            r#"# Following Conventions

When making changes, first understand file's code conventions. Mimic style, use existing libraries, follow patterns.
- NEVER assume libraries are available. Check package.json/Cargo.toml before using any library.
- When creating components: Look at existing ones, consider framework, naming, typing conventions.
- When editing: Look at imports, understand framework, make changes idiomatically.
- Follow security best practices. Never expose/log secrets. Never commit secrets.

# Code Style

IMPORTANT: DO NOT ADD ***ANY*** COMMENTS unless asked.

# Doing Tasks

For software engineering tasks (bugs, features, refactoring, etc.):
- Use search tools extensively (parallel + sequential) to understand codebase
- Implement using all available tools
- Verify with tests. NEVER assume test framework. Check README or search codebase.
- VERY IMPORTANT: Run lint/typecheck commands when done. If not found, ask user and suggest writing to CLAUDE.md.
- NEVER commit unless user explicitly asks.

# Tool Usage Policy

When multiple independent pieces of information needed, batch tool calls for optimal performance.
When making multiple bash calls, send single message with multiple tool calls to run in parallel.
Example: To run "git status" and "git diff", send single message with two tool calls.

# Code References

When referencing code, use `file_path:line_number` pattern for easy navigation.

<example>
user: Where is error handling?
assistant: Client marked failed in src/services/process.ts:712 connectToServer function.
</example>

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
        Some("ADDITIONAL INSTRUCTIONS\n\n# 项目描述 (from CLAUDE.md)\n\n{instructions}")
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
