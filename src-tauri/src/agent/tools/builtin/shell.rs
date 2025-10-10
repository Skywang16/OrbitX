use std::process::Stdio;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPermission, ToolPriority, ToolResult,
    ToolResultContent,
};

const COMMAND_TIMEOUT_MS: u64 = 120_000;
const MAX_COMMAND_LENGTH: usize = 1_000;
const DANGEROUS_COMMANDS: &[&str] = &[
    "rm -rf /",
    "sudo rm -rf",
    "format",
    "fdisk",
    "mkfs",
    "dd if=/dev/",
    "shutdown",
    "reboot",
    "halt",
    "poweroff",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShellArgs {
    command: String,
    #[serde(rename = "paneId")]
    pane_id: Option<i64>,
}

pub struct ShellTool;

impl ShellTool {
    pub fn new() -> Self {
        Self
    }

    fn validate_command(command: &str) -> Result<(), String> {
        if command.trim().is_empty() {
            return Err("Command cannot be empty".to_string());
        }

        if command.len() > MAX_COMMAND_LENGTH {
            return Err("Command too long, please break into shorter commands".to_string());
        }

        let lower = command.to_lowercase();
        if DANGEROUS_COMMANDS
            .iter()
            .any(|danger| lower.contains(&danger.to_lowercase()))
        {
            return Err(format!(
                "Dangerous command detected and blocked: {}",
                command
            ));
        }

        Ok(())
    }

    async fn execute(command: &str, cwd: &str) -> Result<(String, String, i32), String> {
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(command);
            c
        } else {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
            let mut c = Command::new(shell);
            c.arg("-lc").arg(command);
            c
        };

        if !cwd.trim().is_empty() {
            cmd.current_dir(cwd);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = timeout(Duration::from_millis(COMMAND_TIMEOUT_MS), cmd.output())
            .await
            .map_err(|_| format!("Command timed out after {} ms", COMMAND_TIMEOUT_MS))
            .and_then(|result| result.map_err(|e| e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or_default();

        Ok((stdout, stderr, exit_code))
    }
}

#[async_trait]
impl RunnableTool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute shell commands in the current workspace with basic safety checks."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Command to execute, e.g. 'ls -la'"
                },
                "paneId": {
                    "type": "number",
                    "description": "Optional terminal pane identifier (unused in backend)"
                }
            },
            "required": ["command"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Execution, ToolPriority::Standard)
            .with_confirmation()
            .with_timeout(Duration::from_millis(COMMAND_TIMEOUT_MS))
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
        if let Err(message) = ShellTool::validate_command(&args.command) {
            return Ok(ToolResult {
                content: vec![ToolResultContent::Error(message)],
                is_error: true,
                execution_time_ms: None,
                ext_info: Some(json!({
                    "command": args.command,
                    "paneId": args.pane_id,
                })),
            });
        }

        match ShellTool::execute(&args.command, &context.cwd).await {
            Ok((stdout, stderr, exit_code)) => {
                // 构建输出字符串：如果有 stderr，显示它；否则显示 stdout
                let output = if !stderr.is_empty() {
                    format!("{}\n{}", stdout, stderr)
                } else {
                    stdout.clone()
                };

                Ok(ToolResult {
                    content: vec![ToolResultContent::Success(output)],
                    is_error: false,
                    execution_time_ms: None,
                    ext_info: Some(json!({
                        "command": args.command,
                        "paneId": args.pane_id,
                        "stdout": stdout,
                        "stderr": stderr,
                        "exitCode": exit_code,
                    })),
                })
            }
            Err(err) => Ok(ToolResult {
                content: vec![ToolResultContent::Error(format!(
                    "Command execution failed: {}",
                    err
                ))],
                is_error: true,
                execution_time_ms: None,
                ext_info: Some(json!({
                    "command": args.command,
                    "paneId": args.pane_id,
                })),
            }),
        }
    }
}
