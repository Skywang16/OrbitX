use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::shell::{AgentShellExecutor, CommandStatus, ShellError};
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPermission, ToolPriority, ToolResult,
    ToolResultContent,
};

/// 默认超时时间（毫秒）
const DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// 全局 Shell 执行器
static SHELL_EXECUTOR: OnceLock<AgentShellExecutor> = OnceLock::new();

fn get_executor() -> &'static AgentShellExecutor {
    SHELL_EXECUTOR.get_or_init(AgentShellExecutor::new)
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
        "Executes a shell command with support for background execution and custom timeout.

Usage:
- command: The shell command to execute (required)
- cwd: Working directory (optional, defaults to current workspace)
- background: Run in background without waiting (optional, default false)
- timeout_ms: Custom timeout in milliseconds (optional, default 120000ms)

Notes:
- For long-running commands (e.g., dev servers), use background=true
- Background commands return immediately with a command ID
- Use read_terminal to check output of background commands
- Dangerous commands like 'rm -rf /', 'shutdown' are blocked
- Quote paths with spaces using double quotes
- Avoid using cat/grep/find - use read_file/orbit_search instead"
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

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::SystemCommand]
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: ShellArgs = serde_json::from_value(args)?;
        let executor = get_executor();

        // 确定工作目录
        let cwd = args.cwd.as_deref().unwrap_or(&context.cwd);

        // 确定超时时间
        let timeout_duration = args
            .timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_millis(DEFAULT_TIMEOUT_MS));

        // 是否后台运行
        let is_background = args.background.unwrap_or(false);

        if is_background {
            // 后台执行
            match executor
                .execute_background(&args.command, cwd, Some(timeout_duration))
                .await
            {
                Ok(command_id) => {
                    // 等待一小段时间收集初始输出（最多 2 秒）
                    tokio::time::sleep(Duration::from_millis(2000)).await;

                    // 获取当前输出和状态
                    let output = executor
                        .get_command_output(command_id)
                        .await
                        .unwrap_or_default();
                    let status = executor
                        .get_command_status(command_id)
                        .await
                        .unwrap_or(CommandStatus::Running { pid: None });

                    let is_still_running = !status.is_terminal();
                    let status_str = if is_still_running {
                        "running"
                    } else {
                        "completed"
                    };

                    let message = if is_still_running {
                        format!(
                            "Command running in background (ID: {}). Initial output:\n{}",
                            command_id,
                            if output.is_empty() {
                                "(no output yet)".to_string()
                            } else {
                                output.clone()
                            }
                        )
                    } else {
                        output.clone()
                    };

                    Ok(ToolResult {
                        content: vec![ToolResultContent::Success(message)],
                        is_error: false,
                        execution_time_ms: Some(2000),
                        ext_info: Some(json!({
                            "command": args.command,
                            "cwd": cwd,
                            "commandId": command_id,
                            "isBackground": true,
                            "status": status_str,
                            "output": output,
                        })),
                    })
                }
                Err(e) => Ok(error_result(&args.command, cwd, &e)),
            }
        } else {
            // 同步执行
            match executor
                .execute(&args.command, cwd, Some(timeout_duration))
                .await
            {
                Ok(result) => {
                    let is_success = matches!(
                        result.status,
                        CommandStatus::Completed { exit_code, .. } if exit_code == 0
                    );

                    let exit_code = match &result.status {
                        CommandStatus::Completed { exit_code, .. } => Some(*exit_code),
                        _ => None,
                    };

                    Ok(ToolResult {
                        content: vec![if is_success {
                            ToolResultContent::Success(result.output.clone())
                        } else {
                            ToolResultContent::Error(result.output.clone())
                        }],
                        is_error: !is_success,
                        execution_time_ms: Some(result.duration_ms),
                        ext_info: Some(json!({
                            "command": args.command,
                            "cwd": cwd,
                            "commandId": result.command_id,
                            "exitCode": exit_code,
                            "durationMs": result.duration_ms,
                            "isBackground": false,
                            "outputTruncated": result.output_truncated,
                            "status": result.status,
                        })),
                    })
                }
                Err(e) => Ok(error_result(&args.command, cwd, &e)),
            }
        }
    }
}

fn error_result(command: &str, cwd: &str, error: &ShellError) -> ToolResult {
    let (message, status) = match error {
        ShellError::Timeout(ms) => (format!("Command timed out after {}ms", ms), "timeout"),
        ShellError::Aborted => ("Command was aborted".to_string(), "aborted"),
        ShellError::DangerousCommand(cmd) => {
            (format!("Dangerous command blocked: {}", cmd), "blocked")
        }
        ShellError::ValidationFailed(msg) => {
            (format!("Validation failed: {}", msg), "validation_error")
        }
        ShellError::TooManyBackgroundCommands(max) => (
            format!("Too many background commands (max: {})", max),
            "limit_exceeded",
        ),
        _ => (format!("Command failed: {}", error), "failed"),
    };

    ToolResult {
        content: vec![ToolResultContent::Error(message)],
        is_error: true,
        execution_time_ms: None,
        ext_info: Some(json!({
            "command": command,
            "cwd": cwd,
            "status": status,
            "error": error.to_string(),
        })),
    }
}
