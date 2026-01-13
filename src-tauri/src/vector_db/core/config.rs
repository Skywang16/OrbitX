use crate::llm::types::LLMProviderConfig;
use serde::{Deserialize, Serialize};

/// 远程向量模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteEmbeddingConfig {
    /// LLM Provider 配置 (API Key, URL 等)
    pub provider_config: LLMProviderConfig,

    /// 模型名称 (如 "text-embedding-3-small")
    pub model_name: String,

    /// 向量维度
    pub dimension: usize,

    /// 分块大小 (token 数量)
    pub chunk_size: usize,

    /// 分块重叠 (token 数量)
    pub chunk_overlap: usize,
}

impl Default for RemoteEmbeddingConfig {
    fn default() -> Self {
        Self {
            provider_config: LLMProviderConfig {
                provider_type: String::new(),
                api_key: String::new(),
                api_url: None,
                options: None,
            },
            model_name: String::new(),
            dimension: 0,
            chunk_size: 512,
            chunk_overlap: 100,
        }
    }
}

/// 向量数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDbConfig {
    /// 远程向量模型配置
    pub embedding: RemoteEmbeddingConfig,

    /// 搜索时返回的最大结果数
    pub max_results: usize,

    /// 相似度阈值
    pub similarity_threshold: f32,

    /// 文件大小限制 (bytes)
    pub max_file_size: u64,

    /// 语义搜索权重 (0.0-1.0)
    pub semantic_weight: f32,

    /// 关键词搜索权重 (0.0-1.0)
    pub keyword_weight: f32,
}

impl Default for VectorDbConfig {
    fn default() -> Self {
        Self {
            embedding: RemoteEmbeddingConfig::default(),
            max_results: 20,
            similarity_threshold: 0.3,
            max_file_size: 10 * 1024 * 1024,
            semantic_weight: 0.7,
            keyword_weight: 0.3,
        }
    }
}

impl VectorDbConfig {
    pub fn validate(&self) -> crate::vector_db::core::Result<()> {
        if self.embedding.model_name.is_empty() {
            return Err(crate::vector_db::core::VectorDbError::Config(
                "Embedding model name is required".to_string(),
            ));
        }
        if self.embedding.provider_config.api_key.is_empty() {
            return Err(crate::vector_db::core::VectorDbError::Config(
                "API key is required".to_string(),
            ));
        }
        if self.embedding.dimension == 0 {
            return Err(crate::vector_db::core::VectorDbError::Config(
                "Dimension must be > 0".to_string(),
            ));
        }
        if self.embedding.chunk_size == 0 {
            return Err(crate::vector_db::core::VectorDbError::Config(
                "Chunk size must be > 0".to_string(),
            ));
        }
        if self.embedding.chunk_overlap >= self.embedding.chunk_size {
            return Err(crate::vector_db::core::VectorDbError::Config(
                "Chunk overlap must be < chunk size".to_string(),
            ));
        }
        if self.similarity_threshold < 0.0 || self.similarity_threshold > 1.0 {
            return Err(crate::vector_db::core::VectorDbError::Config(
                "Similarity threshold must be in [0, 1]".to_string(),
            ));
        }
        Ok(())
    }
}
