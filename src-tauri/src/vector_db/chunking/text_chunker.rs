use crate::vector_db::core::{Chunk, ChunkType, Language, Result, Span};
use super::TreeSitterChunker;
use std::path::Path;

pub struct TextChunker {
    chunk_size: usize,
    tree_sitter_chunker: TreeSitterChunker,
}

impl TextChunker {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            tree_sitter_chunker: TreeSitterChunker::new(chunk_size),
        }
    }
    
    pub fn chunk(&self, content: &str, file_path: &Path) -> Result<Vec<Chunk>> {
        // 尝试使用 tree-sitter 智能分块
        if let Some(language) = Language::from_path(file_path) {
            // 对支持的语言使用 tree-sitter
            if matches!(language, 
                Language::Python | Language::TypeScript | Language::JavaScript | 
                Language::Rust | Language::Go | Language::Java | 
                Language::C | Language::Cpp | Language::CSharp | Language::Ruby
            ) {
                tracing::debug!("Using tree-sitter chunking for {:?}", language);
                if let Ok(chunks) = self.tree_sitter_chunker.chunk(content, file_path, language) {
                    if !chunks.is_empty() {
                        return Ok(chunks);
                    }
                }
                // 如果 tree-sitter 失败，回退到简单分块
                tracing::warn!("Tree-sitter failed, fallback to simple chunking");
            }
        }
        
        // 回退到简单分块
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut current_chunk = String::new();
        let mut start_line = 1;
        let mut start_byte = 0;
        
        for (line_num, line) in lines.iter().enumerate() {
            if current_chunk.len() + line.len() > self.chunk_size && !current_chunk.is_empty() {
                let end_byte = start_byte + current_chunk.len();
                let span = Span::new(start_byte, end_byte, start_line, line_num);
                
                chunks.push(Chunk::new(
                    file_path.to_path_buf(),
                    span,
                    current_chunk.clone(),
                    ChunkType::Generic,
                    ));
                
                current_chunk.clear();
                start_line = line_num + 2;
                start_byte = end_byte;
            }
            
            current_chunk.push_str(line);
            current_chunk.push('\n');
        }
        
        if !current_chunk.is_empty() {
            let end_byte = start_byte + current_chunk.len();
            let span = Span::new(start_byte, end_byte, start_line, lines.len());
            chunks.push(Chunk::new(
                file_path.to_path_buf(),
                span,
                current_chunk,
                ChunkType::Generic,
                ));
        }
        
        Ok(chunks)
    }
}
