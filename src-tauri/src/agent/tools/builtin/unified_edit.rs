use std::path::PathBuf;

use async_trait::async_trait;
use diffy::{apply, Patch};
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

/// 默认模糊匹配阈值 (1.0 = 精确匹配, 0.9 = 允许10%差异)
const DEFAULT_FUZZY_THRESHOLD: f64 = 0.9;

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

/// 计算两个字符串的相似度 (0.0 - 1.0)
/// 参考 Roo-Code 的 getSimilarity 函数
fn get_similarity(original: &str, search: &str) -> f64 {
    if search.is_empty() {
        return 0.0;
    }

    // 规范化字符串：处理智能引号和特殊字符
    let normalized_original = normalize_string(original);
    let normalized_search = normalize_string(search);

    if normalized_original == normalized_search {
        return 1.0;
    }

    // 计算 Levenshtein 距离
    let dist = levenshtein_distance(&normalized_original, &normalized_search);
    let max_len = normalized_original.len().max(normalized_search.len());

    if max_len == 0 {
        return 1.0;
    }

    1.0 - (dist as f64 / max_len as f64)
}

/// 规范化字符串：处理智能引号等特殊字符
fn normalize_string(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            // 智能引号转普通引号
            '\u{201C}' | '\u{201D}' => '"',
            '\u{2018}' | '\u{2019}' => '\'',
            // 其他特殊空格转普通空格
            '\u{00A0}' | '\u{2003}' | '\u{2002}' => ' ',
            _ => c,
        })
        .collect()
}

/// 计算 Levenshtein 编辑距离
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row: Vec<usize> = vec![0; b_len + 1];

    for i in 1..=a_len {
        curr_row[0] = i;
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr_row[j] = (prev_row[j] + 1)
                .min(curr_row[j - 1] + 1)
                .min(prev_row[j - 1] + cost);
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}

/// 模糊搜索结果
struct FuzzySearchResult {
    best_score: f64,
    best_match_index: Option<usize>,
    best_match_content: String,
}

/// 中间向外模糊搜索
/// 参考 Roo-Code 的 fuzzySearch 函数
fn fuzzy_search(
    lines: &[&str],
    search_chunk: &str,
    start_index: usize,
    end_index: usize,
) -> FuzzySearchResult {
    let mut best_score = 0.0;
    let mut best_match_index = None;
    let mut best_match_content = String::new();

    let search_lines: Vec<&str> = search_chunk.lines().collect();
    let search_len = search_lines.len();

    if search_len == 0 || end_index <= start_index {
        return FuzzySearchResult {
            best_score,
            best_match_index,
            best_match_content,
        };
    }

    // 从中点开始向两边搜索
    let mid_point = (start_index + end_index) / 2;
    let mut left_index = mid_point as isize;
    let mut right_index = mid_point + 1;

    while left_index >= start_index as isize || right_index <= end_index.saturating_sub(search_len)
    {
        // 向左搜索
        if left_index >= start_index as isize {
            let idx = left_index as usize;
            if idx + search_len <= lines.len() {
                let original_chunk = lines[idx..idx + search_len].join("\n");
                let similarity = get_similarity(&original_chunk, search_chunk);
                if similarity > best_score {
                    best_score = similarity;
                    best_match_index = Some(idx);
                    best_match_content = original_chunk;
                }
            }
            left_index -= 1;
        }

        // 向右搜索
        if right_index <= end_index.saturating_sub(search_len) {
            if right_index + search_len <= lines.len() {
                let original_chunk = lines[right_index..right_index + search_len].join("\n");
                let similarity = get_similarity(&original_chunk, search_chunk);
                if similarity > best_score {
                    best_score = similarity;
                    best_match_index = Some(right_index);
                    best_match_content = original_chunk;
                }
            }
            right_index += 1;
        }
    }

    FuzzySearchResult {
        best_score,
        best_match_index,
        best_match_content,
    }
}

/// 获取行的前导空白字符
fn get_leading_whitespace(line: &str) -> &str {
    let trimmed = line.trim_start();
    &line[..line.len() - trimmed.len()]
}

/// 智能缩进替换
/// 参考 Roo-Code 的缩进保持逻辑
fn apply_replacement_with_indent(
    matched_lines: &[&str],
    search_lines: &[&str],
    replace_lines: &[&str],
) -> Vec<String> {
    if matched_lines.is_empty() || search_lines.is_empty() {
        return replace_lines.iter().map(|s| s.to_string()).collect();
    }

    // 获取原文件每行的精确缩进
    let original_indents: Vec<&str> = matched_lines
        .iter()
        .map(|line| get_leading_whitespace(line))
        .collect();

    // 获取搜索内容每行的缩进
    let search_indents: Vec<&str> = search_lines
        .iter()
        .map(|line| get_leading_whitespace(line))
        .collect();

    let matched_base_indent = original_indents.first().copied().unwrap_or("");
    let search_base_indent = search_indents.first().copied().unwrap_or("");
    let search_base_level = search_base_indent.len();

    // 应用替换时保持精确缩进
    replace_lines
        .iter()
        .map(|line| {
            let current_indent = get_leading_whitespace(line);
            let current_level = current_indent.len();
            let relative_level = current_level as isize - search_base_level as isize;

            let final_indent = if relative_level < 0 {
                // 相对缩进为负，减少缩进
                let new_len = (matched_base_indent.len() as isize + relative_level).max(0) as usize;
                &matched_base_indent[..new_len]
            } else {
                // 相对缩进为正或零，保持或增加缩进
                let extra = if current_level > search_base_level {
                    &current_indent[search_base_level..]
                } else {
                    ""
                };
                // 需要返回拼接后的字符串
                return format!("{}{}{}", matched_base_indent, extra, line.trim());
            };

            format!("{}{}", final_indent, line.trim())
        })
        .collect()
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
                    return Err(error_result(format!(
                        "Path {} is a directory",
                        path.display()
                    )));
                }
            }
            Err(_) => {
                return Err(error_result(format!(
                    "File does not exist: {}",
                    path.display()
                )));
            }
        }

        if is_probably_binary(path) {
            return Err(error_result(format!(
                "File {} may be binary",
                path.display()
            )));
        }

        match fs::read_to_string(path).await {
            Ok(content) => Ok(content),
            Err(err) => Err(error_result(format!(
                "Failed to read file {}: {}",
                path.display(),
                err
            ))),
        }
    }

    async fn ensure_parent(path: &PathBuf) -> Result<(), ToolResult> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(error_result(format!(
                    "Parent directory does not exist: {}",
                    parent.display()
                )));
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
        "Performs smart string replacements or insertions in files with fuzzy matching and intelligent indentation preservation.

Usage:
- The path parameter must be an absolute path (e.g., '/Users/user/project/src/main.ts')
- You MUST use the read_file tool at least once before editing. This tool will error if you attempt an edit without reading the file first.
- The tool uses fuzzy matching (90% similarity threshold) to find the target text, tolerating minor whitespace differences
- Indentation is automatically preserved: the tool detects the original file's indentation style and applies it to replacements
- ALWAYS prefer editing existing files in the codebase. NEVER write new files unless explicitly required.
- Only use emojis if the user explicitly requests it. Avoid adding emojis to files unless asked.
- For replace mode, include enough surrounding context to make the old_text unique"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute path to the file to modify. For example: \"/Users/user/project/src/main.ts\""
                },
                "mode": {
                    "type": "string",
                    "enum": ["replace", "insert", "diff"],
                    "description": "Edit mode: 'replace' for find-and-replace, 'insert' for inserting at a line number, 'diff' for applying unified diffs"
                },
                "old_text": {
                    "type": "string",
                    "description": "[replace mode only] The exact text to find and replace. Must match exactly including whitespace and indentation. Include enough surrounding context to make this unique in the file."
                },
                "new_text": {
                    "type": "string",
                    "description": "[replace mode only] The text to replace old_text with. Must be different from old_text."
                },
                "after_line": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "[insert mode only] 0-based line number after which to insert content. Use 0 to insert at the beginning of the file."
                },
                "content": {
                    "type": "string",
                    "description": "[insert mode only] The content to insert at the specified line."
                },
                "diff_content": {
                    "type": "string",
                    "description": "[diff mode only] A unified diff format patch to apply to the file."
                }
            },
            "required": ["path", "mode"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileWrite, ToolPriority::Standard)
            .with_tags(vec!["filesystem".into(), "edit".into()])
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

                // 验证搜索和替换内容不相同
                if old_text == new_text {
                    return Ok(error_result(
                        "Search and replace content are identical - no changes would be made",
                    ));
                }

                // 检测行尾格式
                let line_ending = if original.contains("\r\n") {
                    "\r\n"
                } else {
                    "\n"
                };
                let lines: Vec<&str> = original.split('\n').collect();
                let search_lines: Vec<&str> = old_text.lines().collect();
                let replace_lines: Vec<&str> = new_text.lines().collect();

                if search_lines.is_empty() {
                    return Ok(error_result("Search content cannot be empty"));
                }

                // 1. 首先尝试精确匹配
                if original.contains(&old_text) {
                    // 检查是否有多个匹配
                    let match_count = original.matches(&old_text).count();
                    if match_count > 1 {
                        return Ok(error_result(format!(
                            "Found {} matches. Please provide more context to make a unique match.",
                            match_count
                        )));
                    }

                    // 精确匹配成功，使用智能缩进替换
                    let match_start = original.find(&old_text).unwrap();
                    let before = &original[..match_start];
                    let after = &original[match_start + old_text.len()..];

                    // 找到匹配位置的行
                    let match_line_start = before.lines().count().saturating_sub(1);
                    let matched_lines: Vec<&str> = lines
                        .iter()
                        .skip(match_line_start)
                        .take(search_lines.len())
                        .copied()
                        .collect();

                    // 应用智能缩进替换
                    let indented_replace = apply_replacement_with_indent(
                        &matched_lines,
                        &search_lines,
                        &replace_lines,
                    );

                    let updated =
                        format!("{}{}{}", before, indented_replace.join(line_ending), after);

                    if let Err(err) = fs::write(&path, &updated).await {
                        return Ok(error_result(format!(
                            "Failed to write file {}: {}",
                            path.display(),
                            err
                        )));
                    }

                    return Ok(success_result(
                        format!(
                            "edit_file applied\nmode=replace\nfile={}\nmatch=exact",
                            path.display()
                        ),
                        json!({
                            "file": path.display().to_string(),
                            "mode": "replace",
                            "matchType": "exact",
                            "old": old_text,
                            "new": new_text
                        }),
                    ));
                }

                // 2. 精确匹配失败，尝试模糊匹配
                let search_chunk = old_text.clone();
                let fuzzy_result = fuzzy_search(&lines, &search_chunk, 0, lines.len());

                if fuzzy_result.best_score >= DEFAULT_FUZZY_THRESHOLD {
                    if let Some(match_index) = fuzzy_result.best_match_index {
                        // 获取匹配的行
                        let matched_lines: Vec<&str> = lines
                            .iter()
                            .skip(match_index)
                            .take(search_lines.len())
                            .copied()
                            .collect();

                        // 应用智能缩进替换
                        let indented_replace = apply_replacement_with_indent(
                            &matched_lines,
                            &search_lines,
                            &replace_lines,
                        );

                        // 构建最终内容
                        let before_match: Vec<&str> =
                            lines.iter().take(match_index).copied().collect();
                        let after_match: Vec<&str> = lines
                            .iter()
                            .skip(match_index + search_lines.len())
                            .copied()
                            .collect();

                        let mut result_lines: Vec<String> =
                            before_match.iter().map(|s| s.to_string()).collect();
                        result_lines.extend(indented_replace);
                        result_lines.extend(after_match.iter().map(|s| s.to_string()));

                        let updated = result_lines.join(line_ending);

                        if let Err(err) = fs::write(&path, &updated).await {
                            return Ok(error_result(format!(
                                "Failed to write file {}: {}",
                                path.display(),
                                err
                            )));
                        }

                        return Ok(success_result(
                            format!(
                                "edit_file applied\nmode=replace\nfile={}\nmatch=fuzzy ({}% similar)",
                                path.display(),
                                (fuzzy_result.best_score * 100.0) as u32
                            ),
                            json!({
                                "file": path.display().to_string(),
                                "mode": "replace",
                                "matchType": "fuzzy",
                                "similarity": fuzzy_result.best_score,
                                "old": fuzzy_result.best_match_content,
                                "new": new_text
                            }),
                        ));
                    }
                }

                // 3. 模糊匹配也失败
                let similarity_pct = (fuzzy_result.best_score * 100.0) as u32;
                let threshold_pct = (DEFAULT_FUZZY_THRESHOLD * 100.0) as u32;

                let error_msg = format!(
                    "No sufficiently similar match found ({}% similar, needs {}%)\n\n\
                    Suggestions:\n\
                    1. Use read_file to verify the file's current content\n\
                    2. Check for exact whitespace/indentation match\n\
                    3. Provide more context to make the match unique\n\n\
                    Best match found:\n{}",
                    similarity_pct,
                    threshold_pct,
                    if fuzzy_result.best_match_content.is_empty() {
                        "(no match)".to_string()
                    } else {
                        fuzzy_result
                            .best_match_content
                            .lines()
                            .take(5)
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                );

                return Ok(error_result(error_msg));
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
                        "File {} may be binary",
                        path.display()
                    )));
                }

                let (mut lines, trailing_newline) = match fs::metadata(&path).await {
                    Ok(meta) => {
                        if meta.is_dir() {
                            return Ok(error_result(format!(
                                "Path {} is a directory",
                                path.display()
                            )));
                        }
                        if is_probably_binary(&path) {
                            return Ok(error_result(format!(
                                "File {} may be binary",
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
                        "Failed to write file {}: {}",
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
                        "file": path.display().to_string(),
                        "mode": "insert",
                        "line": after_line,
                        "old": "",
                        "new": content
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
                        return Ok(error_result(format!("Failed to parse patch: {}", err)));
                    }
                };

                let updated = match apply(&original, &patch) {
                    Ok(result) => result,
                    Err(err) => {
                        return Ok(error_result(format!("Failed to apply patch: {}", err)));
                    }
                };

                if let Err(err) = fs::write(&path, &updated).await {
                    return Ok(error_result(format!(
                        "Failed to write file {}: {}",
                        path.display(),
                        err
                    )));
                }

                success_result(
                    format!("edit_file applied\nmode=diff\nfile={}", path.display()),
                    json!({
                        "file": path.display().to_string(),
                        "mode": "diff",
                        "old": "",
                        "new": ""
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
        content: vec![ToolResultContent::Success(text)],
        is_error: false,
        execution_time_ms: None,
        ext_info: Some(ext),
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
