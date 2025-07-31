/*!
 * 简化的三层缓存系统
 *
 * 根据设计文档实现：
 * L1: 内存缓存（配置数据）
 * L2: LRU缓存（查询结果）
 * L3: 磁盘缓存（预编译查询）
 *
 * 遵循"不过度工程化"原则，保持简洁
 */

use crate::storage::paths::StoragePaths;
use crate::utils::error::AppResult;
use anyhow::Context;
use lru::LruCache;
use serde_json::Value;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// 简化的缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// L1内存缓存大小限制
    pub l1_capacity: usize,
    /// L2 LRU缓存大小限制
    pub l2_capacity: usize,
    /// 是否启用L3磁盘缓存
    pub l3_enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_capacity: 100,
            l2_capacity: 1000,
            l3_enabled: true,
        }
    }
}

/// 简化的多层缓存管理器
pub struct MultiLayerCache {
    /// L1: 内存缓存（配置数据）
    l1_memory: Arc<RwLock<HashMap<String, Value>>>,
    /// L2: LRU缓存（查询结果）
    l2_lru: Arc<RwLock<LruCache<String, Value>>>,
    /// L3: 磁盘缓存路径
    l3_disk_path: std::path::PathBuf,
    /// 配置
    config: CacheConfig,
}

impl MultiLayerCache {
    /// 创建新的多层缓存管理器
    pub async fn new(paths: &StoragePaths, config: CacheConfig) -> AppResult<Self> {
        info!("初始化简化的多层缓存系统");

        let l1_memory = Arc::new(RwLock::new(HashMap::new()));
        let l2_lru = Arc::new(RwLock::new(LruCache::new(
            NonZeroUsize::new(config.l2_capacity).unwrap(),
        )));
        let l3_disk_path = paths.cache_dir.clone();

        // 确保磁盘缓存目录存在
        if config.l3_enabled {
            tokio::fs::create_dir_all(&l3_disk_path)
                .await
                .context("创建磁盘缓存目录失败")?;
        }

        Ok(Self {
            l1_memory,
            l2_lru,
            l3_disk_path,
            config,
        })
    }

    /// 获取缓存值
    pub async fn get(&self, key: &str) -> AppResult<Option<Value>> {
        debug!("从缓存获取: {}", key);

        // L1: 检查内存缓存
        {
            let l1 = self.l1_memory.read().await;
            if let Some(value) = l1.get(key) {
                debug!("从L1内存缓存命中: {}", key);
                return Ok(Some(value.clone()));
            }
        }

        // L2: 检查LRU缓存
        {
            let mut l2 = self.l2_lru.write().await;
            if let Some(value) = l2.get(key) {
                debug!("从L2 LRU缓存命中: {}", key);
                return Ok(Some(value.clone()));
            }
        }

        // L3: 检查磁盘缓存
        if self.config.l3_enabled {
            let disk_file = self.l3_disk_path.join(format!("{}.json", key));
            if disk_file.exists() {
                match tokio::fs::read_to_string(&disk_file).await {
                    Ok(content) => {
                        if let Ok(value) = serde_json::from_str::<Value>(&content) {
                            debug!("从L3磁盘缓存命中: {}", key);
                            // 将值提升到L2缓存
                            let mut l2 = self.l2_lru.write().await;
                            l2.put(key.to_string(), value.clone());
                            return Ok(Some(value));
                        }
                    }
                    Err(e) => {
                        debug!("读取磁盘缓存失败: {}", e);
                    }
                }
            }
        }

        Ok(None)
    }

    /// 设置缓存值
    pub async fn put(
        &self,
        key: &str,
        value: Value,
        _ttl: Option<std::time::Duration>,
    ) -> AppResult<()> {
        debug!("设置缓存: {}", key);

        // 根据键的类型决定存储层级
        if key.starts_with("config:") {
            // 配置数据存储到L1内存缓存
            let mut l1 = self.l1_memory.write().await;
            l1.insert(key.to_string(), value);
        } else {
            // 查询结果存储到L2 LRU缓存
            let mut l2 = self.l2_lru.write().await;
            l2.put(key.to_string(), value.clone());

            // 如果启用L3，也存储到磁盘
            if self.config.l3_enabled {
                let disk_file = self.l3_disk_path.join(format!("{}.json", key));
                if let Ok(content) = serde_json::to_string_pretty(&value) {
                    if let Err(e) = tokio::fs::write(&disk_file, content).await {
                        debug!("写入磁盘缓存失败: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// 清理所有缓存
    pub async fn clear(&self) -> AppResult<()> {
        info!("清理所有缓存");

        // 清理L1内存缓存
        {
            let mut l1 = self.l1_memory.write().await;
            l1.clear();
        }

        // 清理L2 LRU缓存
        {
            let mut l2 = self.l2_lru.write().await;
            l2.clear();
        }

        // 清理L3磁盘缓存
        if self.config.l3_enabled && self.l3_disk_path.exists() {
            if let Ok(mut entries) = tokio::fs::read_dir(&self.l3_disk_path).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if entry.path().extension().map_or(false, |ext| ext == "json") {
                        let _ = tokio::fs::remove_file(entry.path()).await;
                    }
                }
            }
        }

        Ok(())
    }

    /// 设置缓存值（put方法的别名，保持API一致性）
    pub async fn set(&self, key: &str, value: Value) -> AppResult<()> {
        self.put(key, value, None).await
    }

    /// 获取缓存统计信息
    pub async fn get_stats(&self) -> crate::storage::types::CacheStats {
        use crate::storage::types::{CacheLayer, LayerStats};
        use std::collections::HashMap;
        use std::time::Duration;

        let l1_count = {
            let l1 = self.l1_memory.read().await;
            l1.len()
        };

        let l2_count = {
            let l2 = self.l2_lru.read().await;
            l2.len()
        };

        // 计算磁盘缓存大小
        let mut l3_count = 0;
        let mut l3_size = 0;
        if self.config.l3_enabled && self.l3_disk_path.exists() {
            if let Ok(mut entries) = tokio::fs::read_dir(&self.l3_disk_path).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if entry.path().extension().map_or(false, |ext| ext == "json") {
                        l3_count += 1;
                        if let Ok(metadata) = entry.metadata().await {
                            l3_size += metadata.len();
                        }
                    }
                }
            }
        }

        let mut layers = HashMap::new();

        // L1 内存缓存统计
        layers.insert(
            CacheLayer::Memory,
            LayerStats {
                hits: 0, // 简化实现
                misses: 0,
                entries: l1_count,
                memory_usage: l1_count as u64 * 1024, // 估算
                avg_access_time: Duration::from_nanos(100),
            },
        );

        // L2 LRU缓存统计
        layers.insert(
            CacheLayer::Lru,
            LayerStats {
                hits: 0,
                misses: 0,
                entries: l2_count,
                memory_usage: l2_count as u64 * 1024, // 估算
                avg_access_time: Duration::from_nanos(500),
            },
        );

        // L3 磁盘缓存统计
        layers.insert(
            CacheLayer::Disk,
            LayerStats {
                hits: 0,
                misses: 0,
                entries: l3_count,
                memory_usage: l3_size,
                avg_access_time: Duration::from_millis(10),
            },
        );

        crate::storage::types::CacheStats {
            layers,
            total_hit_rate: 0.0, // 简化实现
            total_memory_usage: (l1_count + l2_count) as u64 * 1024 + l3_size,
            total_entries: l1_count + l2_count + l3_count,
        }
    }
}
