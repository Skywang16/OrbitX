use std::cmp::min;
use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;
use tree_sitter::{Parser, TreeCursor};

use crate::agent::context::FileOperationRecord;
use crate::agent::context::FileRecordSource;
use crate::agent::core::context::AgentRunContext;
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};
use crate::vector_db::core::Language;

use super::file_utils::{ensure_absolute, is_probably_binary};

const DEFAULT_MAX_LINES: usize = 2000;
const MAX_LINE_LENGTH: usize = 2000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadFileArgs {
    path: String,
    offset: Option<i64>,
    limit: Option<i64>,
    /// Reading mode: "full" (default), "outline", "symbol"
    mode: Option<String>,
    /// When mode="symbol", specify the symbol name to read
    symbol: Option<String>,
}

pub struct ReadFileTool;

impl Default for ReadFileTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadFileTool {
    pub fn new() -> Self {
        Self
    }

    /// Read full file content (original logic)
    fn read_full(&self, content: &str, offset: Option<i64>, limit: Option<i64>) -> ToolResult {
        let lines: Vec<&str> = content.split('\n').collect();
        let total_lines = lines.len();

        let offset = match offset {
            Some(v) if v < 0 => {
                return validation_error("offset must be greater than or equal to 0");
            }
            Some(v) => v as usize,
            None => 0,
        };
        let limit = match limit {
            Some(v) if v <= 0 => {
                return validation_error("limit must be greater than 0");
            }
            Some(v) => v as usize,
            None => DEFAULT_MAX_LINES,
        };

        let start_line = min(offset, total_lines);
        let end_line = min(start_line.saturating_add(limit), total_lines);

        let mut output_lines = Vec::new();
        let mut truncated_line_detected = false;

        for line in lines
            .iter()
            .skip(start_line)
            .take(end_line.saturating_sub(start_line))
        {
            let mut char_iter = line.chars();
            let mut truncated = String::new();
            for _ in 0..MAX_LINE_LENGTH {
                if let Some(ch) = char_iter.next() {
                    truncated.push(ch);
                } else {
                    break;
                }
            }
            if char_iter.next().is_some() {
                truncated.push_str("... [truncated]");
                truncated_line_detected = true;
            }
            output_lines.push(truncated);
        }

        let result_text = output_lines.join("\n");

        ToolResult {
            content: vec![ToolResultContent::Success(result_text)],
            status: ToolResultStatus::Success,
            cancel_reason: None,
            execution_time_ms: None,
            ext_info: Some(json!({
                "mode": "full",
                "startLine": if total_lines == 0 { 0 } else { start_line + 1 },
                "endLine": end_line,
                "totalLines": total_lines,
                "limit": limit,
                "linesReturned": output_lines.len(),
                "hasMore": end_line < total_lines,
                "lineTruncated": truncated_line_detected,
            })),
        }
    }

    /// Read code outline (reuse tree-sitter parsing from vector module)
    fn read_outline(&self, path: &Path, content: &str) -> ToolResult {
        if let Some(language) = Language::from_path(path) {
            match self.extract_code_outline(content, language) {
                Ok(outline) => ToolResult {
                    content: vec![ToolResultContent::Success(outline)],
                    status: ToolResultStatus::Success,
                    cancel_reason: None,
                    execution_time_ms: None,
                    ext_info: Some(json!({
                        "mode": "outline",
                        "language": format!("{:?}", language),
                    })),
                },
                Err(e) => validation_error(format!("Failed to parse code outline: {e}")),
            }
        } else {
            // Unsupported language, return simple line number index
            let lines: Vec<&str> = content.lines().collect();
            let mut outline = Vec::new();
            outline.push(format!("File: {} ({} lines)", path.display(), lines.len()));
            outline.push("Language not supported for syntax parsing".to_string());

            ToolResult {
                content: vec![ToolResultContent::Success(outline.join("\n"))],
                status: ToolResultStatus::Success,
                cancel_reason: None,
                execution_time_ms: None,
                ext_info: Some(json!({
                    "mode": "outline",
                    "language": "unsupported",
                })),
            }
        }
    }

    /// Read code for specified symbol
    fn read_symbol(&self, path: &Path, content: &str, symbol_name: &str) -> ToolResult {
        if let Some(language) = Language::from_path(path) {
            match self.find_symbol_range(content, language, symbol_name) {
                Ok(Some((start_line, end_line))) => {
                    let lines: Vec<&str> = content.lines().collect();
                    let Some(symbol_lines) = (start_line <= end_line && end_line < lines.len())
                        .then(|| lines.get(start_line..=end_line))
                        .flatten()
                    else {
                        return validation_error(format!(
                            "Invalid symbol range resolved for '{symbol_name}' in {}",
                            path.display()
                        ));
                    };
                    let result_text = symbol_lines.join("\n");

                    ToolResult {
                        content: vec![ToolResultContent::Success(result_text)],
                        status: ToolResultStatus::Success,
                        cancel_reason: None,
                        execution_time_ms: None,
                        ext_info: Some(json!({
                            "mode": "symbol",
                            "symbol": symbol_name,
                            "startLine": start_line + 1,
                            "endLine": end_line + 1,
                            "linesReturned": symbol_lines.len(),
                        })),
                    }
                }
                Ok(None) => validation_error(format!("Symbol '{symbol_name}' not found in file")),
                Err(e) => validation_error(format!("Failed to parse file: {e}")),
            }
        } else {
            validation_error("Language not supported for symbol extraction")
        }
    }

    /// Extract code outline (reuse vector module logic)
    fn extract_code_outline(
        &self,
        content: &str,
        language: Language,
    ) -> ToolExecutorResult<String> {
        let mut parser = Parser::new();

        // Set language parser (reuse vector module logic)
        match language {
            Language::Python => {
                parser
                    .set_language(&tree_sitter_python::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set Python language: {e}"),
                    })?;
            }
            Language::TypeScript => {
                parser
                    .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set TypeScript language: {e}"),
                    })?;
            }
            Language::JavaScript => {
                parser
                    .set_language(&tree_sitter_javascript::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set JavaScript language: {e}"),
                    })?;
            }
            Language::Rust => {
                parser
                    .set_language(&tree_sitter_rust::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set Rust language: {e}"),
                    })?;
            }
            Language::Go => {
                parser
                    .set_language(&tree_sitter_go::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set Go language: {e}"),
                    })?;
            }
            Language::Java => {
                parser
                    .set_language(&tree_sitter_java::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set Java language: {e}"),
                    })?;
            }
            Language::C => {
                parser
                    .set_language(&tree_sitter_c::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set C language: {e}"),
                    })?;
            }
            Language::Cpp => {
                parser
                    .set_language(&tree_sitter_cpp::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set C++ language: {e}"),
                    })?;
            }
            Language::CSharp => {
                parser
                    .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set C# language: {e}"),
                    })?;
            }
            Language::Ruby => {
                parser
                    .set_language(&tree_sitter_ruby::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set Ruby language: {e}"),
                    })?;
            }
            _ => {
                return Err(ToolExecutorError::InvalidArguments {
                    tool_name: "read_file".to_string(),
                    error: format!("Language {language:?} not supported for parsing"),
                });
            }
        }

        let tree =
            parser
                .parse(content, None)
                .ok_or_else(|| ToolExecutorError::ExecutionFailed {
                    tool_name: "read_file".to_string(),
                    error: format!("Failed to parse {language:?} code"),
                })?;

        let mut outline = Vec::new();
        let mut cursor = tree.root_node().walk();

        Self::extract_outline_recursive(&mut cursor, content, &mut outline, language, 0);

        Ok(outline.join("\n"))
    }

    /// Recursively extract outline
    fn extract_outline_recursive(
        cursor: &mut TreeCursor,
        source: &str,
        outline: &mut Vec<String>,
        language: Language,
        depth: usize,
    ) {
        let node = cursor.node();
        let node_kind = node.kind();

        // Determine if it's an important code structure
        let is_important = match language {
            Language::Python => {
                matches!(node_kind, "function_definition" | "class_definition")
            }
            Language::TypeScript | Language::JavaScript => {
                matches!(
                    node_kind,
                    "function_declaration"
                        | "class_declaration"
                        | "method_definition"
                        | "arrow_function"
                )
            }
            Language::Rust => {
                matches!(
                    node_kind,
                    "function_item"
                        | "impl_item"
                        | "struct_item"
                        | "enum_item"
                        | "trait_item"
                        | "mod_item"
                )
            }
            Language::Go => {
                matches!(
                    node_kind,
                    "function_declaration" | "method_declaration" | "type_declaration"
                )
            }
            Language::Java | Language::CSharp => {
                matches!(
                    node_kind,
                    "method_declaration" | "class_declaration" | "interface_declaration"
                )
            }
            Language::C | Language::Cpp => {
                matches!(
                    node_kind,
                    "function_definition" | "struct_specifier" | "class_specifier"
                )
            }
            Language::Ruby => {
                matches!(node_kind, "method" | "class" | "module")
            }
            _ => false,
        };

        if is_important {
            let start_pos = node.start_position();
            let end_pos = node.end_position();
            let start_byte = node.start_byte();
            let end_byte = node.end_byte().min(start_byte + 200); // Limit preview length

            let Some(preview) = source.get(start_byte..end_byte) else {
                return;
            };
            let Some(first_line) = preview.lines().next() else {
                return;
            };
            let first_line = first_line.trim();

            let indent = "  ".repeat(depth);
            outline.push(format!(
                "{}├── {} ({}:{}-{}:{})",
                indent,
                first_line,
                start_pos.row + 1,
                start_pos.column + 1,
                end_pos.row + 1,
                end_pos.column + 1
            ));
        }

        // Recursively process child nodes
        if cursor.goto_first_child() {
            loop {
                Self::extract_outline_recursive(cursor, source, outline, language, depth + 1);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Find line range of symbol
    fn find_symbol_range(
        &self,
        content: &str,
        language: Language,
        symbol_name: &str,
    ) -> ToolExecutorResult<Option<(usize, usize)>> {
        let mut parser = Parser::new();

        // Set language parser (same as extract_code_outline)
        match language {
            Language::Python => {
                parser
                    .set_language(&tree_sitter_python::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set Python language: {e}"),
                    })?;
            }
            Language::TypeScript => {
                parser
                    .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set TypeScript language: {e}"),
                    })?;
            }
            Language::JavaScript => {
                parser
                    .set_language(&tree_sitter_javascript::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set JavaScript language: {e}"),
                    })?;
            }
            Language::Rust => {
                parser
                    .set_language(&tree_sitter_rust::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set Rust language: {e}"),
                    })?;
            }
            Language::Go => {
                parser
                    .set_language(&tree_sitter_go::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set Go language: {e}"),
                    })?;
            }
            Language::Java => {
                parser
                    .set_language(&tree_sitter_java::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set Java language: {e}"),
                    })?;
            }
            Language::C => {
                parser
                    .set_language(&tree_sitter_c::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set C language: {e}"),
                    })?;
            }
            Language::Cpp => {
                parser
                    .set_language(&tree_sitter_cpp::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set C++ language: {e}"),
                    })?;
            }
            Language::CSharp => {
                parser
                    .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set C# language: {e}"),
                    })?;
            }
            Language::Ruby => {
                parser
                    .set_language(&tree_sitter_ruby::LANGUAGE.into())
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "read_file".to_string(),
                        error: format!("Failed to set Ruby language: {e}"),
                    })?;
            }
            _ => {
                return Err(ToolExecutorError::InvalidArguments {
                    tool_name: "read_file".to_string(),
                    error: format!("Language {language:?} not supported for parsing"),
                });
            }
        }

        let tree =
            parser
                .parse(content, None)
                .ok_or_else(|| ToolExecutorError::ExecutionFailed {
                    tool_name: "read_file".to_string(),
                    error: format!("Failed to parse {language:?} code"),
                })?;

        let mut cursor = tree.root_node().walk();
        Ok(self.find_symbol_recursive(&mut cursor, content, symbol_name, language))
    }

    /// Recursively find symbol
    fn find_symbol_recursive(
        &self,
        cursor: &mut TreeCursor,
        source: &str,
        symbol_name: &str,
        language: Language,
    ) -> Option<(usize, usize)> {
        let node = cursor.node();

        // Check if it's the target symbol
        if self.is_target_symbol(node, source, symbol_name, language) {
            return Some((node.start_position().row, node.end_position().row));
        }

        // Recursively search child nodes
        if cursor.goto_first_child() {
            loop {
                if let Some(range) =
                    self.find_symbol_recursive(cursor, source, symbol_name, language)
                {
                    return Some(range);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }

        None
    }

    /// Check if node is target symbol
    fn is_target_symbol(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbol_name: &str,
        language: Language,
    ) -> bool {
        let node_kind = node.kind();

        // Check different node types based on language
        let is_definition = match language {
            Language::Python => {
                matches!(node_kind, "function_definition" | "class_definition")
            }
            Language::TypeScript | Language::JavaScript => {
                matches!(
                    node_kind,
                    "function_declaration"
                        | "class_declaration"
                        | "method_definition"
                        | "arrow_function"
                )
            }
            Language::Rust => {
                matches!(
                    node_kind,
                    "function_item"
                        | "impl_item"
                        | "struct_item"
                        | "enum_item"
                        | "trait_item"
                        | "mod_item"
                )
            }
            Language::Go => {
                matches!(
                    node_kind,
                    "function_declaration" | "method_declaration" | "type_declaration"
                )
            }
            Language::Java | Language::CSharp => {
                matches!(
                    node_kind,
                    "method_declaration" | "class_declaration" | "interface_declaration"
                )
            }
            Language::C | Language::Cpp => {
                matches!(
                    node_kind,
                    "function_definition" | "struct_specifier" | "class_specifier"
                )
            }
            Language::Ruby => {
                matches!(node_kind, "method" | "class" | "module")
            }
            _ => false,
        };

        if !is_definition {
            return false;
        }

        // Extract symbol name and compare - safe slicing
        let Some(text) = source.get(node.start_byte()..node.end_byte()) else {
            return false;
        };
        let Some(first_line) = text.lines().next() else {
            return false;
        };

        // Simple name matching (can be further optimized)
        first_line.contains(symbol_name)
    }
}

#[async_trait]
impl RunnableTool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        r#"Reads a file from the local filesystem. You can access any file directly.

Assume this tool can read all files on the machine. If the user provides a path, assume it is valid. It is okay to read a file that does not exist; an error will be returned.

Usage:
- The path parameter must be an absolute path, not a relative path
- By default, it reads up to 2000 lines starting from the beginning of the file
- You can optionally specify a line offset and limit (especially handy for long files)
- Any lines longer than 2000 characters will be truncated
- Results are returned with line numbers for easy reference when editing
- You can call multiple tools in a single response. It is always better to speculatively read multiple files as a batch that are potentially useful.
- If you read a file that exists but has empty contents, you will receive a warning.
- You can read image files using this tool.

Reading Modes:
- mode="full" (default): Read full file content with optional offset/limit
- mode="outline": Show code structure (classes, functions, methods) - ideal for understanding file organization before editing
- mode="symbol": Read a specific function/class/method by name - gets complete code blocks without truncation

Best Practices:
- Always read a file before editing it. The edit tool will fail if you haven't read the file first.
- For large files, use mode="outline" first to understand structure, then read specific sections.
- When exploring a codebase, batch-read multiple files in parallel.
- Do not re-read files after editing them - the edit tool will fail if the edit didn't work.

Examples:
- Read entire file: {"path": "/path/to/file.js"}
- Get file structure: {"path": "/path/to/file.js", "mode": "outline"}
- Read specific function: {"path": "/path/to/file.js", "mode": "symbol", "symbol": "myFunction"}
- Read with pagination: {"path": "/path/to/file.js", "offset": 100, "limit": 50}"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute path to the file to read. For example: \"/Users/user/project/src/main.ts\". Will return an error if the file doesn't exist."
                },
                "mode": {
                    "type": "string",
                    "enum": ["full", "outline", "symbol"],
                    "description": "Reading mode: 'full' (default) reads file content, 'outline' shows code structure, 'symbol' reads a specific function/class by name."
                },
                "symbol": {
                    "type": "string",
                    "description": "When mode='symbol', specify the symbol name to read (e.g., 'MyClass', 'myFunction', 'MyClass.myMethod'). Required when mode='symbol'."
                },
                "offset": {
                    "type": "number",
                    "minimum": 0,
                    "description": "Only for mode='full': The 0-based offset to start reading from. Leave empty to read from the beginning."
                },
                "limit": {
                    "type": "number",
                    "minimum": 1,
                    "description": "Only for mode='full': The maximum number of lines to read (default: 2000). Leave empty to read the entire file."
                }
            },
            "required": ["path"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileRead, ToolPriority::Standard)
            .with_tags(vec!["filesystem".into(), "read".into()])
            .with_summary_key_arg("path")
    }

    async fn run(
        &self,
        context: &AgentRunContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: ReadFileArgs = serde_json::from_value(args)?;

        let path = match ensure_absolute(&args.path, &context.cwd) {
            Ok(resolved) => resolved,
            Err(err) => return Ok(validation_error(err.to_string())),
        };

        let metadata = match fs::metadata(&path).await {
            Ok(meta) => meta,
            Err(_) => {
                return Ok(validation_error(format!(
                    "File not found: {}",
                    path.display()
                )));
            }
        };

        if metadata.is_dir() {
            return Ok(validation_error(format!(
                "Path {} is a directory, please use list_files tool to view directory contents",
                path.display()
            )));
        }

        if is_probably_binary(&path) {
            return Ok(validation_error(format!(
                "File {} is binary, cannot read as text",
                path.display()
            )));
        }

        let raw_content = match fs::read_to_string(&path).await {
            Ok(content) => content,
            Err(err) => {
                return Ok(tool_error(format!(
                    "Failed to read file {}: {}",
                    path.display(),
                    err
                )));
            }
        };

        // Record file operation
        context
            .file_tracker()
            .track_file_operation(FileOperationRecord::new(
                path.as_path(),
                FileRecordSource::ReadTool,
            ))
            .await?;

        // Record file mtime for checking before subsequent edits
        context
            .file_tracker()
            .record_file_mtime(path.as_path())
            .await?;

        context
            .note_agent_read_snapshot(path.as_path(), &raw_content)
            .await;

        // Process based on mode
        let mode = args.mode.as_deref().unwrap_or("full");
        match mode {
            "outline" => Ok(self.read_outline(&path, &raw_content)),
            "symbol" => {
                if let Some(symbol_name) = args.symbol {
                    Ok(self.read_symbol(&path, &raw_content, &symbol_name))
                } else {
                    Ok(validation_error(
                        "symbol parameter is required when mode='symbol'",
                    ))
                }
            }
            "full" => Ok(self.read_full(&raw_content, args.offset, args.limit)),
            _ => Ok(validation_error(format!(
                "Unsupported mode '{mode}'. Expected one of: full, outline, symbol"
            ))),
        }
    }
}

fn validation_error(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        status: ToolResultStatus::Error,
        cancel_reason: None,
        execution_time_ms: None,
        ext_info: None,
    }
}

fn tool_error(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        status: ToolResultStatus::Error,
        cancel_reason: None,
        execution_time_ms: None,
        ext_info: None,
    }
}
