use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// 文件语言类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Ruby,
    Php,
    Swift,
    Kotlin,
}

impl Language {
    /// 从文件扩展名推断语言
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(Language::Rust),
            "ts" | "tsx" => Some(Language::TypeScript),
            "js" | "jsx" => Some(Language::JavaScript),
            "py" => Some(Language::Python),
            "go" => Some(Language::Go),
            "java" => Some(Language::Java),
            "c" => Some(Language::C),
            "cpp" | "cc" | "cxx" | "c++" => Some(Language::Cpp),
            "cs" => Some(Language::CSharp),
            "rb" => Some(Language::Ruby),
            "php" => Some(Language::Php),
            "swift" => Some(Language::Swift),
            "kt" | "kts" => Some(Language::Kotlin),
            _ => None,
        }
    }

    /// 从文件路径推断语言
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }
}

/// 文本片段位置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
}

impl Span {
    /// 创建新的 Span
    pub fn new(byte_start: usize, byte_end: usize, line_start: usize, line_end: usize) -> Self {
        Self {
            byte_start,
            byte_end,
            line_start,
            line_end,
        }
    }

    /// 验证 Span 的有效性
    pub fn validate(&self) -> crate::vector_db::core::Result<()> {
        if self.byte_start > self.byte_end {
            return Err(crate::vector_db::core::VectorDbError::InvalidSpan(format!(
                "Invalid byte range: {} > {}",
                self.byte_start, self.byte_end
            )));
        }
        if self.line_start > self.line_end {
            return Err(crate::vector_db::core::VectorDbError::InvalidSpan(format!(
                "Invalid line range: {} > {}",
                self.line_start, self.line_end
            )));
        }
        Ok(())
    }

    /// 获取字节长度
    pub fn byte_len(&self) -> usize {
        self.byte_end.saturating_sub(self.byte_start)
    }

    /// 获取行数
    pub fn line_count(&self) -> usize {
        self.line_end.saturating_sub(self.line_start) + 1
    }
}

/// 文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub hash: String,
    pub last_modified: u64,
    pub size: u64,
    pub language: Option<Language>,
}

impl FileMetadata {
    /// 创建文件元数据
    pub fn new(path: PathBuf, hash: String, last_modified: u64, size: u64) -> Self {
        let language = Language::from_path(&path);
        Self {
            path,
            hash,
            last_modified,
            size,
            language,
        }
    }
}

/// 向量块 ID
pub type ChunkId = Uuid;

/// 块类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChunkType {
    Function,
    Class,
    Method,
    Struct,
    Enum,
    Generic,
}

impl std::fmt::Display for ChunkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkType::Function => write!(f, "function"),
            ChunkType::Class => write!(f, "class"),
            ChunkType::Method => write!(f, "method"),
            ChunkType::Struct => write!(f, "struct"),
            ChunkType::Enum => write!(f, "enum"),
            ChunkType::Generic => write!(f, "generic"),
        }
    }
}

/// 文本块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: ChunkId,
    pub file_path: PathBuf,
    pub span: Span,
    pub content: String,
    pub chunk_type: ChunkType,
}

impl Chunk {
    /// 创建新的文本块
    pub fn new(
        file_path: PathBuf,
        span: Span,
        content: String,
        chunk_type: ChunkType,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            file_path,
            span,
            content,
            chunk_type,
        }
    }
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file_path: PathBuf,
    pub span: Span,
    pub score: f32,
    pub preview: String,
    pub language: Option<Language>,
    pub chunk_type: Option<ChunkType>,
}

impl SearchResult {
    /// 创建搜索结果
    pub fn new(
        file_path: PathBuf,
        span: Span,
        score: f32,
        preview: String,
        language: Option<Language>,
        chunk_type: Option<ChunkType>,
    ) -> Self {
        Self {
            file_path,
            span,
            score,
            preview,
            language,
            chunk_type,
        }
    }
}
