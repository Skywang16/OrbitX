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
        "Executes a given shell command with optional timeout, ensuring proper handling and security measures.

Before executing the command, please follow these steps:

1. Directory Verification:
   - If the command will create new directories or files, first use the list_files tool to verify the parent directory exists and is the correct location
   - For example, before running \"mkdir foo/bar\", first use list_files to check that \"foo\" exists and is the intended parent directory

2. Command Execution:
   - Always quote file paths that contain spaces with double quotes (e.g., cd \"path with spaces/file.txt\")
   - Examples of proper quoting:
     - cd \"/Users/name/My Documents\" (correct)
     - cd /Users/name/My Documents (incorrect - will fail)
     - python \"/path/with spaces/script.py\" (correct)
     - python /path/with spaces/script.py (incorrect - will fail)
   - After ensuring proper quoting, execute the command.
   - Capture the output of the command.

Usage notes:
  - The command argument is required.
  - Commands will timeout after 120000ms (2 minutes).
  - VERY IMPORTANT: You MUST avoid using search commands like `find` and `grep`. Instead use orbit_search to search. You MUST avoid read tools like `cat`, `head`, `tail`, and `ls`, and use read_file and list_files instead.
  - If the output exceeds a certain limit, output will be truncated before being returned to you.
  - Dangerous commands like 'rm -rf /', 'shutdown', 'reboot' are automatically blocked for safety.
  - You MUST ensure that commands you run do not hang or require user input."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute. Examples: 'git status', 'npm test', 'cargo build'. IMPORTANT: Quote paths with spaces using double quotes. Avoid using cat/grep/find - use dedicated tools instead."
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
                })),
            }),
        }
    }
}
