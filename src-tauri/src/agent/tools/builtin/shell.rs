use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::file_utils::lenient;
use crate::agent::common::TruncationPolicy;
use crate::agent::core::context::AgentRunContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::terminal::{AgentTerminalManager, TerminalExecutionMode, TerminalStatus};
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};

/// Default timeout (milliseconds)
const DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// Git safety protocol validation
fn validate_git_command(command: &str) -> Result<(), String> {
    let cmd_lower = command.to_lowercase();

    // Check if it's a git command
    if !cmd_lower.trim_start().starts_with("git ") {
        return Ok(());
    }

    // Prohibited dangerous operations
    if cmd_lower.contains("git config") {
        return Err("NEVER update git config - this violates Git Safety Protocol".to_string());
    }

    if cmd_lower.contains("push --force") || cmd_lower.contains("push -f") {
        return Err(
            "NEVER force push without explicit user request - this violates Git Safety Protocol"
                .to_string(),
        );
    }

    if cmd_lower.contains("--no-verify") || cmd_lower.contains("--no-gpg-sign") {
        return Err(
            "NEVER skip hooks without explicit user request - this violates Git Safety Protocol"
                .to_string(),
        );
    }

    if cmd_lower.contains("reset --hard") {
        return Err("NEVER run hard reset without explicit user request - this is destructive and violates Git Safety Protocol".to_string());
    }

    if ((cmd_lower.contains("push") && cmd_lower.contains("main"))
        || (cmd_lower.contains("push") && cmd_lower.contains("master")))
        && (cmd_lower.contains("--force") || cmd_lower.contains("-f"))
    {
        return Err(
            "NEVER force push to main/master - this violates Git Safety Protocol".to_string(),
        );
    }

    // Warn about amend operations (but don't completely prohibit, as there are legitimate use cases)
    if cmd_lower.contains("commit --amend") || cmd_lower.contains("commit -a") {
        // More complex logic can be added here to check if amend safety conditions are met
        // But for simplicity, we allow it for now and provide detailed guidance in the description
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShellArgs {
    command: String,
    cwd: Option<String>,
    background: Option<bool>,
    #[serde(default, deserialize_with = "lenient::deserialize_opt_u64")]
    timeout_ms: Option<u64>,
}

pub struct ShellTool;

impl Default for ShellTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        r#"Execute shell commands for terminal operations (git, npm, cargo, docker, process management).

Usage:
- Use cwd parameter to change directory (NOT cd && command)
- Commands run in blocking mode unless you set background=true explicitly
- Use background=true for long-running commands (dev servers, watchers, log tails, `cargo run`, `npm run dev`)
- Quote paths with spaces
- Run independent commands in parallel

DO NOT use for file operations - use specialized tools:
- read_file (not cat), edit_file (not sed), write_file (not echo >)
- list_files (not find/ls), grep (not shell grep)

Git safety: Never commit/push/amend without explicit user request."#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute. Examples: 'git status', 'npm test', 'cargo build'."
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory for the command. Defaults to current workspace."
                },
                "background": {
                    "type": "boolean",
                    "description": "Set true to run the command in the background. If omitted, the command runs in blocking mode."
                },
                "timeoutMs": {
                    "type": "integer",
                    "minimum": 1000,
                    "maximum": 600000,
                    "description": "Timeout in milliseconds (default: 120000, max: 600000)."
                }
            },
            "required": ["command"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Execution, ToolPriority::Standard)
            .with_confirmation()
            .with_timeout(Duration::from_millis(DEFAULT_TIMEOUT_MS))
            .with_tags(vec!["shell".into(), "command".into()])
            .with_summary_key_arg("command")
    }

    async fn run(
        &self,
        context: &AgentRunContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: ShellArgs = serde_json::from_value(args)?;
        let manager = match AgentTerminalManager::global() {
            Some(manager) => manager,
            None => {
                return Ok(tool_error(
                    "Agent terminal manager is not initialized.",
                    &args.command,
                    context.cwd.as_ref(),
                ));
            }
        };

        // Git safety check
        if let Err(validation_error) = validate_git_command(&args.command) {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Error(validation_error)],
                status: ToolResultStatus::Error,
                cancel_reason: None,
                execution_time_ms: None,
                ext_info: Some(json!({
                    "command": args.command,
                    "error": "git_safety_violation",
                })),
            });
        }

        // Determine working directory
        let cwd = args.cwd.as_deref().unwrap_or(&context.cwd);

        // Determine timeout duration
        let timeout_duration = args
            .timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_millis(DEFAULT_TIMEOUT_MS));

        let is_background = args.background.unwrap_or(false);
        let mode = if is_background {
            TerminalExecutionMode::Background
        } else {
            TerminalExecutionMode::Blocking
        };

        let terminal_cwd = if cwd.trim().is_empty() {
            None
        } else {
            Some(cwd.to_string())
        };

        let terminal = match manager
            .create_terminal(
                args.command.clone(),
                mode.clone(),
                context.thread_id,
                terminal_cwd,
                None,
            )
            .await
        {
            Ok(terminal) => terminal,
            Err(err) => return Ok(tool_error(err, &args.command, cwd)),
        };

        if is_background {
            let message = format!(
                "Command running in background (terminalId: {}). Use read_terminal with this terminalId to check output.",
                terminal.id
            );
            return Ok(ToolResult {
                content: vec![ToolResultContent::Success(message)],
                status: ToolResultStatus::Success,
                cancel_reason: None,
                execution_time_ms: None,
                ext_info: Some(json!({
                    "command": args.command,
                    "cwd": cwd,
                    "terminalId": terminal.id,
                    "paneId": terminal.pane_id,
                    "isBackground": true,
                    "status": "running",
                })),
            });
        }

        let exec_start = Instant::now();

        let status = match manager
            .wait_for_completion(&terminal.id, timeout_duration)
            .await
        {
            Ok(status) => status,
            Err(err) => return Ok(tool_error(err, &args.command, cwd)),
        };

        let duration_secs = exec_start.elapsed().as_secs_f32();
        let raw_output = match manager.get_terminal_last_command_output(&terminal.id) {
            Ok(output) => output,
            Err(err) => return Ok(tool_error(err, &args.command, cwd)),
        };
        let exit_code = match status {
            TerminalStatus::Completed { exit_code } => exit_code,
            _ => None,
        };
        let is_success = matches!(status, TerminalStatus::Completed { exit_code: Some(0) });

        let truncation = TruncationPolicy::shell_output();
        let truncated = crate::agent::common::truncate_middle(&raw_output, truncation);
        let output = truncated.text;

        Ok(ToolResult {
            content: vec![if is_success {
                ToolResultContent::Success(output.clone())
            } else {
                ToolResultContent::Error(output.clone())
            }],
            status: if is_success {
                ToolResultStatus::Success
            } else {
                ToolResultStatus::Error
            },
            cancel_reason: None,
            execution_time_ms: Some((duration_secs * 1000.0) as u64),
            ext_info: Some(json!({
                "command": args.command,
                "cwd": cwd,
                "terminalId": terminal.id,
                "paneId": terminal.pane_id,
                "exitCode": exit_code,
                "isBackground": false,
                "status": status,
                "truncated": truncated.was_truncated,
                "originalLines": truncated.info.as_ref().map(|i| i.lines),
            })),
        })
    }
}

fn tool_error(message: impl Into<String>, command: &str, cwd: &str) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        status: ToolResultStatus::Error,
        cancel_reason: None,
        execution_time_ms: None,
        ext_info: Some(json!({
            "command": command,
            "cwd": cwd,
            "status": "failed",
        })),
    }
}
