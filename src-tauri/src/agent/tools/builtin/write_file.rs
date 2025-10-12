use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::agent::context::FileOperationRecord;
use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::persistence::FileRecordSource;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPermission, ToolPriority, ToolResult,
    ToolResultContent,
};

use super::file_utils::{ensure_absolute, is_probably_binary};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteFileArgs {
    path: String,
    content: String,
}

pub struct WriteFileTool;

impl WriteFileTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Create or overwrite a file with the provided content."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file. Must be a complete path, for example: \"/Users/user/project/src/main.ts\""
                },
                "content": { "type": "string" }
            },
            "required": ["path", "content"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileWrite, ToolPriority::Standard)
            .with_tags(vec!["filesystem".into(), "write".into()])
            .with_summary_key_arg("path")
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: WriteFileArgs = serde_json::from_value(args)?;
        let path = match ensure_absolute(&args.path, &context.cwd) {
            Ok(resolved) => resolved,
            Err(err) => return Ok(error_result(err.to_string())),
        };

        if is_probably_binary(&path) {
            return Ok(error_result(format!(
                "文件 {} 可能为二进制",
                path.display()
            )));
        }

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Ok(error_result(format!("父目录不存在: {}", parent.display())));
            }
        }

        if let Ok(meta) = fs::metadata(&path).await {
            if meta.is_dir() {
                return Ok(error_result(format!("路径 {} 是目录", path.display())));
            }
        }

        if let Err(err) = fs::write(&path, args.content).await {
            return Ok(error_result(format!(
                "写入文件 {} 失败: {}",
                path.display(),
                err
            )));
        }

        context
            .file_tracker()
            .track_file_operation(FileOperationRecord::new(
                path.as_path(),
                FileRecordSource::AgentEdited,
            ))
            .await?;

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(format!(
                "write_file applied\nfile={}",
                path.display()
            ))],
            is_error: false,
            execution_time_ms: None,
            ext_info: Some(json!({
                "path": path.display().to_string()
            })),
        })
    }
}

fn error_result(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        is_error: true,
        execution_time_ms: None,
        ext_info: None,
    }
}
