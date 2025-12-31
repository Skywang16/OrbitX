use super::{ChunkMetadata, FileStore, IndexManifest};
use crate::vector_db::chunking::TextChunker;
use crate::vector_db::core::{Chunk, Result, VectorDbConfig, VectorDbError};
use crate::vector_db::embedding::Embedder;
use crate::vector_db::index::VectorIndex;
use crate::vector_db::utils::{blake3_hash_bytes, collect_source_files};
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct IndexManager {
    pub(crate) store: Arc<FileStore>,
    pub(crate) manifest: Arc<RwLock<IndexManifest>>,
    pub(crate) config: VectorDbConfig,
}

impl IndexManager {
    pub fn new(project_root: &Path, config: VectorDbConfig) -> Result<Self> {
        let store = Arc::new(FileStore::new(project_root)?);
        store.initialize()?;

        let manifest_path = store.root_path().join("manifest.json");
        let manifest = if manifest_path.exists() {
            IndexManifest::load(&manifest_path)?
        } else {
            IndexManifest::new(
                config.embedding.model_name.clone(),
                config.embedding.dimension,
            )
        };

        Ok(Self {
            store,
            manifest: Arc::new(RwLock::new(manifest)),
            config,
        })
    }

    fn manifest_path(&self) -> PathBuf {
        self.store.root_path().join("manifest.json")
    }

    fn save_manifest(&self) -> Result<()> {
        let manifest = self.manifest.read().clone();
        manifest.save(&self.manifest_path())
    }

    pub async fn index_file_with(
        &self,
        file_path: &Path,
        embedder: &dyn Embedder,
        vector_index: &VectorIndex,
    ) -> Result<()> {
        // 0. 限制：尺寸
        let meta = std::fs::metadata(file_path).map_err(VectorDbError::Io)?;
        if meta.len() > self.config.max_file_size {
            return Ok(()); // 跳过过大文件
        }

        // 1. 读取内容
        let content = std::fs::read_to_string(file_path)?;
        let file_hash = blake3_hash_bytes(content.as_bytes());
        let _language = crate::vector_db::core::Language::from_path(file_path);
        let last_modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // 2. 旧块清理（如果已存在）
        {
            let guard = self.manifest.read();
            let existing_ids: Vec<_> = guard
                .get_file_chunks(file_path)
                .into_iter()
                .map(|(id, _)| id)
                .collect();
            drop(guard);
            if !existing_ids.is_empty() {
                // 删除该文件的向量文件
                let _ = self.store.delete_file_vectors(file_path);
                let mut manifest = self.manifest.write();
                for chunk_id in existing_ids {
                    manifest.remove_chunk(&chunk_id);
                }
                manifest.remove_file(file_path);
            }
        }

        // 3. 分块
        let chunker = TextChunker::new(self.config.embedding.chunk_size);
        let chunks: Vec<Chunk> = chunker.chunk(&content, file_path)?;

        if chunks.is_empty() {
            return Ok(());
        }

        // 4. 生成嵌入（使用引用，零克隆）
        let texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
        let embeddings = embedder.embed(&texts).await?;
        if embeddings.is_empty() {
            return Err(VectorDbError::Embedding("No embeddings returned".into()));
        }
        let actual_dim = embeddings[0].len();
        if actual_dim != self.config.embedding.dimension {
            tracing::error!(
                "向量维度不匹配: 期望 {}, 实际 {}. 请在模型配置中设置正确的维度。",
                self.config.embedding.dimension,
                actual_dim
            );
            return Err(VectorDbError::InvalidDimension {
                expected: self.config.embedding.dimension,
                actual: actual_dim,
            });
        }

        // 5. 写入索引与清单
        let mut file_vectors: Vec<(crate::vector_db::core::ChunkId, Vec<f32>)> = Vec::new();
        {
            let mut manifest = self.manifest.write();
            manifest.add_file(file_path.to_path_buf(), file_hash);
            for (chunk, vecf) in chunks.iter().zip(embeddings.into_iter()) {
                let chunk_hash = blake3_hash_bytes(chunk.content.as_bytes());
                let metadata = ChunkMetadata {
                    file_path: file_path.to_path_buf(),
                    span: chunk.span.clone(),
                    chunk_type: chunk.chunk_type.clone(),
                    hash: chunk_hash,
                };
                // 收集向量数据
                file_vectors.push((chunk.id, vecf.clone()));
                // update in-memory index
                vector_index.add(chunk.id, vecf, metadata.clone())?;
                // add to manifest
                manifest.add_chunk(chunk.id, metadata);
            }
        }

        // 一次性保存该文件的所有向量
        self.store.save_file_vectors(file_path, &file_vectors)?;

        // 6. 文件元数据保存
        let file_meta = crate::vector_db::core::FileMetadata::new(
            file_path.to_path_buf(),
            blake3_hash_bytes(content.as_bytes()),
            last_modified as u64,
            meta.len(),
        );
        self.store.save_file_metadata(&file_meta)?;

        // 7. 保存清单
        self.save_manifest()?;

        Ok(())
    }

    pub async fn index_files_with(
        &self,
        file_paths: &[PathBuf],
        embedder: &dyn Embedder,
        vector_index: &VectorIndex,
    ) -> Result<()> {
        use futures::stream::{self, StreamExt};

        // 收集所有需要索引的文件
        let mut files_to_index = Vec::new();
        for p in file_paths {
            if p.is_file() {
                files_to_index.push(p.clone());
            } else if p.is_dir() {
                let files = collect_source_files(p, self.config.max_file_size);
                files_to_index.extend(files);
            }
        }

        // 并行索引（最多 4 个并发任务）
        let concurrency = 4;
        let results: Vec<Result<()>> = stream::iter(files_to_index)
            .map(|file_path| async move {
                self.index_file_with(&file_path, embedder, vector_index)
                    .await
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;

        // 检查是否有错误
        for result in results {
            result?;
        }

        Ok(())
    }

    pub async fn update_index(
        &self,
        file_path: &Path,
        embedder: &dyn Embedder,
        vector_index: &VectorIndex,
    ) -> Result<()> {
        self.index_file_with(file_path, embedder, vector_index)
            .await
    }

    pub fn remove_file(&self, file_path: &Path) -> Result<()> {
        // 删除该文件的向量文件
        let _ = self.store.delete_file_vectors(file_path);

        let mut manifest = self.manifest.write();
        let chunk_ids: Vec<_> = manifest
            .get_file_chunks(file_path)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        for id in chunk_ids {
            manifest.remove_chunk(&id);
        }
        manifest.remove_file(file_path);
        drop(manifest);
        self.save_manifest()?;
        Ok(())
    }

    pub async fn rebuild(
        &self,
        root: &Path,
        embedder: &dyn Embedder,
        vector_index: &VectorIndex,
    ) -> Result<()> {
        // 重置清单
        {
            let mut manifest = self.manifest.write();
            *manifest = IndexManifest::new(
                self.config.embedding.model_name.clone(),
                self.config.embedding.dimension,
            );
        }
        self.save_manifest()?;

        let files = collect_source_files(root, self.config.max_file_size);
        self.index_files_with(&files, embedder, vector_index).await
    }

    pub fn get_status(&self) -> IndexStatus {
        let manifest = self.manifest.read();
        IndexStatus {
            total_files: manifest.files.len(),
            total_chunks: manifest.chunks.len(),
            embedding_model: manifest.embedding_model.clone(),
            vector_dimension: manifest.vector_dimension,
        }
    }

    /// 获取所有 chunk_id
    pub fn get_chunk_ids(&self) -> Vec<crate::vector_db::core::ChunkId> {
        let manifest = self.manifest.read();
        manifest.chunks.keys().cloned().collect()
    }

    /// 获取所有 chunk 元数据
    pub fn get_all_chunk_metadata(&self) -> Vec<(crate::vector_db::core::ChunkId, ChunkMetadata)> {
        let manifest = self.manifest.read();
        manifest
            .chunks
            .iter()
            .map(|(id, meta)| (*id, meta.clone()))
            .collect()
    }

    /// 获取 FileStore 引用
    pub fn store(&self) -> &FileStore {
        &self.store
    }
}

#[derive(Debug, serde::Serialize)]
pub struct IndexStatus {
    pub total_files: usize,
    pub total_chunks: usize,
    pub embedding_model: String,
    pub vector_dimension: usize,
}
