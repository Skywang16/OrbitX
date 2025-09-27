/*!
 * Apply Diff Tool
 *
 * Apply multiple text hunks across multiple files with optional preview.
 * Limitations: no interactive approval yet (to be added via front-end roundtrip).
 */

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs;

use crate::agent::state::context::TaskContext;
use crate::agent::tools::{
    error::ToolExecutorResult, RunnableTool, ToolPermission, ToolResult, ToolResultContent,
};

#[derive(Debug, Deserialize, Clone)]
struct DiffHunk {
    #[serde(rename = "contextBefore", default)]
    context_before: Option<Vec<String>>,
    #[serde(default)]
    old: Option<Vec<String>>,
    #[serde(default)]
    new: Option<Vec<String>>,
    #[serde(rename = "contextAfter", default)]
    context_after: Option<Vec<String>>,
    #[serde(rename = "startLineHint", default)]
    start_line_hint: Option<usize>, // 1-based
}

#[derive(Debug, Deserialize, Clone)]
struct FileDiff {
    path: String,
    hunks: Vec<DiffHunk>,
}

#[derive(Debug, Deserialize)]
struct ApplyDiffArgs {
    files: Vec<FileDiff>,
    #[serde(rename = "previewOnly", default)]
    preview_only: Option<bool>,
}

#[derive(Debug, Serialize, Clone)]
struct HunkResult {
    index: usize,
    success: bool,
    reason: Option<String>,
    #[serde(rename = "appliedAtLine", skip_serializing_if = "Option::is_none")]
    applied_at_line: Option<usize>,
    #[serde(rename = "oldLineCount", skip_serializing_if = "Option::is_none")]
    old_line_count: Option<usize>,
    #[serde(rename = "newLineCount", skip_serializing_if = "Option::is_none")]
    new_line_count: Option<usize>,
}

#[derive(Debug, Serialize, Clone)]
struct PerFileResult {
    path: String,
    exists: bool,
    #[serde(rename = "isDirectory")]
    is_directory: bool,
    #[serde(rename = "isBinary")]
    is_binary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    original_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_content: Option<String>,
    hunks: Vec<HunkResult>,
    applied: bool,
}

pub struct ApplyDiffTool;
impl ApplyDiffTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for ApplyDiffTool {
    fn name(&self) -> &str {
        "apply_diff"
    }

    fn description(&self) -> &str {
        "Apply multiple hunks across multiple files with optional preview-only mode."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "files": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" },
                            "hunks": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "contextBefore": { "type": "array", "items": { "type": "string" } },
                                        "old": { "type": "array", "items": { "type": "string" } },
                                        "new": { "type": "array", "items": { "type": "string" } },
                                        "contextAfter": { "type": "array", "items": { "type": "string" } },
                                        "startLineHint": { "type": "number", "minimum": 1 }
                                    },
                                    "required": []
                                }
                            }
                        },
                        "required": ["path", "hunks"]
                    }
                },
                "previewOnly": { "type": "boolean", "default": false }
            },
            "required": ["files"]
        })
    }

    fn required_permissions(&self) -> Vec<ToolPermission> {
        vec![ToolPermission::FileSystem]
    }

    fn tags(&self) -> Vec<String> {
        vec!["file".to_string(), "diff".to_string(), "patch".to_string()]
    }

    async fn run(
        &self,
        _context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: ApplyDiffArgs = serde_json::from_value(args)?;
        let preview_only = args.preview_only.unwrap_or(false);

        let mut results: Vec<PerFileResult> = Vec::new();

        for (file_index, file) in args.files.iter().enumerate() {
            let path = file.path.trim().to_string();
            if path.is_empty() {
                return Ok(tool_error_result(format!(
                    "files[{}].path is empty",
                    file_index
                )));
            }

            let exists = fs::metadata(&path).await.ok().is_some();
            let mut is_directory = false;
            let mut is_binary = false;
            let mut original_content: Option<String> = None;
            let mut new_content: Option<String> = None;
            let mut hunks_result: Vec<HunkResult> = Vec::new();

            if exists {
                if let Ok(meta) = fs::metadata(&path).await {
                    is_directory = meta.is_dir();
                }
            }

            if !exists || is_directory {
                results.push(PerFileResult {
                    path: path.clone(),
                    exists,
                    is_directory,
                    is_binary,
                    original_content,
                    new_content,
                    hunks: Vec::new(),
                    applied: false,
                });
                continue;
            }

            let content = match fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(e) => {
                    results.push(PerFileResult {
                        path: path.clone(),
                        exists,
                        is_directory,
                        is_binary,
                        original_content,
                        new_content,
                        hunks: vec![HunkResult {
                            index: 0,
                            success: false,
                            reason: Some(format!("read error: {}", e)),
                            applied_at_line: None,
                            old_line_count: None,
                            new_line_count: None,
                        }],
                        applied: false,
                    });
                    continue;
                }
            };

            is_binary = false;
            let mut working_lines: Vec<String> =
                content.split('\n').map(|s| s.to_string()).collect();

            for (i, h) in file.hunks.iter().enumerate() {
                let old_lines: Vec<String> = h.old.clone().unwrap_or_default();
                let new_lines: Vec<String> = h.new.clone().unwrap_or_default();
                let context_before: Vec<String> = h.context_before.clone().unwrap_or_default();
                let context_after: Vec<String> = h.context_after.clone().unwrap_or_default();
                let start_hint = h.start_line_hint;

                match find_hunk_index(
                    &working_lines,
                    &old_lines,
                    &context_before,
                    &context_after,
                    start_hint,
                ) {
                    Some(idx) => {
                        let applied_at_line = idx + 1; // 1-based
                        let old_count = old_lines.len();
                        if old_count == 0 {
                            working_lines.splice(idx..idx, new_lines.clone());
                        } else {
                            let end = idx + old_count;
                            working_lines.splice(idx..end, new_lines.clone());
                        }
                        hunks_result.push(HunkResult {
                            index: i,
                            success: true,
                            reason: None,
                            applied_at_line: Some(applied_at_line),
                            old_line_count: Some(old_lines.len()),
                            new_line_count: Some(new_lines.len()),
                        });
                    }
                    None => {
                        hunks_result.push(HunkResult {
                            index: i,
                            success: false,
                            reason: Some("No matching block found".to_string()),
                            applied_at_line: None,
                            old_line_count: Some(old_lines.len()),
                            new_line_count: Some(new_lines.len()),
                        });
                    }
                }
            }

            let applied_any = hunks_result.iter().any(|h| h.success);
            original_content = Some(content);
            if applied_any {
                new_content = Some(working_lines.join("\n"));
            }

            if applied_any && !preview_only {
                if let Some(ref nc) = new_content {
                    if let Err(e) = fs::write(&path, nc).await {
                        hunks_result.push(HunkResult {
                            index: file.hunks.len(),
                            success: false,
                            reason: Some(format!("write error: {}", e)),
                            applied_at_line: None,
                            old_line_count: None,
                            new_line_count: None,
                        });
                    }
                }
            }

            results.push(PerFileResult {
                path: path.clone(),
                exists,
                is_directory,
                is_binary,
                original_content,
                new_content,
                hunks: hunks_result,
                applied: applied_any,
            });
        }

        let mut total_files_changed = 0usize;
        let mut total_applied = 0usize;
        let mut any_failures = false;
        let mut lines = Vec::new();

        for r in &results {
            lines.push(format!("File: {}", r.path));
            lines.push(format!(
                "  exists: {}, dir: {}, binary: {}",
                r.exists, r.is_directory, r.is_binary
            ));
            if r.hunks.is_empty() {
                lines.push(String::from("  No hunks provided or not applicable"));
            }
            let mut file_applied = 0usize;
            for h in &r.hunks {
                if h.success {
                    total_applied += 1;
                    file_applied += 1;
                } else {
                    any_failures = true;
                }
                if h.success {
                    lines.push(format!(
                        "  Hunk #{}: applied at line {} (old {} -> new {})",
                        h.index,
                        h.applied_at_line.unwrap_or(0),
                        h.old_line_count.unwrap_or(0),
                        h.new_line_count.unwrap_or(0)
                    ));
                } else {
                    lines.push(format!(
                        "  Hunk #{}: FAILED - {}",
                        h.index,
                        h.reason.clone().unwrap_or_default()
                    ));
                }
            }
            if file_applied > 0 {
                total_files_changed += 1;
            }
            lines.push(String::new());
        }

        let header = format!(
            "Summary: {} hunk(s) applied across {} file(s).{}",
            total_applied,
            total_files_changed,
            if any_failures {
                " Some hunks failed or were skipped."
            } else {
                ""
            }
        );

        let preview_text = std::iter::once(header)
            .chain(std::iter::once(String::new()))
            .chain(lines.into_iter())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(ToolResult {
            content: vec![ToolResultContent::Text {
                text: if preview_only {
                    format!("apply_diff preview\n{}", preview_text)
                } else {
                    format!("apply_diff applied\n{}", preview_text)
                },
            }],
            is_error: false,
            execution_time_ms: None,
            metadata: Some(json!({
                "files": results,
                "totalApplied": total_applied,
                "totalFilesChanged": total_files_changed,
                "failures": any_failures,
                "previewOnly": preview_only,
            })),
        })
    }
}

fn find_hunk_index(
    working_lines: &Vec<String>,
    old_lines: &Vec<String>,
    context_before: &Vec<String>,
    context_after: &Vec<String>,
    start_line_hint: Option<usize>,
) -> Option<usize> {
    if old_lines.is_empty() {
        let idx = start_line_hint
            .and_then(|h| Some(h.saturating_sub(1).min(working_lines.len())))
            .unwrap_or(working_lines.len());
        return Some(idx);
    }

    let start_idx = start_line_hint
        .and_then(|h| Some(h.saturating_sub(1)))
        .unwrap_or(0);

    let search_ranges = if start_idx < working_lines.len() {
        vec![(start_idx, working_lines.len()), (0, start_idx)]
    } else {
        vec![(0, working_lines.len())]
    };

    for (begin, end) in search_ranges {
        if end < begin {
            continue;
        }
        let range_len = end.saturating_sub(begin);
        if range_len < old_lines.len() {
            continue;
        }

        for i in begin..=end.saturating_sub(old_lines.len()) {
            let mut match_old = true;
            for j in 0..old_lines.len() {
                if working_lines[i + j] != old_lines[j] {
                    match_old = false;
                    break;
                }
            }
            if !match_old {
                continue;
            }

            if !context_before.is_empty() {
                if i < context_before.len() {
                    continue;
                }
                let start = i - context_before.len();
                let mut ok = true;
                for k in 0..context_before.len() {
                    if working_lines[start + k] != context_before[k] {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    continue;
                }
            }

            if !context_after.is_empty() {
                let start = i + old_lines.len();
                let end_after = start + context_after.len();
                if end_after > working_lines.len() {
                    continue;
                }
                let mut ok = true;
                for k in 0..context_after.len() {
                    if working_lines[start + k] != context_after[k] {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    continue;
                }
            }

            return Some(i);
        }
    }

    None
}

fn tool_error_result(msg: String) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error {
            message: msg,
            details: None,
        }],
        is_error: true,
        execution_time_ms: None,
        metadata: None,
    }
}
