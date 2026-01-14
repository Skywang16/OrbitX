use std::path::{Path, PathBuf};

use async_trait::async_trait;
use diffy::{apply, Patch};
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::agent::context::FileOperationRecord;
use crate::agent::context::FileRecordSource;
use crate::agent::core::context::TaskContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
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

/// 匹配策略枚举
#[derive(Debug, Clone)]
enum MatchStrategy {
    Exact,                // 精确匹配
    LineTrimmed,          // 忽略行首空白
    WhitespaceNormalized, // 空白规范化
    IndentationFlexible,  // 缩进弹性匹配
    BlockAnchor,          // 首尾行锚定 + 模糊匹配
}

/// 匹配结果
#[derive(Debug, Clone)]
struct MatchResult {
    strategy: MatchStrategy,
    score: f64,
    match_index: usize,
    match_content: String,
}

/// 多策略匹配器
struct MultiStrategyMatcher;

impl MultiStrategyMatcher {
    /// 按优先级顺序尝试多种匹配策略
    fn find_best_match(lines: &[&str], search_text: &str, threshold: f64) -> Option<MatchResult> {
        let strategies = [
            MatchStrategy::Exact,
            MatchStrategy::LineTrimmed,
            MatchStrategy::WhitespaceNormalized,
            MatchStrategy::IndentationFlexible,
            MatchStrategy::BlockAnchor,
        ];

        for strategy in &strategies {
            if let Some(result) = Self::try_strategy(lines, search_text, strategy.clone()) {
                if result.score >= threshold {
                    return Some(result);
                }
            }
        }

        None
    }

    /// 尝试特定策略
    fn try_strategy(
        lines: &[&str],
        search_text: &str,
        strategy: MatchStrategy,
    ) -> Option<MatchResult> {
        match strategy {
            MatchStrategy::Exact => Self::exact_match(lines, search_text),
            MatchStrategy::LineTrimmed => Self::line_trimmed_match(lines, search_text),
            MatchStrategy::WhitespaceNormalized => {
                Self::whitespace_normalized_match(lines, search_text)
            }
            MatchStrategy::IndentationFlexible => {
                Self::indentation_flexible_match(lines, search_text)
            }
            MatchStrategy::BlockAnchor => Self::block_anchor_match(lines, search_text),
        }
    }

    /// 策略1: 精确匹配
    fn exact_match(lines: &[&str], search_text: &str) -> Option<MatchResult> {
        let full_text = lines.join("\n");
        if let Some(start) = full_text.find(search_text) {
            // 计算匹配的行索引
            let before_match = &full_text[..start];
            let match_line_start = before_match.lines().count().saturating_sub(1);

            return Some(MatchResult {
                strategy: MatchStrategy::Exact,
                score: 1.0,
                match_index: match_line_start,
                match_content: search_text.to_string(),
            });
        }
        None
    }

    /// 策略2: 忽略行首空白匹配
    fn line_trimmed_match(lines: &[&str], search_text: &str) -> Option<MatchResult> {
        let search_lines: Vec<&str> = search_text.lines().collect();
        if search_lines.is_empty() {
            return None;
        }

        let search_trimmed: Vec<&str> = search_lines.iter().map(|line| line.trim_start()).collect();

        for (i, window) in lines.windows(search_lines.len()).enumerate() {
            let window_trimmed: Vec<&str> = window.iter().map(|line| line.trim_start()).collect();

            if window_trimmed == search_trimmed {
                return Some(MatchResult {
                    strategy: MatchStrategy::LineTrimmed,
                    score: 0.95, // 略低于精确匹配
                    match_index: i,
                    match_content: window.join("\n"),
                });
            }
        }
        None
    }

    /// 策略3: 空白规范化匹配
    fn whitespace_normalized_match(lines: &[&str], search_text: &str) -> Option<MatchResult> {
        let normalize_whitespace = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");

        let search_normalized = normalize_whitespace(search_text);
        let search_lines: Vec<&str> = search_text.lines().collect();

        for (i, window) in lines.windows(search_lines.len()).enumerate() {
            let window_text = window.join("\n");
            let window_normalized = normalize_whitespace(&window_text);

            if window_normalized == search_normalized {
                return Some(MatchResult {
                    strategy: MatchStrategy::WhitespaceNormalized,
                    score: 0.90,
                    match_index: i,
                    match_content: window_text,
                });
            }
        }
        None
    }

    /// 策略4: 缩进弹性匹配
    fn indentation_flexible_match(lines: &[&str], search_text: &str) -> Option<MatchResult> {
        let search_lines: Vec<&str> = search_text.lines().collect();
        if search_lines.is_empty() {
            return None;
        }

        // 获取搜索文本的相对缩进模式
        let search_indents = Self::get_relative_indents(&search_lines);

        for (i, window) in lines.windows(search_lines.len()).enumerate() {
            let window_indents = Self::get_relative_indents(window);

            // 比较相对缩进模式和内容
            if Self::indents_match(&search_indents, &window_indents)
                && Self::content_matches_ignoring_indent(&search_lines, window)
            {
                return Some(MatchResult {
                    strategy: MatchStrategy::IndentationFlexible,
                    score: 0.85,
                    match_index: i,
                    match_content: window.join("\n"),
                });
            }
        }
        None
    }

    /// 策略5: 首尾行锚定 + 模糊匹配
    fn block_anchor_match(lines: &[&str], search_text: &str) -> Option<MatchResult> {
        let search_lines: Vec<&str> = search_text.lines().collect();
        if search_lines.len() < 2 {
            return None; // 需要至少2行才能做首尾锚定
        }

        let first_search_line = search_lines[0].trim();
        let last_search_line = search_lines[search_lines.len() - 1].trim();

        for (i, window) in lines.windows(search_lines.len()).enumerate() {
            let first_window_line = window[0].trim();
            let last_window_line = window[window.len() - 1].trim();

            // 首尾行必须匹配
            if first_search_line == first_window_line && last_search_line == last_window_line {
                // 计算中间内容的相似度
                let middle_similarity = if search_lines.len() > 2 {
                    let search_middle = search_lines[1..search_lines.len() - 1].join("\n");
                    let window_middle = window[1..window.len() - 1].join("\n");
                    get_similarity(&window_middle, &search_middle)
                } else {
                    1.0 // 只有首尾两行，直接匹配
                };

                if middle_similarity >= 0.7 {
                    // 中间内容70%相似度即可
                    return Some(MatchResult {
                        strategy: MatchStrategy::BlockAnchor,
                        score: 0.8 + (middle_similarity * 0.15), // 0.8-0.95之间
                        match_index: i,
                        match_content: window.join("\n"),
                    });
                }
            }
        }
        None
    }

    /// 获取相对缩进级别
    fn get_relative_indents(lines: &[&str]) -> Vec<usize> {
        lines
            .iter()
            .map(|line| line.len() - line.trim_start().len())
            .collect()
    }

    /// 比较缩进模式是否匹配
    fn indents_match(search_indents: &[usize], window_indents: &[usize]) -> bool {
        if search_indents.len() != window_indents.len() {
            return false;
        }

        // 计算相对缩进差异
        let search_base = search_indents.first().copied().unwrap_or(0);
        let window_base = window_indents.first().copied().unwrap_or(0);

        for (&search_indent, &window_indent) in search_indents.iter().zip(window_indents.iter()) {
            let search_relative = search_indent.saturating_sub(search_base);
            let window_relative = window_indent.saturating_sub(window_base);

            if search_relative != window_relative {
                return false;
            }
        }
        true
    }

    /// 比较内容是否匹配（忽略缩进）
    fn content_matches_ignoring_indent(search_lines: &[&str], window_lines: &[&str]) -> bool {
        if search_lines.len() != window_lines.len() {
            return false;
        }

        for (search_line, window_line) in search_lines.iter().zip(window_lines.iter()) {
            if search_line.trim() != window_line.trim() {
                return false;
            }
        }
        true
    }
}

/// 获取行的前导空白字符
fn get_leading_whitespace(line: &str) -> &str {
    // trim_start() 返回的切片保证在字符边界上
    // 所以 line.len() - trimmed.len() 一定是有效的字节索引
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
                matched_base_indent.get(..new_len).unwrap_or("")
            } else {
                // 相对缩进为正或零，保持或增加缩进
                let extra = if current_level > search_base_level {
                    current_indent.get(search_base_level..).unwrap_or("")
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

impl Default for UnifiedEditTool {
    fn default() -> Self {
        Self::new()
    }
}

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

    async fn ensure_parent(path: &Path) -> Result<(), ToolResult> {
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
        r#"Performs smart string replacements, insertions, or diff applications in files with advanced multi-strategy matching and intelligent indentation preservation.

Usage:
- You MUST use the read_file tool at least once in the conversation before editing. This tool will error if you attempt an edit without reading the file first
- The path parameter must be an absolute path (e.g., '/Users/user/project/src/main.ts')
- When editing text from read_file tool output, ensure you preserve the exact indentation (tabs/spaces) as it appears in the file content
- ALWAYS prefer editing existing files in the codebase. NEVER write new files unless explicitly required
- Only use emojis if the user explicitly requests it. Avoid adding emojis to files unless asked

Edit Modes:
- mode="replace": Find and replace text with advanced multi-strategy matching
- mode="insert": Insert content at a specific position
- mode="diff": Apply unified diff patches to files

Replace Mode Guidelines:
- Uses 5 intelligent matching strategies in order: Exact → LineTrimmed → WhitespaceNormalized → IndentationFlexible → BlockAnchor
- Exact: Perfect string match (100% accuracy)
- LineTrimmed: Ignores leading whitespace differences (95% accuracy)
- WhitespaceNormalized: Normalizes all whitespace (90% accuracy)  
- IndentationFlexible: Matches content with flexible indentation (85% accuracy)
- BlockAnchor: Matches first/last lines exactly, fuzzy matches middle content (80-95% accuracy)
- Include enough surrounding context to make the old_text unique in the file
- Indentation is automatically preserved: the tool detects the original file's indentation style and applies it to replacements
- For renaming variables/functions across a file, consider using multiple replace operations

Insert Mode Guidelines:
- Use after_line parameter (0-based line index) to specify insertion point
- Use 0 to insert at the beginning of the file
- Content will be inserted with appropriate indentation

Diff Mode Guidelines:
- Provide unified diff format patches
- Useful for complex multi-location changes
- The tool will validate and apply the patch safely

Examples:
- Replace text: {"path": "/path/file.js", "mode": "replace", "old_text": "function oldName() {\n  return true;\n}", "new_text": "function newName() {\n  return false;\n}"}
- Insert at position: {"path": "/path/file.js", "mode": "insert", "after_line": 10, "content": "// New comment\nconst newVar = 'value';"}
- Apply diff: {"path": "/path/file.js", "mode": "diff", "diff_content": "--- a/file.js\n+++ b/file.js\n@@ -1,3 +1,3 @@\n-old line\n+new line"}"#
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
                    "description": "Edit mode: 'replace' for find-and-replace, 'insert' for inserting at a position, 'diff' for applying unified diffs"
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
                    "description": "[insert mode only] 0-based line index after which to insert content. Use 0 to insert at the beginning of the file."
                },
                "content": {
                    "type": "string",
                    "description": "[insert mode only] The content to insert at the specified position."
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
            .with_confirmation()
            .with_tags(vec!["filesystem".into(), "edit".into()])
            .with_summary_key_arg("path")
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
                            "Found {match_count} matches. Please provide more context to make a unique match."
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

                    context.note_agent_write_intent(path.as_path()).await;
                    snapshot_before_edit(context, self.name(), path.as_path()).await?;

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

                // 2. 精确匹配失败，尝试多策略模糊匹配
                if let Some(match_result) = MultiStrategyMatcher::find_best_match(
                    &lines,
                    &old_text,
                    DEFAULT_FUZZY_THRESHOLD,
                ) {
                    let match_index = match_result.match_index;

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
                    let before_match: Vec<&str> = lines.iter().take(match_index).copied().collect();
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

                    context.note_agent_write_intent(path.as_path()).await;
                    snapshot_before_edit(context, self.name(), path.as_path()).await?;

                    if let Err(err) = fs::write(&path, &updated).await {
                        return Ok(error_result(format!(
                            "Failed to write file {}: {}",
                            path.display(),
                            err
                        )));
                    }

                    return Ok(success_result(
                        format!(
                            "edit_file applied\nmode=replace\nfile={}\nmatch={:?} ({}% similar)",
                            path.display(),
                            match_result.strategy,
                            (match_result.score * 100.0) as u32
                        ),
                        json!({
                            "file": path.display().to_string(),
                            "mode": "replace",
                            "matchType": format!("{:?}", match_result.strategy).to_lowercase(),
                            "similarity": match_result.score,
                            "old": match_result.match_content,
                            "new": new_text
                        }),
                    ));
                }

                // 3. 所有匹配策略都失败
                let threshold_pct = (DEFAULT_FUZZY_THRESHOLD * 100.0) as u32;

                let error_msg = format!(
                    "No sufficiently similar match found (needs {threshold_pct}% similarity)\n\n\
                    Tried strategies: Exact, LineTrimmed, WhitespaceNormalized, IndentationFlexible, BlockAnchor\n\n\
                    Suggestions:\n\
                    1. Use read_file to verify the file's current content\n\
                    2. Check for exact whitespace/indentation match\n\
                    3. Provide more context to make the match unique\n\
                    4. Try using mode='outline' to see file structure first"
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
                lines.splice(position..position, insert_lines);
                let mut updated = lines.join("\n");
                if (trailing_newline || content.ends_with('\n')) && !updated.ends_with('\n') {
                    updated.push('\n');
                }

                context.note_agent_write_intent(path.as_path()).await;
                snapshot_before_edit(context, self.name(), path.as_path()).await?;

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
                        return Ok(error_result(format!("Failed to parse patch: {err}")));
                    }
                };

                let updated = match apply(&original, &patch) {
                    Ok(result) => result,
                    Err(err) => {
                        return Ok(error_result(format!("Failed to apply patch: {err}")));
                    }
                };

                context.note_agent_write_intent(path.as_path()).await;
                snapshot_before_edit(context, self.name(), path.as_path()).await?;

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
        status: ToolResultStatus::Success,
        cancel_reason: None,
        execution_time_ms: None,
        ext_info: Some(ext),
    }
}

fn error_result(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        status: ToolResultStatus::Error,
        cancel_reason: None,
        execution_time_ms: None,
        ext_info: None,
    }
}

async fn snapshot_before_edit(
    context: &TaskContext,
    tool_name: &str,
    path: &Path,
) -> ToolExecutorResult<()> {
    context
        .snapshot_file_before_edit(path)
        .await
        .map_err(|err| ToolExecutorError::ExecutionFailed {
            tool_name: tool_name.to_string(),
            error: err.to_string(),
        })
}
