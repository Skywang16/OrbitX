use std::path::PathBuf;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};
use crate::code_intel::tree_sitter_diagnostics::{diagnose_syntax, TreeSitterDiagnostic};
use crate::vector_db::core::Language;

use super::file_utils::{ensure_absolute, is_probably_binary};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyntaxDiagnosticsArgs {
    /// Absolute file paths to diagnose.
    paths: Vec<String>,
}

pub struct SyntaxDiagnosticsTool;

impl SyntaxDiagnosticsTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for SyntaxDiagnosticsTool {
    fn name(&self) -> &'static str {
        "syntax_diagnostics"
    }

    fn description(&self) -> &'static str {
        "Run tree-sitter syntax diagnostics on files and return error ranges (syntax only)."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "paths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Absolute file paths to check."
                }
            },
            "required": ["paths"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::CodeAnalysis, ToolPriority::Standard)
            .with_tags(vec!["diagnostics".into(), "tree-sitter".into()])
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: SyntaxDiagnosticsArgs = serde_json::from_value(args)?;

        let mut all_diags: Vec<TreeSitterDiagnostic> = Vec::new();
        let mut checked = Vec::new();
        let mut skipped = Vec::new();

        for raw in args.paths {
            let path = match ensure_absolute(&raw, &context.cwd) {
                Ok(resolved) => resolved,
                Err(err) => {
                    skipped.push(json!({ "path": raw, "reason": err.to_string() }));
                    continue;
                }
            };

            let Some(language) = Language::from_path(&path) else {
                skipped.push(json!({ "path": path.display().to_string(), "reason": "unknown language" }));
                continue;
            };

            let Ok(meta) = fs::metadata(&path).await else {
                skipped.push(json!({ "path": path.display().to_string(), "reason": "missing file" }));
                continue;
            };
            if meta.is_dir() {
                skipped.push(json!({ "path": path.display().to_string(), "reason": "is a directory" }));
                continue;
            }
            if is_probably_binary(&path) {
                skipped.push(json!({ "path": path.display().to_string(), "reason": "binary file" }));
                continue;
            }

            let content = match fs::read_to_string(&path).await {
                Ok(text) => text,
                Err(err) => {
                    skipped.push(json!({ "path": path.display().to_string(), "reason": format!("read failed: {}", err) }));
                    continue;
                }
            };

            match diagnose_syntax(&path, &content, language) {
                Ok(mut diags) => {
                    checked.push(path.display().to_string());
                    all_diags.append(&mut diags);
                }
                Err(err) => {
                    skipped.push(json!({ "path": path.display().to_string(), "reason": err.to_string() }));
                }
            }
        }

        all_diags.sort_by(|a, b| {
            (a.file.as_str(), a.range.start.line, a.range.start.column)
                .cmp(&(b.file.as_str(), b.range.start.line, b.range.start.column))
        });

        let summary = format_diagnostics_summary(&all_diags);
        let status = if all_diags.is_empty() {
            ToolResultStatus::Success
        } else {
            ToolResultStatus::Success
        };

        Ok(ToolResult {
            content: vec![ToolResultContent::Success(summary)],
            status,
            ext_info: Some(json!({
                "checked": checked,
                "skipped": skipped,
                "diagnostics": all_diags,
                "errorCount": all_diags.len(),
            })),
            execution_time_ms: None,
            cancel_reason: None,
        })
    }
}

fn format_diagnostics_summary(diags: &[TreeSitterDiagnostic]) -> String {
    if diags.is_empty() {
        return "syntax_diagnostics: OK (no syntax errors)".to_string();
    }

    let mut out = String::new();
    out.push_str("syntax_diagnostics: syntax errors found\n");
    out.push_str("<file_diagnostics>\n");

    for d in diags {
        out.push_str(&format!(
            "{}:{}:{} {}\n",
            PathBuf::from(&d.file)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&d.file),
            d.range.start.line,
            d.range.start.column,
            d.message
        ));
    }

    out.push_str("</file_diagnostics>");
    out
}
