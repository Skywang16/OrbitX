use std::path::Path;

use serde::Serialize;
use tree_sitter::{Node, Parser, Point, TreeCursor};

use crate::code_intel::tree_sitter::{configure_parser_for_language, TreeSitterError};
use crate::vector_db::core::Language;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct DiagnosticPosition {
    pub line: usize,
    pub column: usize,
}

impl From<Point> for DiagnosticPosition {
    fn from(p: Point) -> Self {
        Self {
            line: p.row + 1,
            column: p.column + 1,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct DiagnosticRange {
    pub start: DiagnosticPosition,
    pub end: DiagnosticPosition,
}

#[derive(Debug, Clone, Serialize)]
pub struct TreeSitterDiagnostic {
    pub file: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub range: DiagnosticRange,
    pub source: &'static str,
}

#[derive(Debug, thiserror::Error)]
pub enum DiagnosticsError {
    #[error("unsupported language for {file}: {language:?}")]
    UnsupportedLanguage { file: String, language: Language },
    #[error("tree-sitter error for {file}: {error}")]
    TreeSitter { file: String, error: TreeSitterError },
    #[error("failed to parse {file}")]
    ParseFailed { file: String },
}

pub fn diagnose_syntax(
    file_path: &Path,
    content: &str,
    language: Language,
) -> Result<Vec<TreeSitterDiagnostic>, DiagnosticsError> {
    let mut parser = Parser::new();
    configure_parser_for_language(&mut parser, file_path, language).map_err(|error| match error {
        TreeSitterError::UnsupportedLanguage(language) => DiagnosticsError::UnsupportedLanguage {
            file: file_path.display().to_string(),
            language,
        },
        error => DiagnosticsError::TreeSitter {
            file: file_path.display().to_string(),
            error,
        },
    })?;

    let tree = parser
        .parse(content, None)
        .ok_or_else(|| DiagnosticsError::ParseFailed {
            file: file_path.display().to_string(),
        })?;

    let root = tree.root_node();
    if !root.has_error() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    let mut cursor = root.walk();
    collect_error_nodes(file_path, &mut cursor, &mut out);
    Ok(out)
}

fn collect_error_nodes(path: &Path, cursor: &mut TreeCursor<'_>, out: &mut Vec<TreeSitterDiagnostic>) {
    loop {
        let node = cursor.node();
        maybe_push_diagnostic(path, node, out);

        if cursor.goto_first_child() {
            collect_error_nodes(path, cursor, out);
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

fn maybe_push_diagnostic(path: &Path, node: Node<'_>, out: &mut Vec<TreeSitterDiagnostic>) {
    if !(node.is_error() || node.is_missing()) {
        return;
    }

    let message = if node.is_missing() {
        format!("Missing {}", node.kind())
    } else {
        "Syntax error".to_string()
    };

    out.push(TreeSitterDiagnostic {
        file: path.display().to_string(),
        severity: DiagnosticSeverity::Error,
        message,
        range: DiagnosticRange {
            start: node.start_position().into(),
            end: node.end_position().into(),
        },
        source: "tree-sitter",
    });
}

