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
            "php" | "phtml" | "php3" | "php4" | "php5" | "phps" | "phar" => Some(Language::Php),
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

/// Stride 信息 - 用于记录大 chunk 拆分后的信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrideInfo {
    /// 原始 chunk 的唯一 ID
    pub original_chunk_id: String,
    /// 当前 stride 的索引（从 0 开始）
    pub stride_index: usize,
    /// 总 stride 数量
    pub total_strides: usize,
    /// 与前一个 stride 重叠的起始字节偏移
    pub overlap_start: usize,
    /// 与后一个 stride 重叠的结束字节偏移
    pub overlap_end: usize,
}

/// Chunk 配置
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// 每个 chunk 的最大 token 数
    pub max_tokens: usize,
    /// stride 重叠的 token 数
    pub stride_overlap: usize,
    /// 是否启用 striding（大 chunk 拆分）
    pub enable_striding: bool,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_tokens: 8192,     // 默认使用大模型限制
            stride_overlap: 1024, // 12.5% 重叠
            enable_striding: true,
        }
    }
}

impl ChunkConfig {
    /// 根据模型名称创建配置
    pub fn for_model(model_name: Option<&str>) -> Self {
        let (max_tokens, stride_overlap) =
            crate::vector_db::chunking::TokenEstimator::get_model_chunk_config(model_name);
        Self {
            max_tokens,
            stride_overlap,
            enable_striding: true,
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
    /// Stride 信息（如果这个 chunk 是从大 chunk 拆分出来的）
    pub stride_info: Option<StrideInfo>,
}

impl Chunk {
    /// 创建新的文本块
    pub fn new(file_path: PathBuf, span: Span, content: String, chunk_type: ChunkType) -> Self {
        Self {
            id: Uuid::new_v4(),
            file_path,
            span,
            content,
            chunk_type,
            stride_info: None,
        }
    }

    /// 创建带 stride 信息的文本块
    pub fn with_stride(
        file_path: PathBuf,
        span: Span,
        content: String,
        chunk_type: ChunkType,
        stride_info: StrideInfo,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            file_path,
            span,
            content,
            chunk_type,
            stride_info: Some(stride_info),
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
