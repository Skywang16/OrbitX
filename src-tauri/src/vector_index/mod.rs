/*!
 * 代码向量索引系统
 *
 * 基于OrbitX现有的EkoCore AI框架和Tauri+Rust架构，提供代码向量索引和语义搜索功能。
 * 系统采用前后端分离架构：Rust后端负责重计算任务（代码解析、向量化、批量处理），
 * Qdrant作为独立向量数据库，TypeScript前端提供轻量化接口（配置和查询工具）。
 *
 * ## 模块组织
 *
 * - `parser`: 代码解析服务，基于Tree-sitter实现语法感知的代码分块
 * - `vectorizer`: 向量化引擎，集成现有LLM服务进行代码向量化
 * - `qdrant`: Qdrant数据库集成层，负责向量存储和搜索
 * - `commands`: Tauri命令接口，提供前端调用的API
 * - `types`: 通用类型定义和数据结构
 *
 * ## 技术要求
 *
 * - Requirements: 1.1, 2.1 (支持主流编程语言解析和LLM集成)
 * - 遵循OrbitX开发规范，使用anyhow进行错误处理
 * - 重用现有LLM服务和配置管理系统
 */

// 子模块声明
pub mod commands;
pub mod monitor;
pub mod parser;
pub mod qdrant;
pub mod service;
pub mod types;
pub mod vectorizer;

// 重新导出核心类型和接口
pub use types::{CodeVector, IndexStats, SearchOptions, SearchResult, VectorIndexConfig};

// 导出主要服务接口
pub use monitor::FileMonitorService;
pub use parser::CodeParser;
pub use qdrant::QdrantService;
pub use service::VectorIndexService;
pub use vectorizer::VectorizationService;

// Tauri命令导出（用于main.rs注册）
pub use commands::{
    build_code_index, cancel_build_index, clear_vector_index, get_current_workspace_path,
    get_file_monitoring_status, get_vector_index_config, get_vector_index_status,
    init_vector_index, save_vector_index_config, search_code_vectors, start_file_monitoring,
    stop_file_monitoring, test_qdrant_connection,
};

// 版本信息
pub const VERSION: &str = "0.1.0";

// 配置常量
pub mod constants {
    /// 默认向量维度（对应 text-embedding-3-small）
    pub const DEFAULT_VECTOR_SIZE: usize = 1536;

    /// 默认批处理大小
    pub const DEFAULT_BATCH_SIZE: usize = 50;

    /// Qdrant批量上传最大大小
    pub const QDRANT_BATCH_SIZE: usize = 1000;

    /// 默认搜索结果数量
    pub const DEFAULT_SEARCH_RESULTS: usize = 10;

    /// 默认相似度阈值
    pub const DEFAULT_MIN_SCORE: f32 = 0.3;
}
