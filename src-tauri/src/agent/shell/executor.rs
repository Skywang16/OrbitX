//! Agent Shell 执行器

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::debug;

use super::config::ShellExecutorConfig;
use super::error::ShellError;
use super::types::*;

/// Agent Shell 执行器
pub struct AgentShellExecutor {
    /// 配置
    config: ShellExecutorConfig,
    /// 运行中的命令
    commands: Arc<RwLock<HashMap<CommandId, RunningCommand>>>,
    /// 命令 ID 生成器
    next_command_id: AtomicU64,
}

impl AgentShellExecutor {
    /// 创建新的执行器
    pub fn new() -> Self {
        Self::with_config(ShellExecutorConfig::default())
    }

    /// 使用指定配置创建执行器
    pub fn with_config(config: ShellExecutorConfig) -> Self {
        Self {
            config,
            commands: Arc::new(RwLock::new(HashMap::new())),
            next_command_id: AtomicU64::new(1),
        }
    }

    /// 生成下一个命令 ID
    fn next_id(&self) -> CommandId {
        self.next_command_id.fetch_add(1, Ordering::Relaxed)
    }

    /// 验证命令
    fn validate_command(&self, command: &str) -> Result<(), ShellError> {
        if command.trim().is_empty() {
            return Err(ShellError::ValidationFailed(
                "Command cannot be empty".into(),
            ));
        }

        if command.len() > self.config.max_command_length {
            return Err(ShellError::ValidationFailed(format!(
                "Command too long (max {} bytes)",
                self.config.max_command_length
            )));
        }

        Ok(())
    }

    /// 同步执行命令（等待完成或超时）
    pub async fn execute(
        &self,
        command: &str,
        cwd: &str,
        timeout_duration: Option<Duration>,
    ) -> Result<ShellExecutionResult, ShellError> {
        self.validate_command(command)?;

        let timeout_duration = timeout_duration
            .unwrap_or(self.config.default_timeout)
            .min(self.config.max_timeout);

        let id = self.next_id();
        let mut running_cmd = RunningCommand::new(
            id,
            command.to_string(),
            cwd.to_string(),
            false,
            self.config.output_buffer_size,
        );

        running_cmd.status = CommandStatus::Running { pid: None };

        // 构建命令
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

        // 执行命令
        let result = timeout(timeout_duration, async {
            let mut child = cmd.spawn()?;

            // 获取 PID
            let pid = child.id();
            running_cmd.pid = pid;
            running_cmd.status = CommandStatus::Running { pid };

            // 读取输出
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            let mut output = String::new();

            if let Some(stdout) = stdout {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    output.push_str(&line);
                    output.push('\n');
                    running_cmd.output_buffer.write_str(&line);
                    running_cmd.output_buffer.write_str("\n");
                }
            }

            if let Some(stderr) = stderr {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    output.push_str(&line);
                    output.push('\n');
                    running_cmd.output_buffer.write_str(&line);
                    running_cmd.output_buffer.write_str("\n");
                }
            }

            let status = child.wait().await?;
            let exit_code = status.code().unwrap_or(-1);

            Ok::<(String, i32), std::io::Error>((output, exit_code))
        })
        .await;

        let duration_ms = running_cmd.elapsed_ms();

        match result {
            Ok(Ok((output, exit_code))) => {
                running_cmd.status = CommandStatus::Completed {
                    exit_code,
                    duration_ms,
                };

                Ok(ShellExecutionResult {
                    command_id: id,
                    status: running_cmd.status.clone(),
                    output,
                    exit_code: Some(exit_code),
                    duration_ms,
                    cwd: cwd.to_string(),
                    output_truncated: running_cmd.output_buffer.is_overflowed(),
                })
            }
            Ok(Err(e)) => {
                running_cmd.status = CommandStatus::Failed {
                    error: e.to_string(),
                };
                Err(ShellError::IoError(e))
            }
            Err(_) => {
                running_cmd.status = CommandStatus::TimedOut { duration_ms };
                Err(ShellError::Timeout(duration_ms))
            }
        }
    }

    /// 后台执行命令（立即返回）
    pub async fn execute_background(
        &self,
        command: &str,
        cwd: &str,
        timeout_duration: Option<Duration>,
    ) -> Result<CommandId, ShellError> {
        self.validate_command(command)?;

        // 检查后台命令数量限制
        {
            let commands = self.commands.read().await;
            let background_count = commands.values().filter(|c| c.is_background).count();
            if background_count >= self.config.max_background_commands {
                return Err(ShellError::TooManyBackgroundCommands(
                    self.config.max_background_commands,
                ));
            }
        }

        let id = self.next_id();
        let running_cmd = RunningCommand::new(
            id,
            command.to_string(),
            cwd.to_string(),
            true,
            self.config.output_buffer_size,
        );

        let abort_signal = running_cmd.abort_signal.clone();

        // 存储命令
        {
            let mut commands = self.commands.write().await;
            commands.insert(id, running_cmd);
        }

        // 启动后台任务
        let commands = self.commands.clone();
        let command = command.to_string();
        let cwd = cwd.to_string();
        let timeout_duration = timeout_duration
            .unwrap_or(self.config.default_timeout)
            .min(self.config.max_timeout);

        tokio::spawn(async move {
            Self::run_background_command(
                commands,
                id,
                &command,
                &cwd,
                timeout_duration,
                abort_signal,
            )
            .await;
        });

        Ok(id)
    }

    /// 后台命令执行逻辑
    async fn run_background_command(
        commands: Arc<RwLock<HashMap<CommandId, RunningCommand>>>,
        id: CommandId,
        command: &str,
        cwd: &str,
        timeout_duration: Duration,
        abort_signal: Arc<AtomicBool>,
    ) {
        // 构建命令
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

        // 更新状态为运行中
        {
            let mut cmds = commands.write().await;
            if let Some(running_cmd) = cmds.get_mut(&id) {
                running_cmd.status = CommandStatus::Running { pid: None };
            }
        }

        let result = timeout(timeout_duration, async {
            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => return Err(e),
            };

            // 更新 PID
            let pid = child.id();
            {
                let mut cmds = commands.write().await;
                if let Some(running_cmd) = cmds.get_mut(&id) {
                    running_cmd.pid = pid;
                    running_cmd.status = CommandStatus::Running { pid };
                }
            }

            // 读取输出
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            if let Some(stdout) = stdout {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if abort_signal.load(Ordering::Relaxed) {
                        let _ = child.kill().await;
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Interrupted,
                            "Aborted",
                        ));
                    }

                    let mut cmds = commands.write().await;
                    if let Some(running_cmd) = cmds.get_mut(&id) {
                        running_cmd.output_buffer.write_str(&line);
                        running_cmd.output_buffer.write_str("\n");
                    }
                }
            }

            if let Some(stderr) = stderr {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if abort_signal.load(Ordering::Relaxed) {
                        let _ = child.kill().await;
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Interrupted,
                            "Aborted",
                        ));
                    }

                    let mut cmds = commands.write().await;
                    if let Some(running_cmd) = cmds.get_mut(&id) {
                        running_cmd.output_buffer.write_str(&line);
                        running_cmd.output_buffer.write_str("\n");
                    }
                }
            }

            let status = child.wait().await?;
            Ok(status.code().unwrap_or(-1))
        })
        .await;

        // 更新最终状态
        let mut cmds = commands.write().await;
        if let Some(running_cmd) = cmds.get_mut(&id) {
            let duration_ms = running_cmd.elapsed_ms();

            match result {
                Ok(Ok(exit_code)) => {
                    running_cmd.status = CommandStatus::Completed {
                        exit_code,
                        duration_ms,
                    };
                }
                Ok(Err(e)) if e.kind() == std::io::ErrorKind::Interrupted => {
                    running_cmd.status = CommandStatus::Aborted;
                }
                Ok(Err(e)) => {
                    running_cmd.status = CommandStatus::Failed {
                        error: e.to_string(),
                    };
                }
                Err(_) => {
                    running_cmd.status = CommandStatus::TimedOut { duration_ms };
                }
            }

            debug!(
                "Background command {} finished: {:?}",
                id, running_cmd.status
            );
        }
    }

    /// 获取命令状态
    pub async fn get_command_status(&self, id: CommandId) -> Option<CommandStatus> {
        let commands = self.commands.read().await;
        commands.get(&id).map(|c| c.status.clone())
    }

    /// 获取命令输出
    pub async fn get_command_output(&self, id: CommandId) -> Option<String> {
        let commands = self.commands.read().await;
        commands.get(&id).map(|c| c.output_buffer.content_string())
    }

    /// 中止命令
    pub async fn abort_command(&self, id: CommandId) -> Result<(), ShellError> {
        let commands = self.commands.read().await;
        if let Some(cmd) = commands.get(&id) {
            cmd.abort_signal.store(true, Ordering::Relaxed);
            Ok(())
        } else {
            Err(ShellError::CommandNotFound(id))
        }
    }

    /// 获取所有运行中的命令
    pub async fn get_running_commands(&self) -> Vec<RunningCommandInfo> {
        let commands = self.commands.read().await;
        commands
            .values()
            .filter(|c| !c.status.is_terminal())
            .map(RunningCommandInfo::from)
            .collect()
    }

    /// 清理已完成的旧命令
    pub async fn cleanup_completed(&self) {
        let retention = self.config.completed_retention;
        let mut commands = self.commands.write().await;

        commands.retain(|_, cmd| {
            if cmd.status.is_terminal() {
                cmd.elapsed_ms() < retention.as_millis() as u64
            } else {
                true
            }
        });
    }
}

impl Default for AgentShellExecutor {
    fn default() -> Self {
        Self::new()
    }
}
