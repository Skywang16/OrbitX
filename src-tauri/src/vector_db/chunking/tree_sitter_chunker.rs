use crate::code_intel::tree_sitter::configure_parser_for_language;
use crate::vector_db::core::{Chunk, ChunkType, Language, Result, Span, VectorDbError};
use std::path::Path;
use tree_sitter::{Parser, TreeCursor};

/// Tree-sitter 智能分块器
pub struct TreeSitterChunker {
    _chunk_size: usize,
}

impl TreeSitterChunker {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            _chunk_size: chunk_size,
        }
    }

    /// 使用 tree-sitter 按语法结构分块
    pub fn chunk(&self, content: &str, file_path: &Path, language: Language) -> Result<Vec<Chunk>> {
        let mut parser = Parser::new();

        // 设置语言解析器
        configure_parser_for_language(&mut parser, file_path, language).map_err(|e| {
            VectorDbError::ChunkingError(format!("Failed to configure tree-sitter language: {}", e))
        })?;

        // 解析代码
        let tree = parser.parse(content, None).ok_or_else(|| {
            VectorDbError::ChunkingError(format!("Failed to parse {:?} code", language))
        })?;

        let mut chunks = Vec::new();
        let mut cursor = tree.root_node().walk();

        self.extract_code_chunks(&mut cursor, content, &mut chunks, file_path, language);

        // 如果没有提取到任何块，返回整个文件作为一个块
        if chunks.is_empty() {
            chunks.push(Chunk::new(
                file_path.to_path_buf(),
                Span::new(0, content.len(), 1, content.lines().count()),
                content.to_string(),
                ChunkType::Generic,
            ));
        }

        Ok(chunks)
    }

    /// 递归提取代码块
    fn extract_code_chunks(
        &self,
        cursor: &mut TreeCursor,
        source: &str,
        chunks: &mut Vec<Chunk>,
        file_path: &Path,
        language: Language,
    ) {
        let node = cursor.node();
        let node_kind = node.kind();

        // 根据语言判断是否为有意义的代码块
        let is_chunk = match language {
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
            Language::Java => {
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
            Language::CSharp => {
                matches!(
                    node_kind,
                    "method_declaration" | "class_declaration" | "interface_declaration"
                )
            }
            Language::Ruby => {
                matches!(node_kind, "method" | "class" | "module")
            }
            _ => false,
        };

        if is_chunk {
            let start_byte = node.start_byte();
            let end_byte = node.end_byte();
            let start_pos = node.start_position();
            let end_pos = node.end_position();

            let text = &source[start_byte..end_byte];

            // 确定块类型
            let chunk_type = match node_kind {
                "function_definition"
                | "function_declaration"
                | "arrow_function"
                | "function_item" => ChunkType::Function,
                "class_definition" | "class_declaration" | "struct_item" | "enum_item"
                | "class_specifier" | "class" => ChunkType::Class,
                "method_definition" | "method_declaration" | "method" => ChunkType::Method,
                "impl_item" | "trait_item" | "mod_item" | "module" | "interface_declaration" => {
                    ChunkType::Struct
                }
                _ => ChunkType::Generic,
            };

            chunks.push(Chunk::new(
                file_path.to_path_buf(),
                Span::new(start_byte, end_byte, start_pos.row + 1, end_pos.row + 1),
                text.to_string(),
                chunk_type,
            ));
        }

        // 递归处理子节点
        if cursor.goto_first_child() {
            loop {
                self.extract_code_chunks(cursor, source, chunks, file_path, language);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_chunking() {
        let code = r#"
def hello():
    print("Hello")

class MyClass:
    def method(self):
        pass
"#;

        let chunker = TreeSitterChunker::new(512);
        let chunks = chunker
            .chunk(code, Path::new("test.py"), Language::Python)
            .unwrap();

        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].chunk_type, ChunkType::Function);
        assert_eq!(chunks[1].chunk_type, ChunkType::Class);
    }

    #[test]
    fn test_rust_chunking() {
        let code = r#"
fn main() {
    println!("Hello");
}

struct MyStruct {
    field: i32,
}

impl MyStruct {
    fn new() -> Self {
        Self { field: 0 }
    }
}
"#;

        let chunker = TreeSitterChunker::new(512);
        let chunks = chunker
            .chunk(code, Path::new("test.rs"), Language::Rust)
            .unwrap();

        assert!(chunks.len() >= 3);
    }
}
