use crate::vector_db::core::{ChunkId, FileMetadata, Result, VectorDbError};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 单个文件的向量数据
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FileVectors {
    /// chunk_id -> 向量
    pub chunks: HashMap<ChunkId, Vec<f32>>,
}

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
    /// 项目根目录
    project_root: PathBuf,
}

impl FileStore {
    /// 创建新的文件存储
    pub fn new(project_root: &Path) -> Result<Self> {
        let root_path = project_root.join(".oxi");
        let vectors_path = root_path.join("vectors");
        let metadata_path = root_path.join("metadata");
        let cache_path = root_path.join("cache");

        Ok(Self {
            root_path,
            vectors_path,
            metadata_path,
            cache_path,
            project_root: project_root.to_path_buf(),
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

    /// 获取文件对应的向量文件路径
    fn get_vector_file_path(&self, source_file: &Path) -> PathBuf {
        // 将源文件路径转换为相对于项目根目录的路径
        let relative_path = source_file
            .strip_prefix(&self.project_root)
            .unwrap_or(source_file);

        // 在 vectors 目录下创建对应的目录结构
        let vector_dir = self
            .vectors_path
            .join(relative_path.parent().unwrap_or_else(|| Path::new("")));

        // 使用源文件名 + .oxi 后缀
        let file_name = format!(
            "{}.oxi",
            relative_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
        );

        vector_dir.join(file_name)
    }

    /// 保存单个文件的所有向量数据
    pub fn save_file_vectors(
        &self,
        source_file: &Path,
        chunks: &[(ChunkId, Vec<f32>)],
    ) -> Result<()> {
        let vector_file = self.get_vector_file_path(source_file);

        // 确保目录存在
        if let Some(parent) = vector_file.parent() {
            fs::create_dir_all(parent)?;
        }

        // 构建向量数据
        let file_vectors = FileVectors {
            chunks: chunks.iter().cloned().collect(),
        };

        // 序列化并保存
        let data = bincode::serialize(&file_vectors)?;
        fs::write(&vector_file, data)?;

        Ok(())
    }

    /// 加载单个文件的所有向量数据
    pub fn load_file_vectors(&self, source_file: &Path) -> Result<FileVectors> {
        let vector_file = self.get_vector_file_path(source_file);

        if !vector_file.exists() {
            return Err(VectorDbError::FileNotFound(format!(
                "Vector file not found: {}",
                vector_file.display()
            )));
        }

        let data = fs::read(&vector_file)?;
        let file_vectors: FileVectors = bincode::deserialize(&data)?;
        Ok(file_vectors)
    }

    /// 删除文件的向量数据
    pub fn delete_file_vectors(&self, source_file: &Path) -> Result<()> {
        let vector_file = self.get_vector_file_path(source_file);
        if vector_file.exists() {
            fs::remove_file(&vector_file)?;
        }
        Ok(())
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
        // 删除向量文件
        self.delete_file_vectors(file_path)?;

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

    /// 获取存储根目录
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// 获取向量目录
    pub fn vectors_path(&self) -> &Path {
        &self.vectors_path
    }
}
