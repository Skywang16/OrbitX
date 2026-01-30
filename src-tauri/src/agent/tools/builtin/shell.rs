use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::terminal::{AgentTerminalManager, TerminalExecutionMode, TerminalStatus};
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};

/// 默认超时时间（毫秒）
const DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// Git 安全协议验证
fn validate_git_command(command: &str) -> Result<(), String> {
    let cmd_lower = command.to_lowercase();

    // 检查是否是 git 命令
    if !cmd_lower.trim_start().starts_with("git ") {
        return Ok(());
    }

    // 禁止的危险操作
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

    // 警告 amend 操作（但不完全禁止，因为有合理使用场景）
    if cmd_lower.contains("commit --amend") || cmd_lower.contains("commit -a") {
        // 这里可以添加更复杂的逻辑来检查是否满足 amend 的安全条件
        // 但为了简化，我们先允许但在描述中给出详细指导
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShellArgs {
    /// 要执行的命令
    command: String,
    /// 工作目录（可选）
    cwd: Option<String>,
    /// 是否后台运行（可选）
    background: Option<bool>,
    /// 超时时间毫秒（可选）
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
        r#"Executes a shell command inside an Agent terminal with support for background execution and custom timeout.

IMPORTANT: This tool is for terminal operations like git, npm, docker, etc. DO NOT use it for file operations (reading, writing, editing, searching, finding files) - use the specialized tools for this instead.

Usage:
- command: The shell command to execute (required)
- cwd: Working directory (optional, defaults to current workspace)
- background: Run in background without waiting (optional, default false)
- timeout_ms: Custom timeout in milliseconds (optional, default 120000ms)

Command Execution Guidelines:
- Always quote file paths that contain spaces with double quotes (e.g., rm "path with spaces/file.txt")
- Examples of proper quoting:
  - mkdir "/Users/name/My Documents" (correct)
  - mkdir /Users/name/My Documents (incorrect - will fail)
  - python "/path/with spaces/script.py" (correct)
  - python /path/with spaces/script.py (incorrect - will fail)
- For long-running commands (e.g., dev servers), use background=true
- Background commands return immediately with a terminal ID
- Use read_agent_terminal to check output of background commands

Tool Usage Policy:
- Avoid using shell with find, grep, cat, head, tail, sed, awk, or echo commands
- Instead, use specialized tools:
  - File search: Use list_files (NOT find or ls)
  - Content search: Use grep tool (NOT shell grep or rg)
  - Read files: Use read_file (NOT cat/head/tail)
  - Edit files: Use unified_edit (NOT sed/awk)
  - Write files: Use write_file (NOT echo >/cat <<EOF)
  - Communication: Output text directly (NOT echo/printf)

Git Safety Protocol:
- NEVER update the git config
- NEVER run destructive/irreversible git commands (like push --force, hard reset, etc) unless the user explicitly requests them
- NEVER skip hooks (--no-verify, --no-gpg-sign, etc) unless the user explicitly requests it
- NEVER run force push to main/master, warn the user if they request it
- Avoid git commit --amend. ONLY use --amend when ALL conditions are met:
  (1) User explicitly requested amend, OR commit SUCCEEDED but pre-commit hook auto-modified files that need including
  (2) HEAD commit was created by you in this conversation (verify: git log -1 --format='%an %ae')
  (3) Commit has NOT been pushed to remote (verify: git status shows "Your branch is ahead")
- CRITICAL: If commit FAILED or was REJECTED by hook, NEVER amend - fix the issue and create a NEW commit
- CRITICAL: If you already pushed to remote, NEVER amend unless user explicitly requests it (requires force push)
- NEVER commit changes unless the user explicitly asks you to. It is VERY IMPORTANT to only commit when explicitly asked

Git Commit Workflow (when user explicitly requests):
1. Run git status, git diff, and git log commands in parallel to understand current state
2. Analyze all staged changes and draft a commit message focusing on the "why" rather than the "what"
3. Add relevant untracked files to staging area, create commit, then run git status to verify
4. If commit fails due to pre-commit hook, fix the issue and create a NEW commit (never amend)

Important Notes:
- DO NOT push to remote repository unless user explicitly asks
- Never use git commands with -i flag (interactive mode not supported)
- If no changes to commit, do not create empty commit
- Quote paths with spaces using double quotes
- Avoid using cat/grep/find - use read_file/grep tool instead"#
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
                    "description": "Run command in background without waiting. Use for long-running commands like dev servers."
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
        context: &TaskContext,
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

        // Git 安全检查
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

        // 确定工作目录
        let cwd = args.cwd.as_deref().unwrap_or(&context.cwd);

        // 确定超时时间
        let timeout_duration = args
            .timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_millis(DEFAULT_TIMEOUT_MS));

        // 是否后台运行
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
                context.session_id,
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
                "Command running in background (terminalId: {}). Use read_agent_terminal to check output.",
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

        let status = match manager
            .wait_for_completion(&terminal.id, timeout_duration)
            .await
        {
            Ok(status) => status,
            Err(err) => return Ok(tool_error(err, &args.command, cwd)),
        };

        let output = manager
            .get_terminal_last_command_output(&terminal.id)
            .unwrap_or_default();
        let exit_code = match status {
            TerminalStatus::Completed { exit_code } => exit_code,
            _ => None,
        };
        let is_success = matches!(status, TerminalStatus::Completed { exit_code: Some(0) });

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
            execution_time_ms: None,
            ext_info: Some(json!({
                "command": args.command,
                "cwd": cwd,
                "terminalId": terminal.id,
                "paneId": terminal.pane_id,
                "exitCode": exit_code,
                "isBackground": false,
                "status": status,
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
