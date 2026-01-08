use crate::vector_db::core::{ChunkId, Result, VectorDbError};
use crate::vector_db::storage::{ChunkMetadata, FileStore};
use parking_lot::RwLock;
use simsimd::SpatialSimilarity;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;

/// 向量索引数据（合并锁以减少争用）
struct IndexData {
    /// 规范化后的向量 (chunk_id -> 向量)
    vectors: HashMap<ChunkId, Vec<f32>>,
    /// 块元数据
    chunks: HashMap<ChunkId, ChunkMetadata>,
}

/// 向量索引
pub struct VectorIndex {
    /// 索引数据（单锁设计）
    data: Arc<RwLock<IndexData>>,
    /// 向量维度
    dimension: usize,
}

/// 用于堆的最小分数包装
#[derive(PartialEq)]
struct MinScore(f32, ChunkId);

impl Eq for MinScore {}

impl PartialOrd for MinScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // 反转比较以创建最小堆
        other.0.partial_cmp(&self.0)
    }
}

impl Ord for MinScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl VectorIndex {
    /// 创建新索引
    pub fn new(dimension: usize) -> Self {
        Self {
            data: Arc::new(RwLock::new(IndexData {
                vectors: HashMap::new(),
                chunks: HashMap::new(),
            })),
            dimension,
        }
    }

    /// 规范化向量（L2 归一化）——使用 SIMD 优化
    #[inline]
    fn normalize_vector(vector: &[f32]) -> Vec<f32> {
        // L2范数平方 = 向量与自身的点积
        let norm_sq = f32::dot(vector, vector).unwrap_or(0.0);
        let norm = (norm_sq as f32).sqrt();
        if norm > 0.0 {
            vector.iter().map(|&x| x / norm).collect()
        } else {
            vector.to_vec()
        }
    }

    /// 计算点积（规范化向量的点积即为余弦相似度）——使用 SIMD 优化
    #[inline]
    fn dot_product(a: &[f32], b: &[f32]) -> f32 {
        f32::dot(a, b).unwrap_or(0.0) as f32
    }

    /// 添加向量（自动规范化）
    pub fn add(&self, chunk_id: ChunkId, vector: Vec<f32>, metadata: ChunkMetadata) -> Result<()> {
        if vector.len() != self.dimension {
            return Err(VectorDbError::InvalidDimension {
                expected: self.dimension,
                actual: vector.len(),
            });
        }

        // 规范化向量
        let normalized = Self::normalize_vector(&vector);

        let mut data = self.data.write();
        data.vectors.insert(chunk_id, normalized);
        data.chunks.insert(chunk_id, metadata);

        Ok(())
    }

    /// 批量添加向量（自动规范化，预分配容量）
    pub fn add_batch(&self, items: Vec<(ChunkId, Vec<f32>, ChunkMetadata)>) -> Result<()> {
        let mut data = self.data.write();

        // 预分配容量
        data.vectors.reserve(items.len());
        data.chunks.reserve(items.len());

        for (chunk_id, vector, metadata) in items {
            if vector.len() != self.dimension {
                return Err(VectorDbError::InvalidDimension {
                    expected: self.dimension,
                    actual: vector.len(),
                });
            }

            // 规范化向量
            let normalized = Self::normalize_vector(&vector);
            data.vectors.insert(chunk_id, normalized);
            data.chunks.insert(chunk_id, metadata);
        }

        Ok(())
    }

    /// 删除向量
    pub fn remove(&self, chunk_id: &ChunkId) -> Result<()> {
        let mut data = self.data.write();
        data.vectors.remove(chunk_id);
        data.chunks.remove(chunk_id);
        Ok(())
    }

    /// 搜索相似向量（使用堆维护 top-k，O(n log k) 时间复杂度）
    pub fn search(
        &self,
        query: &[f32],
        top_k: usize,
        threshold: f32,
    ) -> Result<Vec<(ChunkId, f32)>> {
        if query.len() != self.dimension {
            return Err(VectorDbError::InvalidDimension {
                expected: self.dimension,
                actual: query.len(),
            });
        }

        // 规范化查询向量（只计算一次）
        let query_normalized = Self::normalize_vector(query);

        let data = self.data.read();

        // 使用最小堆维护 top-k（性能更优）
        let mut top_k_heap = BinaryHeap::with_capacity(top_k);

        for (chunk_id, vector) in data.vectors.iter() {
            // 规范化向量的点积即为余弦相似度
            let similarity = Self::dot_product(&query_normalized, vector);

            if similarity < threshold {
                continue;
            }

            if top_k_heap.len() < top_k {
                top_k_heap.push(MinScore(similarity, *chunk_id));
            } else if let Some(min) = top_k_heap.peek() {
                if similarity > min.0 {
                    top_k_heap.pop();
                    top_k_heap.push(MinScore(similarity, *chunk_id));
                }
            }
        }

        // 从堆中提取结果并排序
        let mut results: Vec<(ChunkId, f32)> = top_k_heap
            .into_iter()
            .map(|MinScore(score, id)| (id, score))
            .collect();

        // 按相似度降序排序
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

        Ok(results)
    }

    /// 获取块元数据
    pub fn get_chunk_metadata(&self, chunk_id: &ChunkId) -> Option<ChunkMetadata> {
        self.data.read().chunks.get(chunk_id).cloned()
    }

    /// 获取向量数量
    pub fn count(&self) -> usize {
        self.data.read().vectors.len()
    }

    /// 持久化到磁盘
    pub async fn save(&self, store: &FileStore) -> Result<()> {
        use std::collections::HashMap;
        use std::path::PathBuf;

        let data = self.data.read();

        // 按文件分组收集向量
        let mut file_vectors: HashMap<PathBuf, Vec<(ChunkId, Vec<f32>)>> = HashMap::new();

        for (chunk_id, vector) in data.vectors.iter() {
            if let Some(chunk_metadata) = data.chunks.get(chunk_id) {
                file_vectors
                    .entry(chunk_metadata.file_path.clone())
                    .or_default()
                    .push((*chunk_id, vector.clone()));
            }
        }

        // 每个文件保存一个向量文件
        for (file_path, vectors) in file_vectors {
            store.save_file_vectors(&file_path, &vectors)?;
        }

        Ok(())
    }

    /// 从磁盘加载
    pub async fn load(
        store: &FileStore,
        chunk_metadata: &HashMap<ChunkId, ChunkMetadata>,
        dimension: usize,
    ) -> Result<Self> {
        use std::collections::HashMap;
        use std::path::PathBuf;

        let index = Self::new(dimension);
        let mut data = index.data.write();

        // 预分配容量
        let chunk_count = chunk_metadata.len();
        data.vectors.reserve(chunk_count);
        data.chunks.reserve(chunk_count);

        // 按文件分组加载
        let mut files_to_load: HashMap<PathBuf, Vec<(ChunkId, &ChunkMetadata)>> = HashMap::new();
        for (chunk_id, metadata) in chunk_metadata {
            files_to_load
                .entry(metadata.file_path.clone())
                .or_default()
                .push((*chunk_id, metadata));
        }

        // 每个文件加载一次
        for (file_path, chunks) in files_to_load {
            if let Ok(file_vectors) = store.load_file_vectors(&file_path) {
                for (chunk_id, metadata) in chunks {
                    if let Some(vector) = file_vectors.chunks.get(&chunk_id) {
                        // 规范化加载的向量
                        let normalized = Self::normalize_vector(vector);
                        data.vectors.insert(chunk_id, normalized);
                        data.chunks.insert(chunk_id, metadata.clone());
                    }
                }
            }
        }

        drop(data);
        Ok(index)
    }
}
