use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs;

use crate::agent::state::context::TaskContext;
use crate::agent::tools::{
    error::ToolExecutorResult, RunnableTool, ToolPermission, ToolResult, ToolResultContent,
};

const DEFAULT_MAX_BYTES: usize = 64 * 1024; // 64KB safeguard

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadManyFilesArgs {
    paths: Vec<String>,
    #[serde(default = "default_max_bytes")]
    max_bytes: usize,
    #[serde(default)]
    encoding: Option<String>,
}

fn default_max_bytes() -> usize {
    DEFAULT_MAX_BYTES
}

#[derive(Debug, Serialize)]
struct FileReadEntry {
    path: String,
    exists: bool,
    is_binary: bool,
    truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub struct ReadManyFilesTool;

impl ReadManyFilesTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for ReadManyFilesTool {
    fn name(&self) -> &str {
        "read_many_files"
    }

    fn description(&self) -> &str {
        "Read multiple files at once with optional size limits."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "paths": {
                    "type": "array",
                    "description": "Absolute or task-relative file paths",
                    "items": { "type": "string" },
                    "minItems": 1
                },
                "maxBytes": {
                    "type": "integer",
                    "minimum": 1024,
                    "description": "Maximum bytes to read per file (defaults to 65536)"
                },
                "encoding": {
                    "type": "string",
                    "enum": ["utf8", "base64"],
                    "description": "Force output encoding; utf8 (default) attempts UTF-8 decode with base64 fallback"
                }
            },
            "required": ["paths"]
        })
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }

    fn tags(&self) -> Vec<String> {
        vec!["file".to_string(), "read".to_string()]
    }

    async fn run(
        &self,
        _context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: ReadManyFilesArgs = serde_json::from_value(args)?;
        let mut entries = Vec::new();

        for raw_path in args.paths {
            let path = PathBuf::from(raw_path.clone());
            let exists = fs::metadata(&path).await.is_ok();

            if !exists {
                entries.push(FileReadEntry {
                    path: raw_path,
                    exists: false,
                    is_binary: false,
                    truncated: false,
                    content: None,
                    base64: None,
                    error: Some("File not found".to_string()),
                });
                continue;
            }

            match fs::read(&path).await {
                Ok(bytes) => {
                    let truncated = bytes.len() > args.max_bytes;
                    let limited = if truncated {
                        bytes[..args.max_bytes].to_vec()
                    } else {
                        bytes
                    };

                    let force_base64 = matches!(args.encoding.as_deref(), Some("base64"));

                    if force_base64 {
                        entries.push(FileReadEntry {
                            path: raw_path,
                            exists: true,
                            is_binary: true,
                            truncated,
                            content: None,
                            base64: Some(base64::encode(&limited)),
                            error: None,
                        });
                        continue;
                    }

                    match String::from_utf8(limited.clone()) {
                        Ok(text) => {
                            entries.push(FileReadEntry {
                                path: raw_path,
                                exists: true,
                                is_binary: false,
                                truncated,
                                content: Some(text),
                                base64: if truncated {
                                    Some(base64::encode(&limited))
                                } else {
                                    None
                                },
                                error: None,
                            });
                        }
                        Err(_) => {
                            entries.push(FileReadEntry {
                                path: raw_path,
                                exists: true,
                                is_binary: true,
                                truncated,
                                content: None,
                                base64: Some(base64::encode(&limited)),
                                error: None,
                            });
                        }
                    }
                }
                Err(err) => {
                    entries.push(FileReadEntry {
                        path: raw_path,
                        exists: true,
                        is_binary: false,
                        truncated: false,
                        content: None,
                        base64: None,
                        error: Some(format!("Failed to read file: {}", err)),
                    });
                }
            }
        }

        let had_error = entries.iter().all(|entry| entry.error.is_some());

        Ok(ToolResult {
            content: vec![ToolResultContent::Json {
                data: json!({ "files": entries }),
            }],
            is_error: had_error,
            execution_time_ms: None,
            metadata: None,
        })
    }
}
