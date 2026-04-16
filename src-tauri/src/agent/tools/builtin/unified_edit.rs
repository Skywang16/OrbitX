// Edit tool — sourced from opencode-dev's design:
// Flat parameters, pipeline-based replacers, no mode switching.
// References:
//   https://github.com/cline/cline/blob/main/evals/diff-edits/diff-apply/
//   https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/utils/editCorrector.ts

use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::agent::context::FileOperationRecord;
use crate::agent::context::FileRecordSource;
use crate::agent::core::context::AgentRunContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};

use super::file_utils::{ensure_absolute, is_probably_binary};

// ============================================================================
// Args — flat structure, no tagged enum, no serde(flatten)
// ============================================================================

#[derive(Debug, Deserialize)]
struct EditFileArgs {
    #[serde(alias = "filePath", alias = "file_path")]
    path: String,
    #[serde(alias = "oldString", alias = "old_string")]
    old_text: String,
    #[serde(alias = "newString", alias = "new_string")]
    new_text: String,
    #[serde(default, alias = "replaceAll", alias = "replace_all")]
    replace_all: bool,
}

// ============================================================================
// Replacer pipeline — each replacer yields candidate match strings
// ============================================================================

/// A Replacer takes (content, search_text) and yields candidate exact substrings
/// of `content` that the search_text was intended to match.
type Replacer = fn(content: &str, find: &str) -> Vec<String>;

/// Strategy 1: Exact / simple match
fn simple_replacer(content: &str, find: &str) -> Vec<String> {
    if content.contains(find) {
        vec![find.to_string()]
    } else {
        vec![]
    }
}

/// Strategy 2: Line-trimmed match (ignore leading whitespace per line)
fn line_trimmed_replacer(content: &str, find: &str) -> Vec<String> {
    let original_lines: Vec<&str> = content.split('\n').collect();
    let search_lines: Vec<&str> = find.split('\n').collect();

    let search_trimmed: Vec<&str> = search_lines.iter().map(|l| l.trim_start()).collect();
    if search_trimmed.is_empty() {
        return vec![];
    }

    let mut results = Vec::new();
    for i in 0..=original_lines.len().saturating_sub(search_lines.len()) {
        let window = &original_lines[i..i + search_lines.len()];
        let window_trimmed: Vec<&str> = window.iter().map(|l| l.trim_start()).collect();

        if window_trimmed == search_trimmed {
            // Compute byte range in original content
            let start_idx = original_lines[..i]
                .iter()
                .map(|line| line.len() + 1)
                .sum::<usize>();
            let mut end_idx = start_idx;
            for (k, line) in original_lines[i..i + search_lines.len()].iter().enumerate() {
                end_idx += line.len();
                if k < search_lines.len() - 1 {
                    end_idx += 1; // newline
                }
            }
            if end_idx <= content.len() {
                results.push(content[start_idx..end_idx].to_string());
            }
        }
    }
    results
}

/// Strategy 3: Block anchor — first/last line anchor + fuzzy middle
fn block_anchor_replacer(content: &str, find: &str) -> Vec<String> {
    let original_lines: Vec<&str> = content.split('\n').collect();
    let mut search_lines: Vec<&str> = find.split('\n').collect();

    if search_lines.len() < 3 {
        return vec![];
    }

    // Remove trailing empty line if present
    if search_lines.last().is_some_and(|line| line.is_empty()) {
        search_lines.pop();
    }
    if search_lines.len() < 3 {
        return vec![];
    }

    let first_search = search_lines[0].trim();
    let last_search = search_lines[search_lines.len() - 1].trim();
    let search_block_size = search_lines.len();

    // Collect candidates where both anchors match
    struct Candidate {
        start: usize,
        end: usize,
    }
    let mut candidates = Vec::new();
    for i in 0..original_lines.len() {
        if original_lines[i].trim() != first_search {
            continue;
        }
        if let Some(j) = original_lines[(i + 2)..]
            .iter()
            .position(|line| line.trim() == last_search)
        {
            candidates.push(Candidate {
                start: i,
                end: i + 2 + j,
            });
        }
    }

    if candidates.is_empty() {
        return vec![];
    }

    // Pick best candidate by similarity
    let mut best: Option<(usize, usize, f64)> = None;
    let threshold = if candidates.len() == 1 { 0.0 } else { 0.3 };

    for c in &candidates {
        let actual_size = c.end - c.start + 1;
        let middle_count = search_block_size
            .saturating_sub(2)
            .min(actual_size.saturating_sub(2));

        let similarity = if middle_count > 0 {
            let mut sim = 0.0;
            for j in 1..search_block_size.min(actual_size) - 1 {
                let orig = original_lines[c.start + j].trim();
                let search = search_lines[j].trim();
                let max_len = orig.len().max(search.len());
                if max_len == 0 {
                    continue;
                }
                let dist = levenshtein_distance(orig, search);
                sim += (1.0 - dist as f64 / max_len as f64) / middle_count as f64;
            }
            sim
        } else {
            1.0
        };

        if similarity >= threshold
            && best
                .as_ref()
                .is_none_or(|candidate| similarity > candidate.2)
        {
            best = Some((c.start, c.end, similarity));
        }
    }

    if let Some((start, end, _)) = best {
        let start_idx = original_lines[..start]
            .iter()
            .map(|line| line.len() + 1)
            .sum::<usize>();
        let mut end_idx = start_idx;
        for (idx, line) in original_lines[start..=end].iter().enumerate() {
            end_idx += line.len();
            if start + idx < end {
                end_idx += 1;
            }
        }
        if end_idx <= content.len() {
            return vec![content[start_idx..end_idx].to_string()];
        }
    }

    vec![]
}

/// Strategy 4: Whitespace normalized match
fn whitespace_normalized_replacer(content: &str, find: &str) -> Vec<String> {
    let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized_find = normalize(find);

    let lines: Vec<&str> = content.split('\n').collect();
    let mut results = Vec::new();

    // Single line matches
    for line in &lines {
        if normalize(line) == normalized_find {
            results.push(line.to_string());
        }
    }

    // Multi-line matches
    let find_lines: Vec<&str> = find.split('\n').collect();
    if find_lines.len() > 1 {
        for i in 0..=lines.len().saturating_sub(find_lines.len()) {
            let block: Vec<&str> = lines[i..i + find_lines.len()].to_vec();
            let block_text = block.join("\n");
            if normalize(&block_text) == normalized_find {
                results.push(block_text);
            }
        }
    }

    results
}

/// Strategy 5: Indentation flexible match (strip common indent, compare)
fn indentation_flexible_replacer(content: &str, find: &str) -> Vec<String> {
    let remove_indent = |text: &str| -> String {
        let lines: Vec<&str> = text.split('\n').collect();
        let non_empty: Vec<&&str> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
        if non_empty.is_empty() {
            return text.to_string();
        }
        let min_indent = non_empty
            .iter()
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);
        lines
            .iter()
            .map(|l| {
                if l.trim().is_empty() {
                    *l
                } else if l.len() >= min_indent {
                    &l[min_indent..]
                } else {
                    l
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let normalized_find = remove_indent(find);
    let content_lines: Vec<&str> = content.split('\n').collect();
    let find_lines: Vec<&str> = find.split('\n').collect();

    let mut results = Vec::new();
    for i in 0..=content_lines.len().saturating_sub(find_lines.len()) {
        let block = content_lines[i..i + find_lines.len()].join("\n");
        if remove_indent(&block) == normalized_find {
            results.push(block);
        }
    }
    results
}

/// Strategy 6: Escape normalized (handles \n, \t, etc.)
fn escape_normalized_replacer(content: &str, find: &str) -> Vec<String> {
    let unescape = |s: &str| -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.peek() {
                    Some('n') => {
                        chars.next();
                        result.push('\n');
                    }
                    Some('t') => {
                        chars.next();
                        result.push('\t');
                    }
                    Some('r') => {
                        chars.next();
                        result.push('\r');
                    }
                    Some('\\') => {
                        chars.next();
                        result.push('\\');
                    }
                    Some('\'') => {
                        chars.next();
                        result.push('\'');
                    }
                    Some('"') => {
                        chars.next();
                        result.push('"');
                    }
                    Some('`') => {
                        chars.next();
                        result.push('`');
                    }
                    Some('$') => {
                        chars.next();
                        result.push('$');
                    }
                    _ => result.push(c),
                }
            } else {
                result.push(c);
            }
        }
        result
    };

    let unescaped_find = unescape(find);
    let mut results = Vec::new();

    // Direct match with unescaped find
    if content.contains(&unescaped_find) {
        results.push(unescaped_find.clone());
    }

    // Block-level matching with unescape
    let lines: Vec<&str> = content.split('\n').collect();
    let find_lines: Vec<&str> = unescaped_find.split('\n').collect();

    if find_lines.len() > 1 {
        for i in 0..=lines.len().saturating_sub(find_lines.len()) {
            let block = lines[i..i + find_lines.len()].join("\n");
            if unescape(&block) == unescaped_find {
                results.push(block);
            }
        }
    }

    results
}

/// Strategy 7: Trimmed boundary (try trimmed version of find)
fn trimmed_boundary_replacer(content: &str, find: &str) -> Vec<String> {
    let trimmed = find.trim();
    if trimmed == find {
        return vec![]; // already trimmed, nothing new
    }

    let mut results = Vec::new();

    if content.contains(trimmed) {
        results.push(trimmed.to_string());
    }

    // Block-level matching
    let lines: Vec<&str> = content.split('\n').collect();
    let find_lines: Vec<&str> = find.split('\n').collect();

    for i in 0..=lines.len().saturating_sub(find_lines.len()) {
        let block = lines[i..i + find_lines.len()].join("\n");
        if block.trim() == trimmed {
            results.push(block);
        }
    }

    results
}

/// Strategy 8: Context-aware (first/last anchor + 50% middle line match)
fn context_aware_replacer(content: &str, find: &str) -> Vec<String> {
    let mut find_lines: Vec<&str> = find.split('\n').collect();
    if find_lines.len() < 3 {
        return vec![];
    }
    if find_lines.last().is_some_and(|line| line.is_empty()) {
        find_lines.pop();
    }
    if find_lines.len() < 3 {
        return vec![];
    }

    let content_lines: Vec<&str> = content.split('\n').collect();
    let first_line = find_lines[0].trim();
    let last_line = find_lines[find_lines.len() - 1].trim();

    for i in 0..content_lines.len() {
        if content_lines[i].trim() != first_line {
            continue;
        }
        for j in (i + 2)..content_lines.len() {
            if content_lines[j].trim() != last_line {
                continue;
            }
            let block_lines = &content_lines[i..=j];
            if block_lines.len() != find_lines.len() {
                break;
            }
            // Check middle similarity
            let mut matching = 0usize;
            let mut total = 0usize;
            for k in 1..block_lines.len() - 1 {
                let bl = block_lines[k].trim();
                let fl = find_lines[k].trim();
                if !bl.is_empty() || !fl.is_empty() {
                    total += 1;
                    if bl == fl {
                        matching += 1;
                    }
                }
            }
            if total == 0 || (matching as f64 / total as f64) >= 0.5 {
                return vec![block_lines.join("\n")];
            }
            break;
        }
    }
    vec![]
}

/// All exact-occurrence finder (for replaceAll support)
fn multi_occurrence_replacer(content: &str, find: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut start = 0;
    while let Some(idx) = content[start..].find(find) {
        results.push(find.to_string());
        start += idx + find.len();
    }
    results
}

// ============================================================================
// Core replace function — pipeline of replacers
// ============================================================================

/// Replace old_text with new_text in content, using a pipeline of increasingly
/// fuzzy matching strategies. Returns the new content or an error message.
pub(crate) fn replace(
    content: &str,
    old_text: &str,
    new_text: &str,
    replace_all: bool,
) -> Result<String, String> {
    if old_text == new_text {
        return Err("oldString and newString must be different".to_string());
    }

    let replacers: &[Replacer] = &[
        simple_replacer,
        line_trimmed_replacer,
        block_anchor_replacer,
        whitespace_normalized_replacer,
        indentation_flexible_replacer,
        escape_normalized_replacer,
        trimmed_boundary_replacer,
        context_aware_replacer,
        multi_occurrence_replacer,
    ];

    let mut not_found = true;

    for replacer in replacers {
        let candidates = replacer(content, old_text);
        for search in &candidates {
            let index = match content.find(search.as_str()) {
                Some(idx) => idx,
                None => continue,
            };
            not_found = false;

            if replace_all {
                return Ok(content.replace(search.as_str(), new_text));
            }

            // Check uniqueness: must have exactly one occurrence
            let last_index = match content.rfind(search.as_str()) {
                Some(last_index) => last_index,
                None => continue,
            };
            if index != last_index {
                continue; // multiple matches, try next candidate/replacer
            }

            return Ok(format!(
                "{}{}{}",
                &content[..index],
                new_text,
                &content[index + search.len()..]
            ));
        }
    }

    if not_found {
        Err("oldString not found in content. Use the read_file tool to verify the file's current content, then retry with the correct oldString.".to_string())
    } else {
        Err("Found multiple matches for oldString. Provide more surrounding lines in oldString to identify the correct match, or set replaceAll to true.".to_string())
    }
}

// ============================================================================
// Levenshtein distance
// ============================================================================

fn levenshtein_distance(a: &str, b: &str) -> usize {
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr: Vec<usize> = vec![0; b_len + 1];

    for i in 1..=a_len {
        curr[0] = i;
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_len]
}

// ============================================================================
// Tool implementation
// ============================================================================

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
}

#[async_trait]
impl RunnableTool for UnifiedEditTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        r#"Performs exact string replacements in files.

Usage:
- You MUST use the read_file tool at least once before editing. This tool will error if you haven't read the file first.
- When editing text from read_file output, preserve the exact indentation (tabs/spaces) as it appears AFTER the line number prefix. The line number prefix format is: spaces + line number + tab. Everything after that tab is the actual file content to match. Never include any part of the line number prefix in old_text or new_text.
- ALWAYS prefer editing existing files. NEVER write new files unless explicitly required.
- Only use emojis if the user explicitly requests it. Avoid adding emojis to files unless asked.

Failure Modes:
- The edit will FAIL if old_text is not found in the file with error "oldString not found in content".
- The edit will FAIL if old_text matches multiple locations with error "oldString found multiple times". Either provide a larger string with more surrounding context to make it unique, or use replace_all to change every instance of old_text.

Parameters:
- path: The absolute path to the file to modify.
- old_text: The text to replace. Must match file content (fuzzy matching handles minor whitespace/indent differences).
- new_text: The text to replace it with. Must be different from old_text.
- replace_all: If true, replaces ALL occurrences of old_text. Useful for renaming variables. Default: false."#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute path to the file to modify"
                },
                "old_text": {
                    "type": "string",
                    "description": "The text to replace"
                },
                "new_text": {
                    "type": "string",
                    "description": "The text to replace it with (must be different from old_text)"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace all occurrences of old_text (default false)"
                }
            },
            "required": ["path", "old_text", "new_text"]
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
        context: &AgentRunContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: EditFileArgs = serde_json::from_value(args)?;

        let path = match ensure_absolute(&args.path, &context.cwd) {
            Ok(resolved) => resolved,
            Err(err) => return Ok(error_result(err.to_string())),
        };

        // Must read file before editing
        if let Err(e) = context
            .file_tracker()
            .assert_file_not_modified(path.as_path())
            .await
        {
            return Ok(error_result(e.to_string()));
        }

        // Validate file exists and is readable
        let original = match load_file_text(&path).await {
            Ok(text) => text,
            Err(err) => return Ok(err),
        };

        if args.old_text.is_empty() {
            return Ok(error_result(
                "oldString cannot be empty for edit_file. Use write_file to create or replace an entire file."
                    .to_string(),
            ));
        }

        // Normalize line endings for matching
        let normalized_content = original.replace("\r\n", "\n");
        let normalized_old = args.old_text.replace("\r\n", "\n");

        // Run the replace pipeline
        let updated = match replace(
            &normalized_content,
            &normalized_old,
            &args.new_text,
            args.replace_all,
        ) {
            Ok(result) => result,
            Err(err_msg) => return Ok(error_result(err_msg)),
        };

        // Preserve original line endings if the file used CRLF
        let final_content = if original.contains("\r\n") {
            updated.replace('\n', "\r\n")
        } else {
            updated
        };

        context.note_agent_write_intent(path.as_path()).await;
        snapshot_before_edit(context, self.name(), path.as_path()).await?;

        if let Err(err) = fs::write(&path, &final_content).await {
            return Ok(error_result(format!(
                "Failed to write file {}: {}",
                path.display(),
                err
            )));
        }

        track_edit(context, &path).await?;

        Ok(success_result(
            format!("edit_file applied\nfile={}", path.display()),
            json!({
                "file": path.display().to_string(),
                "old": args.old_text,
                "new": args.new_text,
            }),
        ))
    }
}

// ============================================================================
// Helpers
// ============================================================================

pub(crate) async fn load_file_text(path: &Path) -> Result<String, ToolResult> {
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
            "File {} appears to be binary",
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

pub(crate) async fn track_edit(context: &AgentRunContext, path: &Path) -> ToolExecutorResult<()> {
    context
        .file_tracker()
        .track_file_operation(FileOperationRecord::new(
            path,
            FileRecordSource::AgentEdited,
        ))
        .await?;

    context.file_tracker().record_file_mtime(path).await?;

    Ok(())
}

pub(crate) fn success_result(text: String, ext: serde_json::Value) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Success(text)],
        status: ToolResultStatus::Success,
        cancel_reason: None,
        execution_time_ms: None,
        ext_info: Some(ext),
    }
}

pub(crate) fn error_result(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        status: ToolResultStatus::Error,
        cancel_reason: None,
        execution_time_ms: None,
        ext_info: None,
    }
}

pub(crate) async fn snapshot_before_edit(
    context: &AgentRunContext,
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_replace() {
        let content = "fn main() {\n    println!(\"hello\");\n}";
        let result = replace(content, "println!(\"hello\")", "println!(\"world\")", false);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("println!(\"world\")"));
    }

    #[test]
    fn test_replace_not_found() {
        let content = "fn main() {}";
        let result = replace(content, "nonexistent", "replacement", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_replace_all() {
        let content = "let x = foo;\nlet y = foo;\nlet z = foo;";
        let result = replace(content, "foo", "bar", true);
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert!(!updated.contains("foo"));
        assert_eq!(updated.matches("bar").count(), 3);
    }

    #[test]
    fn test_multiple_matches_no_replace_all() {
        let content = "foo\nfoo\nfoo";
        let result = replace(content, "foo", "bar", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("multiple matches"));
    }

    #[test]
    fn test_line_trimmed_match() {
        let content = "    fn hello() {\n        world();\n    }";
        let find = "fn hello() {\n    world();\n}";
        let result = replace(content, find, "fn goodbye() {}", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_whitespace_normalized() {
        let content = "let   x  =   1;";
        let find = "let x = 1;";
        let result = replace(content, find, "let x = 2;", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_same_old_new() {
        let result = replace("content", "same", "same", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be different"));
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }
}
