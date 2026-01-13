use crate::vector_db::core::{ChunkId, Result, VectorDbConfig, VectorDbError};
use crate::vector_db::storage::{ChunkMetadata, IndexManager};
use hnsw_rs::prelude::*;
use lru::LruCache;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
struct IndexSignature {
    updated_at: u64,
    embedding_model: String,
    vector_dimension: usize,
    total_chunks: usize,
}

impl IndexSignature {
    fn from_manager(manager: &IndexManager) -> Self {
        let status = manager.get_status();
        let manifest = manager.manifest.read();
        Self {
            updated_at: manifest.updated_at,
            embedding_model: status.embedding_model,
            vector_dimension: status.vector_dimension,
            total_chunks: status.total_chunks,
        }
    }
}

pub struct WorkspaceIndexCache {
    inner: Mutex<CacheInner>,
    max_bytes: usize,
}

struct CacheInner {
    lru: LruCache<PathBuf, Arc<CachedWorkspaceIndex>>,
    bytes: usize,
    in_flight: HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>,
}

impl WorkspaceIndexCache {
    pub fn new(max_workspaces: usize, max_bytes: usize) -> Self {
        let cap = NonZeroUsize::new(max_workspaces.max(1)).unwrap();
        Self {
            inner: Mutex::new(CacheInner {
                lru: LruCache::new(cap),
                bytes: 0,
                in_flight: HashMap::new(),
            }),
            max_bytes,
        }
    }

    pub fn invalidate(&self, workspace_root: &Path) {
        let mut inner = self.inner.lock();
        if let Some((_k, v)) = inner.lru.pop_entry(workspace_root) {
            inner.bytes = inner.bytes.saturating_sub(v.approx_bytes);
        }
    }

    pub async fn get_or_build(
        &self,
        workspace_root: &Path,
        config: &VectorDbConfig,
    ) -> Result<Arc<CachedWorkspaceIndex>> {
        let workspace_root = workspace_root.to_path_buf();
        let lock_key = workspace_root.clone();

        // Fast path: check cache with current signature.
        if let Some(entry) = self.try_get_if_fresh(&workspace_root, config)? {
            return Ok(entry);
        }

        let lock = {
            let mut inner = self.inner.lock();
            inner
                .in_flight
                .entry(lock_key.clone())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone()
        };

        let _guard = lock.lock().await;

        // Re-check after acquiring build lock.
        if let Some(entry) = self.try_get_if_fresh(&workspace_root, config)? {
            let mut inner = self.inner.lock();
            inner.in_flight.remove(&lock_key);
            return Ok(entry);
        }

        let config = config.clone();
        let workspace_root_for_build = workspace_root.clone();
        let built = tokio::task::spawn_blocking(move || {
            build_workspace_index(&workspace_root_for_build, &config)
        })
        .await
        .map_err(|e| VectorDbError::Index(format!("index build join failed: {e}")))??;

        let built = Arc::new(built);
        self.insert(workspace_root, built.clone());
        let mut inner = self.inner.lock();
        inner.in_flight.remove(&lock_key);
        Ok(built)
    }

    fn try_get_if_fresh(
        &self,
        workspace_root: &Path,
        config: &VectorDbConfig,
    ) -> Result<Option<Arc<CachedWorkspaceIndex>>> {
        let manager = IndexManager::new(workspace_root, config.clone())?;
        if manager.get_status().total_chunks == 0 {
            return Ok(Some(Arc::new(CachedWorkspaceIndex::empty(
                IndexSignature::from_manager(&manager),
                config.embedding.dimension,
            ))));
        }
        let signature = IndexSignature::from_manager(&manager);

        let mut inner = self.inner.lock();
        if let Some(existing) = inner.lru.get(workspace_root) {
            if existing.signature == signature {
                return Ok(Some(existing.clone()));
            }
        }
        Ok(None)
    }

    fn insert(&self, workspace_root: PathBuf, entry: Arc<CachedWorkspaceIndex>) {
        let mut inner = self.inner.lock();

        if let Some((_k, old)) = inner.lru.pop_entry(&workspace_root) {
            inner.bytes = inner.bytes.saturating_sub(old.approx_bytes);
        }

        inner.bytes = inner.bytes.saturating_add(entry.approx_bytes);
        inner.lru.put(workspace_root, entry);

        while inner.bytes > self.max_bytes {
            if let Some((_k, v)) = inner.lru.pop_lru() {
                inner.bytes = inner.bytes.saturating_sub(v.approx_bytes);
            } else {
                break;
            }
        }
    }
}

pub struct CachedWorkspaceIndex {
    signature: IndexSignature,
    dimension: usize,
    ids: Vec<ChunkId>,
    metas: Vec<ChunkMetadata>,
    hnsw: Option<Hnsw<'static, f32, DistCosine>>,
    approx_bytes: usize,
}

impl CachedWorkspaceIndex {
    fn empty(signature: IndexSignature, dimension: usize) -> Self {
        Self {
            signature,
            dimension,
            ids: Vec::new(),
            metas: Vec::new(),
            hnsw: None,
            approx_bytes: 0,
        }
    }

    pub fn search(&self, query: &[f32], top_k: usize, threshold: f32) -> Result<Vec<(usize, f32)>> {
        if query.len() != self.dimension {
            return Err(VectorDbError::InvalidDimension {
                expected: self.dimension,
                actual: query.len(),
            });
        }
        if self.ids.is_empty() || top_k == 0 {
            return Ok(Vec::new());
        }

        let query_norm = normalize_l2(query);
        let max_dist = 1.0f32 - threshold;

        let hnsw = match &self.hnsw {
            Some(h) => h,
            None => return Ok(Vec::new()),
        };

        // ef_search: trade recall vs latency. Keep it modest to control CPU.
        let ef_search = (top_k * 8).clamp(32, 256);
        let neighbors = hnsw.search(&query_norm, top_k, ef_search);

        let mut results: Vec<(usize, f32)> = neighbors
            .into_iter()
            .filter_map(|n| {
                let idx = n.d_id;
                let dist = n.distance;
                if dist <= max_dist && idx < self.ids.len() {
                    Some((idx, 1.0f32 - dist))
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(results)
    }

    pub fn chunk_meta_by_internal(&self, idx: usize) -> Option<(&ChunkId, &ChunkMetadata)> {
        self.ids.get(idx).zip(self.metas.get(idx))
    }
}

fn build_workspace_index(
    workspace_root: &Path,
    config: &VectorDbConfig,
) -> Result<CachedWorkspaceIndex> {
    let manager = IndexManager::new(workspace_root, config.clone())?;
    let signature = IndexSignature::from_manager(&manager);
    let status = manager.get_status();
    if status.total_chunks == 0 {
        return Ok(CachedWorkspaceIndex::empty(
            signature,
            config.embedding.dimension,
        ));
    }

    if status.vector_dimension != config.embedding.dimension {
        return Err(VectorDbError::InvalidDimension {
            expected: config.embedding.dimension,
            actual: status.vector_dimension,
        });
    }

    let store = manager.store();

    let chunk_metadata_vec = manager.get_all_chunk_metadata();
    let mut by_file: HashMap<PathBuf, Vec<(ChunkId, ChunkMetadata)>> = HashMap::new();
    by_file.reserve(chunk_metadata_vec.len().max(1));
    for (id, meta) in chunk_metadata_vec {
        by_file
            .entry(meta.file_path.clone())
            .or_default()
            .push((id, meta));
    }

    // HNSW params: good default trade-off.
    let m = 16;
    let ef_construction = 200;
    let expected = signature.total_chunks.max(1);
    let hnsw: Hnsw<'static, f32, DistCosine> = Hnsw::new(
        m,
        expected,
        config.embedding.dimension,
        ef_construction,
        DistCosine {},
    );

    let mut ids: Vec<ChunkId> = Vec::with_capacity(signature.total_chunks);
    let mut metas: Vec<ChunkMetadata> = Vec::with_capacity(signature.total_chunks);

    for (file_path, chunks) in by_file {
        let file_vectors = match store.load_file_vectors(&file_path) {
            Ok(v) => v,
            Err(_) => continue,
        };

        for (chunk_id, meta) in chunks {
            let Some(vecf) = file_vectors.chunks.get(&chunk_id) else {
                continue;
            };
            if vecf.len() != config.embedding.dimension {
                return Err(VectorDbError::InvalidDimension {
                    expected: config.embedding.dimension,
                    actual: vecf.len(),
                });
            }

            let v = normalize_l2(vecf);
            let internal_id = ids.len();
            hnsw.insert((&v, internal_id));
            ids.push(chunk_id);
            metas.push(meta);
        }
    }

    let approx_bytes = ids
        .len()
        .saturating_mul(config.embedding.dimension)
        .saturating_mul(std::mem::size_of::<f32>());

    Ok(CachedWorkspaceIndex {
        signature,
        dimension: config.embedding.dimension,
        ids,
        metas,
        hnsw: Some(hnsw),
        approx_bytes,
    })
}

#[inline]
fn normalize_l2(vector: &[f32]) -> Vec<f32> {
    // Keep it simple: this runs during build, not per dot-product loop.
    let mut norm_sq = 0.0f32;
    for &x in vector {
        norm_sq += x * x;
    }
    let norm = norm_sq.sqrt();
    if norm > 0.0 {
        vector.iter().map(|&x| x / norm).collect()
    } else {
        vector.to_vec()
    }
}
