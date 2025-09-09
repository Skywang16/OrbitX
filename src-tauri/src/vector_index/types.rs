/*!
 * 向量索引系统类型定义
 *
 * 包含向量索引系统所需的所有数据结构和类型定义。
 * 遵循Requirement 1.1和2.1的要求，支持多种编程语言和LLM集成。
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// use std::time::Duration; // removed: using milliseconds (u64) instead

/// 向量索引服务配置
///
/// 简化的配置结构，只包含用户需要配置的核心参数。
/// 技术细节参数（如批处理大小、块大小等）由程序内部自动优化。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct VectorIndexConfig {
    /// Qdrant数据库URL
    pub qdrant_url: String,

    /// Qdrant API密钥（可选）
    pub qdrant_api_key: Option<String>,

    /// 向量集合名称
    pub collection_name: String,

    /// 关联的embedding模型ID
    pub embedding_model_id: String,

    /// 最大并发文件处理数量（性能调优参数）
    #[serde(default = "default_max_concurrent_files")]
    pub max_concurrent_files: usize,
}

/// 内部配置参数
///
/// 这些参数由程序内部控制，不暴露给用户配置界面。
/// 根据模型类型、项目特征等自动优化。
#[derive(Debug, Clone)]
pub struct VectorIndexInternalConfig {
    /// 向量维度大小（根据模型自动推断）
    pub vector_size: usize,

    /// 批处理大小（根据性能自动调整）
    pub batch_size: usize,

    /// 支持的文件扩展名（根据项目类型自动检测）
    pub supported_extensions: Vec<String>,

    /// 忽略的文件模式（遵循.gitignore规则）
    pub ignore_patterns: Vec<String>,

    /// 代码块大小范围（根据语言特征优化）
    pub chunk_size_range: [usize; 2],
}

/// 完整的向量索引配置（用户配置 + 内部配置）
#[derive(Debug, Clone)]
pub struct VectorIndexFullConfig {
    /// 用户配置部分
    pub user_config: VectorIndexConfig,
    /// 内部配置部分
    pub internal_config: VectorIndexInternalConfig,
}

// 默认值函数
fn default_max_concurrent_files() -> usize {
    4
}

impl Default for VectorIndexConfig {
    fn default() -> Self {
        Self {
            qdrant_url: "http://localhost:6334".to_string(),
            qdrant_api_key: None,
            collection_name: "orbitx-code-vectors".to_string(),
            embedding_model_id: "text-embedding-3-small".to_string(),
            max_concurrent_files: default_max_concurrent_files(),
        }
    }
}

impl Default for VectorIndexInternalConfig {
    fn default() -> Self {
        Self {
            vector_size: crate::vector_index::constants::DEFAULT_VECTOR_SIZE,
            batch_size: crate::vector_index::constants::DEFAULT_BATCH_SIZE,
            supported_extensions: vec![
                ".ts".to_string(),
                ".tsx".to_string(),
                ".js".to_string(),
                ".jsx".to_string(),
                ".rs".to_string(),
                ".py".to_string(),
                ".go".to_string(),
                ".java".to_string(),
                ".c".to_string(),
                ".cpp".to_string(),
                ".h".to_string(),
                ".hpp".to_string(),
            ],
            ignore_patterns: vec![
                "**/node_modules/**".to_string(),
                "**/target/**".to_string(),
                "**/dist/**".to_string(),
                "**/.git/**".to_string(),
                "**/build/**".to_string(),
                "**/.vscode/**".to_string(),
                "**/.idea/**".to_string(),
            ],
            chunk_size_range: [10, 2000],
        }
    }
}

impl VectorIndexFullConfig {
    /// 创建新的完整配置
    pub fn new(user_config: VectorIndexConfig) -> Self {
        Self {
            user_config,
            internal_config: VectorIndexInternalConfig::default(),
        }
    }

    /// 根据用户配置和模型信息创建优化的完整配置
    pub fn from_user_config_with_model_info(
        user_config: VectorIndexConfig,
        model_name: &str,
    ) -> Self {
        let mut internal_config = VectorIndexInternalConfig::default();

        // 根据模型名称推断向量维度
        internal_config.vector_size = infer_vector_size_from_model(model_name);

        Self {
            user_config,
            internal_config,
        }
    }

    /// 根据用户配置和实际检测的维度创建完整配置
    pub fn from_user_config_with_actual_dimension(
        user_config: VectorIndexConfig,
        actual_dimension: usize,
    ) -> Self {
        let mut internal_config = VectorIndexInternalConfig::default();

        // 使用实际检测到的维度
        internal_config.vector_size = actual_dimension;

        Self {
            user_config,
            internal_config,
        }
    }

    /// 获取向量维度大小
    pub fn vector_size(&self) -> usize {
        self.internal_config.vector_size
    }

    /// 获取批处理大小
    pub fn batch_size(&self) -> usize {
        self.internal_config.batch_size
    }

    /// 获取支持的文件扩展名
    pub fn supported_extensions(&self) -> &[String] {
        &self.internal_config.supported_extensions
    }

    /// 获取忽略模式
    pub fn ignore_patterns(&self) -> &[String] {
        &self.internal_config.ignore_patterns
    }

    /// 获取代码块大小范围
    pub fn chunk_size_range(&self) -> [usize; 2] {
        self.internal_config.chunk_size_range
    }
}

/// 根据模型名称推断向量维度（仅作为备用，推荐使用实际检测）
pub fn infer_vector_size_from_model(model_name: &str) -> usize {
    let model_lower = model_name.to_lowercase();

    // OpenAI 模型
    if model_lower.contains("text-embedding-3-large") {
        3072
    } else if model_lower.contains("text-embedding-3-small")
        || model_lower.contains("text-embedding-ada-002")
    {
        1536
    }
    // 豆包/字节跳动模型
    else if model_lower.contains("doubao-embedding-large") 
        || model_lower.contains("embedding-large-text") 
    {
        2048
    } else if model_lower.contains("doubao-embedding") {
        1536
    }
    // 其他常见模型维度
    else if model_lower.contains("bge-large") {
        1024
    } else if model_lower.contains("bge-base") {
        768
    } else if model_lower.contains("gte-large") {
        1024
    } else if model_lower.contains("m3e-large") {
        1024
    } else {
        // 默认维度
        1536
    }
}

/// 代码向量表示
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeVector {
    /// 唯一标识符
    pub id: String,

    /// 文件路径
    pub file_path: String,

    /// 代码内容
    pub content: String,

    /// 起始行号
    pub start_line: u32,

    /// 结束行号
    pub end_line: u32,

    /// 编程语言
    pub language: String,

    /// 代码块类型（function, class, method等）
    pub chunk_type: String,

    /// 向量表示
    pub vector: Vec<f32>,

    /// 额外元数据
    pub metadata: HashMap<String, String>,
}

/// 代码搜索选项
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOptions {
    /// 查询文本
    pub query: String,

    /// 最大结果数量
    pub max_results: Option<usize>,

    /// 最小相似度分数
    pub min_score: Option<f32>,

    /// 目录过滤（仅搜索指定目录）
    pub directory_filter: Option<String>,

    /// 语言过滤（仅搜索指定编程语言）
    pub language_filter: Option<String>,

    /// 代码块类型过滤
    pub chunk_type_filter: Option<String>,

    /// 内容长度过滤范围
    pub min_content_length: Option<usize>,
    pub max_content_length: Option<usize>,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    /// 向量ID
    pub id: String,

    /// 文件路径
    pub file_path: String,

    /// 代码内容
    pub content: String,

    /// 起始行号
    pub start_line: u32,

    /// 结束行号
    pub end_line: u32,

    /// 编程语言
    pub language: String,

    /// 代码块类型
    pub chunk_type: String,

    /// 相似度分数 (0.0 - 1.0)
    pub score: f32,

    /// 额外元数据
    pub metadata: HashMap<String, String>,
}

/// 索引构建统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStats {
    /// 处理的文件总数
    pub total_files: usize,

    /// 生成的代码块总数
    pub total_chunks: usize,

    /// 成功向量化的代码块数
    pub vectorized_chunks: usize,

    /// 上传到Qdrant的向量数
    pub uploaded_vectors: usize,

    /// 处理耗时（毫秒）
    #[serde(rename = "processingTime")]
    pub processing_time: u64,

    /// 失败的文件列表
    pub failed_files: Vec<String>,

    /// 错误信息
    pub errors: Vec<String>,
}

/// 代码解析结果
#[derive(Debug, Clone)]
pub struct ParsedCode {
    /// 文件路径
    pub file_path: String,

    /// 代码块列表
    pub chunks: Vec<CodeChunk>,

    /// 解析错误（如果有）
    pub errors: Vec<String>,
}

/// 代码块
#[derive(Debug, Clone)]
pub struct CodeChunk {
    /// 代码内容
    pub content: String,

    /// 起始行号
    pub start_line: u32,

    /// 结束行号
    pub end_line: u32,

    /// 代码块类型
    pub chunk_type: ChunkType,

    /// 额外元数据
    pub metadata: HashMap<String, String>,
}

/// 代码块类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkType {
    /// 函数
    Function,
    /// 方法
    Method,
    /// 类
    Class,
    /// 结构体
    Struct,
    /// 接口
    Interface,
    /// 枚举
    Enum,
    /// 模块
    Module,
    /// 注释块
    Comment,
    /// 导入/使用声明
    Import,
    /// 其他代码块
    Other,
}

impl ChunkType {
    /// 转换为字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            ChunkType::Function => "function",
            ChunkType::Method => "method",
            ChunkType::Class => "class",
            ChunkType::Struct => "struct",
            ChunkType::Interface => "interface",
            ChunkType::Enum => "enum",
            ChunkType::Module => "module",
            ChunkType::Comment => "comment",
            ChunkType::Import => "import",
            ChunkType::Other => "other",
        }
    }
}

/// 支持的编程语言
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    TypeScript,
    JavaScript,
    Rust,
    Python,
    Go,
    Java,
    C,
    Cpp,
}

impl Language {
    /// 从文件扩展名推断语言
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "ts" | "tsx" => Some(Language::TypeScript),
            "js" | "jsx" => Some(Language::JavaScript),
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "go" => Some(Language::Go),
            "java" => Some(Language::Java),
            "c" | "h" => Some(Language::C),
            "cpp" | "cc" | "cxx" | "hpp" => Some(Language::Cpp),
            _ => None,
        }
    }

    /// 获取语言的字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Rust => "rust",
            Language::Python => "python",
            Language::Go => "go",
            Language::Java => "java",
            Language::C => "c",
            Language::Cpp => "cpp",
        }
    }
}

/// 向量索引服务状态
#[derive(Debug)]
pub enum IndexServiceState {
    /// 未初始化
    Uninitialized,
    /// 正在初始化
    Initializing,
    /// 已就绪
    Ready,
    /// 正在构建索引
    Building,
    /// 错误状态
    Error(String),
}

/// 任务进度信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    /// 任务ID
    pub task_id: String,

    /// 当前进度（0.0 - 1.0）
    pub progress: f32,

    /// 状态描述
    pub status: String,

    /// 当前处理的文件
    pub current_file: Option<String>,

    /// 已处理文件数
    pub processed_files: usize,

    /// 总文件数
    pub total_files: usize,

    /// 是否可取消
    pub cancellable: bool,
}

/// 向量索引整体状态（供前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorIndexStatus {
    /// 是否已初始化
    pub is_initialized: bool,
    /// 向量总数
    pub total_vectors: usize,
    /// 集合名称（可选）
    pub collection_name: Option<String>,
    /// 最近更新时间（ISO8601，或null）
    pub last_updated: Option<String>,
}
