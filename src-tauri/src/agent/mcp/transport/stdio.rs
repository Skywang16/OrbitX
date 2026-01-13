use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex};

use crate::agent::mcp::error::{McpError, McpResult};
use crate::agent::mcp::protocol::jsonrpc::{JsonRpcId, JsonRpcRequest, JsonRpcResponse};
use crate::agent::mcp::transport::McpTransport;

pub struct StdioTransport {
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<tokio::process::ChildStdin>>,
    pending: Arc<DashMap<i64, oneshot::Sender<McpResult<JsonRpcResponse>>>>,
    next_id: AtomicI64,
    connected: Arc<AtomicBool>,
}

impl StdioTransport {
    pub async fn spawn(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
        cwd: Option<&Path>,
    ) -> McpResult<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(cwd) = cwd {
            cmd.current_dir(cwd);
        }

        for (k, v) in env {
            cmd.env(k, v);
        }

        let mut child = cmd.spawn().map_err(|source| McpError::SpawnFailed {
            command: command.to_string(),
            source,
        })?;

        let stdin = child.stdin.take();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Process("Failed to capture stdout".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| McpError::Process("Failed to capture stderr".into()))?;

        let transport = Self {
            child: Mutex::new(Some(child)),
            stdin: Mutex::new(stdin),
            pending: Arc::new(DashMap::new()),
            next_id: AtomicI64::new(1),
            connected: Arc::new(AtomicBool::new(true)),
        };

        transport.start_stdout_loop(stdout);
        transport.start_stderr_loop(stderr);

        Ok(transport)
    }

    fn start_stdout_loop(&self, stdout: tokio::process::ChildStdout) {
        let pending = Arc::clone(&self.pending);
        let connected = Arc::clone(&self.connected);

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
                let Ok(val) = parsed else {
                    continue;
                };

                let response: Result<JsonRpcResponse, _> = serde_json::from_value(val.clone());
                match response {
                    Ok(resp) => {
                        if let JsonRpcId::Number(id) = &resp.id {
                            if let Some((_, tx)) = pending.remove(id) {
                                let _ = tx.send(Ok(resp));
                            }
                        }
                    }
                    Err(_) => {
                        // notifications or unknown messages are ignored for now
                    }
                }
            }

            connected.store(false, Ordering::SeqCst);

            // Fail all pending
            for entry in pending.iter() {
                let id = *entry.key();
                if let Some((_, tx)) = pending.remove(&id) {
                    let _ = tx.send(Err(McpError::Closed));
                }
            }
        });
    }

    fn start_stderr_loop(&self, stderr: tokio::process::ChildStderr) {
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                tracing::warn!(target: "mcp", "mcp stderr: {}", line);
            }
        });
    }

    fn alloc_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    async fn write_line(&self, line: &str) -> McpResult<()> {
        let mut guard = self.stdin.lock().await;
        let Some(stdin) = guard.as_mut() else {
            return Err(McpError::NotConnected);
        };

        stdin.write_all(line.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        Ok(())
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn request(&self, mut request: JsonRpcRequest) -> McpResult<JsonRpcResponse> {
        if !self.is_connected() {
            return Err(McpError::NotConnected);
        }

        let id = self.alloc_id();
        request.id = Some(JsonRpcId::Number(id));

        let (tx, rx) = oneshot::channel();
        self.pending.insert(id, tx);

        let json = serde_json::to_string(&request)?;
        self.write_line(&json).await?;

        let res = tokio::time::timeout(std::time::Duration::from_secs(60), rx)
            .await
            .map_err(|_| McpError::Timeout)?;

        match res {
            Ok(inner) => inner,
            Err(_) => Err(McpError::Closed),
        }
    }

    async fn notify(&self, request: JsonRpcRequest) -> McpResult<()> {
        if !self.is_connected() {
            return Err(McpError::NotConnected);
        }
        let json = serde_json::to_string(&request)?;
        self.write_line(&json).await?;
        Ok(())
    }

    async fn close(&self) -> McpResult<()> {
        self.connected.store(false, Ordering::SeqCst);

        {
            let mut stdin = self.stdin.lock().await;
            stdin.take();
        }

        let mut child_guard = self.child.lock().await;
        if let Some(mut child) = child_guard.take() {
            let _ = child.kill().await;
        }

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
}

pub fn expand_workspace(input: &str, workspace_root: &Path) -> String {
    let ws = workspace_root.to_string_lossy();
    input
        .replace("${workspaceFolder}", &ws)
        .replace("${workspace}", &ws)
}

pub fn expand_env_map(
    env: &HashMap<String, String>,
    workspace_root: &Path,
) -> HashMap<String, String> {
    env.iter()
        .map(|(k, v)| (k.clone(), expand_workspace(v, workspace_root)))
        .collect()
}

pub fn expand_args(args: &[String], workspace_root: &Path) -> Vec<String> {
    args.iter()
        .map(|a| expand_workspace(a, workspace_root))
        .collect()
}

pub fn ensure_abs_workspace(workspace_root: &Path) -> McpResult<PathBuf> {
    if workspace_root.is_absolute() {
        Ok(workspace_root.to_path_buf())
    } else {
        Err(McpError::WorkspaceNotAbsolute(workspace_root.to_path_buf()))
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        self.connected.store(false, Ordering::SeqCst);

        if let Ok(mut child_guard) = self.child.try_lock() {
            if let Some(ref mut child) = *child_guard {
                let _ = child.start_kill();
            }
        }
    }
}
