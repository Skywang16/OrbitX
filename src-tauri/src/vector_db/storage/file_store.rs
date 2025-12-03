use crate::vector_db::core::{ChunkId, FileMetadata, Result, VectorDbError};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 文件系统存储管理器
pub struct FileStore {
    /// 索引根目录
    root_path: PathBuf,
    /// 向量数据目录
    vectors_path: PathBuf,
    /// 元数据目录
    metadata_path: PathBuf,
    /// 缓存目录
    cache_path: PathBuf,
}

impl FileStore {
    /// 创建新的文件存储
    pub fn new(project_root: &Path) -> Result<Self> {
        let root_path = project_root.join(".oxi");
        let vectors_path = root_path.clone();
        let metadata_path = root_path.join("metadata");
        let cache_path = root_path.join("cache");

        Ok(Self {
            root_path,
            vectors_path,
            metadata_path,
            cache_path,
        })
    }

    /// 初始化存储目录结构
    pub fn initialize(&self) -> Result<()> {
        fs::create_dir_all(&self.root_path)?;
        fs::create_dir_all(&self.vectors_path)?;
        fs::create_dir_all(&self.metadata_path)?;
        fs::create_dir_all(&self.cache_path)?;
        Ok(())
    }

    /// 保存向量数据
    pub fn save_vectors(&self, chunk_id: ChunkId, vectors: &[f32]) -> Result<()> {
        let file_path = self.vectors_path.join(format!("{}.oxi", chunk_id));
        let data = bincode::serialize(vectors)?;
        fs::write(file_path, data)?;
        Ok(())
    }

    /// 加载向量数据
    pub fn load_vectors(&self, chunk_id: ChunkId) -> Result<Vec<f32>> {
        let file_path = self.vectors_path.join(format!("{}.oxi", chunk_id));
        if !file_path.exists() {
            return Err(VectorDbError::FileNotFound(format!(
                "Vector file not found: {}",
                file_path.display()
            )));
        }
        let data = fs::read(file_path)?;
        let vectors: Vec<f32> = bincode::deserialize(&data)?;
        Ok(vectors)
    }

    /// 保存文件元数据
    pub fn save_file_metadata(&self, metadata: &FileMetadata) -> Result<()> {
        let mut all_metadata = self.load_all_file_metadata().unwrap_or_default();
        all_metadata.insert(metadata.path.clone(), metadata.clone());

        let file_path = self.metadata_path.join("files.json");
        let json = serde_json::to_string_pretty(&all_metadata)?;
        fs::write(file_path, json)?;
        Ok(())
    }

    /// 加载所有文件元数据
    pub fn load_all_file_metadata(&self) -> Result<HashMap<PathBuf, FileMetadata>> {
        let file_path = self.metadata_path.join("files.json");
        if !file_path.exists() {
            return Ok(HashMap::new());
        }
        let content = fs::read_to_string(file_path)?;
        let metadata: HashMap<PathBuf, FileMetadata> = serde_json::from_str(&content)?;
        Ok(metadata)
    }

    /// 删除文件相关数据
    pub fn delete_file_data(&self, file_path: &Path) -> Result<()> {
        // 删除元数据
        let mut all_metadata = self.load_all_file_metadata().unwrap_or_default();
        all_metadata.remove(file_path);

        let metadata_file = self.metadata_path.join("files.json");
        let json = serde_json::to_string_pretty(&all_metadata)?;
        fs::write(metadata_file, json)?;

        Ok(())
    }

    /// 清理过期数据
    pub fn cleanup(&self) -> Result<()> {
        // 实现清理逻辑
        // 1. 检查孤立的向量文件
        // 2. 删除不再引用的缓存
        Ok(())
    }

    /// 删除某个向量文件
    pub fn delete_vectors(&self, chunk_id: ChunkId) -> Result<()> {
        let file_path = self.vectors_path.join(format!("{}.oxi", chunk_id));
        if file_path.exists() {
            std::fs::remove_file(file_path)?;
        }
        Ok(())
    }

    /// 获取存储根目录
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }
}
