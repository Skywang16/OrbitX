use crate::vector_db::core::{ChunkId, ChunkType, Result, Span};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 索引清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexManifest {
    /// 版本号
    pub version: String,

    /// 创建时间 (Unix timestamp)
    pub created_at: u64,

    /// 最后更新时间 (Unix timestamp)
    pub updated_at: u64,

    /// 嵌入模型
    pub embedding_model: String,

    /// 向量维度
    pub vector_dimension: usize,

    /// 文件索引映射 (文件路径 -> 文件哈希)
    pub files: HashMap<PathBuf, String>,

    /// 块索引映射 (块 ID -> 块元数据)
    pub chunks: HashMap<ChunkId, ChunkMetadata>,
}

/// 块元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub file_path: PathBuf,
    pub span: Span,
    pub chunk_type: ChunkType,
    pub hash: String,
}

impl IndexManifest {
    /// 创建新的索引清单
    pub fn new(embedding_model: String, vector_dimension: usize) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version: "1.0.0".to_string(),
            created_at: now,
            updated_at: now,
            embedding_model,
            vector_dimension,
            files: HashMap::new(),
            chunks: HashMap::new(),
        }
    }

    /// 从文件加载清单
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Self = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// 保存清单到文件
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// 添加文件
    pub fn add_file(&mut self, file_path: PathBuf, file_hash: String) {
        self.files.insert(file_path, file_hash);
        self.update_timestamp();
    }

    /// 删除文件
    pub fn remove_file(&mut self, file_path: &Path) {
        self.files.remove(file_path);
        // 删除该文件的所有块
        self.chunks
            .retain(|_, metadata| metadata.file_path != file_path);
        self.update_timestamp();
    }

    /// 添加块
    pub fn add_chunk(&mut self, chunk_id: ChunkId, metadata: ChunkMetadata) {
        self.chunks.insert(chunk_id, metadata);
        self.update_timestamp();
    }

    /// 删除块
    pub fn remove_chunk(&mut self, chunk_id: &ChunkId) {
        self.chunks.remove(chunk_id);
        self.update_timestamp();
    }

    /// 更新时间戳
    fn update_timestamp(&mut self) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// 获取文件的所有块
    pub fn get_file_chunks(&self, file_path: &Path) -> Vec<(ChunkId, &ChunkMetadata)> {
        self.chunks
            .iter()
            .filter(|(_, metadata)| metadata.file_path == file_path)
            .map(|(id, metadata)| (*id, metadata))
            .collect()
    }
}
