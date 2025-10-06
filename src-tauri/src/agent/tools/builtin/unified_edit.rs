use std::path::PathBuf;

use async_trait::async_trait;
use diffy::{apply, Patch};
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::agent::context::FileOperationRecord;
use crate::agent::core::context::TaskContext;
use crate::agent::persistence::FileRecordSource;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolExecutorResult, ToolMetadata, ToolPermission, ToolPriority,
    ToolResult, ToolResultContent,
};

use super::file_utils::{ensure_absolute, is_probably_binary};

#[derive(Debug, Deserialize)]
struct UnifiedEditArgs {
    path: String,
    #[serde(flatten)]
    mode: EditMode,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
enum EditMode {
    Replace { old_text: String, new_text: String },
    Insert { after_line: u32, content: String },
    Diff { diff_content: String },
}

pub struct UnifiedEditTool;

impl UnifiedEditTool {
    pub fn new() -> Self {
        Self
    }

    async fn load_existing_text(path: &PathBuf) -> Result<String, ToolResult> {
        match fs::metadata(path).await {
            Ok(meta) => {
                if meta.is_dir() {
                    return Err(error_result(format!("路径 {} 是目录", path.display())));
                }
            }
            Err(_) => {
                return Err(error_result(format!("文件不存在: {}", path.display())));
            }
        }

        if is_probably_binary(path) {
            return Err(error_result(format!(
                "文件 {} 可能为二进制",
                path.display()
            )));
        }

        match fs::read_to_string(path).await {
            Ok(content) => Ok(content),
            Err(err) => Err(error_result(format!(
                "读取文件 {} 失败: {}",
                path.display(),
                err
            ))),
        }
    }

    async fn ensure_parent(path: &PathBuf) -> Result<(), ToolResult> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(error_result(format!("父目录不存在: {}", parent.display())));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl RunnableTool for UnifiedEditTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file using replace, insert, or diff modes."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "mode": { "type": "string", "enum": ["replace", "insert", "diff"] },
                "old_text": { "type": "string" },
                "new_text": { "type": "string" },
                "after_line": { "type": "integer", "minimum": 0 },
                "content": { "type": "string" },
                "diff_content": { "type": "string" }
            },
            "required": ["path", "mode"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileWrite, ToolPriority::Standard)
            .with_tags(vec!["filesystem".into(), "edit".into()])
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: UnifiedEditArgs = serde_json::from_value(args)?;
        let path = match ensure_absolute(&args.path, &context.cwd) {
            Ok(resolved) => resolved,
            Err(err) => return Ok(error_result(err.to_string())),
        };

        let result = match args.mode {
            EditMode::Replace { old_text, new_text } => {
                let original = match Self::load_existing_text(&path).await {
                    Ok(text) => text,
                    Err(err) => return Ok(err),
                };

                if !original.contains(&old_text) {
                    return Ok(error_result("未找到需要替换的文本"));
                }

                let updated = original.replace(&old_text, &new_text);
                if let Err(err) = fs::write(&path, &updated).await {
                    return Ok(error_result(format!(
                        "写入文件 {} 失败: {}",
                        path.display(),
                        err
                    )));
                }

                success_result(
                    format!("edit_file applied\nmode=replace\nfile={}", path.display()),
                    json!({
                        "path": path.display().to_string(),
                        "mode": "replace"
                    }),
                )
            }
            EditMode::Insert {
                after_line,
                content,
            } => {
                if let Err(err) = Self::ensure_parent(&path).await {
                    return Ok(err);
                }

                if is_probably_binary(&path) {
                    return Ok(error_result(format!(
                        "文件 {} 可能为二进制",
                        path.display()
                    )));
                }

                let (mut lines, trailing_newline) = match fs::metadata(&path).await {
                    Ok(meta) => {
                        if meta.is_dir() {
                            return Ok(error_result(format!("路径 {} 是目录", path.display())));
                        }
                        if is_probably_binary(&path) {
                            return Ok(error_result(format!(
                                "文件 {} 可能为二进制",
                                path.display()
                            )));
                        }
                        match fs::read_to_string(&path).await {
                            Ok(existing) => (
                                existing
                                    .lines()
                                    .map(|s| s.to_string())
                                    .collect::<Vec<String>>(),
                                existing.ends_with('\n'),
                            ),
                            Err(_) => (Vec::new(), false),
                        }
                    }
                    Err(_) => (Vec::new(), false),
                };

                let insert_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                let position = after_line.min(lines.len() as u32) as usize;
                lines.splice(position..position, insert_lines.into_iter());
                let mut updated = lines.join("\n");
                if trailing_newline || content.ends_with('\n') {
                    if !updated.ends_with('\n') {
                        updated.push('\n');
                    }
                }

                if let Err(err) = fs::write(&path, &updated).await {
                    return Ok(error_result(format!(
                        "写入文件 {} 失败: {}",
                        path.display(),
                        err
                    )));
                }

                success_result(
                    format!(
                        "edit_file applied\nmode=insert\nfile={}\nline={}",
                        path.display(),
                        after_line
                    ),
                    json!({
                        "path": path.display().to_string(),
                        "mode": "insert",
                        "line": after_line
                    }),
                )
            }
            EditMode::Diff { diff_content } => {
                let original = match Self::load_existing_text(&path).await {
                    Ok(text) => text,
                    Err(err) => return Ok(err),
                };

                let patch = match Patch::from_str(&diff_content) {
                    Ok(patch) => patch,
                    Err(err) => {
                        return Ok(error_result(format!("解析补丁失败: {}", err)));
                    }
                };

                let updated = match apply(&original, &patch) {
                    Ok(result) => result,
                    Err(err) => {
                        return Ok(error_result(format!("应用补丁失败: {}", err)));
                    }
                };

                if let Err(err) = fs::write(&path, &updated).await {
                    return Ok(error_result(format!(
                        "写入文件 {} 失败: {}",
                        path.display(),
                        err
                    )));
                }

                success_result(
                    format!("edit_file applied\nmode=diff\nfile={}", path.display()),
                    json!({
                        "path": path.display().to_string(),
                        "mode": "diff"
                    }),
                )
            }
        };

        context
            .file_tracker()
            .track_file_operation(FileOperationRecord::new(
                path.as_path(),
                FileRecordSource::AgentEdited,
            ))
            .await?;

        Ok(result)
    }
}

fn success_result(text: String, ext: serde_json::Value) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Text { text }],
        is_error: false,
        execution_time_ms: None,
        ext_info: Some(ext),
    }
}

fn error_result(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error {
            message: message.into(),
            details: None,
        }],
        is_error: true,
        execution_time_ms: None,
        ext_info: None,
    }
}
