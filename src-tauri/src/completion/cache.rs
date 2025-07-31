//! 补全缓存模块
//!
//! 提供高效的补全结果缓存，减少重复计算

use crate::completion::types::CompletionItem;
use crate::utils::error::AppResult;
use anyhow::anyhow;
use lru::LruCache;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// 缓存项
#[derive(Debug, Clone)]
struct CacheEntry {
    /// 补全结果
    items: Vec<CompletionItem>,
    /// 创建时间
    created_at: Instant,
    /// 过期时间
    ttl: Duration,
}

impl CacheEntry {
    /// 创建新的缓存项
    fn new(items: Vec<CompletionItem>, ttl: Duration) -> Self {
        Self {
            items,
            created_at: Instant::now(),
            ttl,
        }
    }

    /// 检查是否过期
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// 补全缓存
pub struct CompletionCache {
    /// LRU缓存
    cache: Arc<RwLock<LruCache<u64, CacheEntry>>>,
    /// 默认TTL
    default_ttl: Duration,
}

impl CompletionCache {
    /// 创建新的补全缓存
    pub fn new(capacity: usize, default_ttl: Duration) -> AppResult<Self> {
        let capacity =
            NonZeroUsize::new(capacity).ok_or_else(|| anyhow!("缓存错误: 缓存容量不能为0"))?;

        Ok(Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            default_ttl,
        })
    }

    /// 生成缓存键
    fn generate_key(&self, input: &str, working_dir: &str, provider_type: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        working_dir.hash(&mut hasher);
        provider_type.hash(&mut hasher);
        hasher.finish()
    }

    /// 获取缓存项
    pub fn get(
        &self,
        input: &str,
        working_dir: &str,
        provider_type: &str,
    ) -> Option<Vec<CompletionItem>> {
        let key = self.generate_key(input, working_dir, provider_type);

        let mut cache = self.cache.write().ok()?;

        if let Some(entry) = cache.get(&key) {
            if !entry.is_expired() {
                return Some(entry.items.clone());
            } else {
                // 移除过期项
                cache.pop(&key);
            }
        }

        None
    }

    /// 存储缓存项
    pub fn put(
        &self,
        input: &str,
        working_dir: &str,
        provider_type: &str,
        items: Vec<CompletionItem>,
    ) -> AppResult<()> {
        self.put_with_ttl(input, working_dir, provider_type, items, self.default_ttl)
    }

    /// 存储缓存项（指定TTL）
    pub fn put_with_ttl(
        &self,
        input: &str,
        working_dir: &str,
        provider_type: &str,
        items: Vec<CompletionItem>,
        ttl: Duration,
    ) -> AppResult<()> {
        let key = self.generate_key(input, working_dir, provider_type);
        let entry = CacheEntry::new(items, ttl);

        let mut cache = self
            .cache
            .write()
            .map_err(|_| anyhow!("缓存错误: 获取缓存写锁失败"))?;

        cache.put(key, entry);
        Ok(())
    }

    /// 清除所有缓存
    pub fn clear(&self) -> AppResult<()> {
        let mut cache = self
            .cache
            .write()
            .map_err(|_| anyhow!("缓存错误: 获取缓存写锁失败"))?;

        cache.clear();
        Ok(())
    }

    /// 清除过期缓存
    pub fn cleanup_expired(&self) -> AppResult<usize> {
        let mut cache = self
            .cache
            .write()
            .map_err(|_| anyhow!("获取缓存写锁失败"))?;

        let mut expired_keys = Vec::new();

        // 收集过期的键
        for (key, entry) in cache.iter() {
            if entry.is_expired() {
                expired_keys.push(*key);
            }
        }

        // 移除过期项
        let removed_count = expired_keys.len();
        for key in expired_keys {
            cache.pop(&key);
        }

        Ok(removed_count)
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> AppResult<CacheStats> {
        let cache = self
            .cache
            .read()
            .map_err(|_| anyhow!("缓存错误: 获取缓存读锁失败"))?;

        let total_entries = cache.len();
        let capacity = cache.cap().get();

        // 计算过期项数量
        let expired_count = cache.iter().filter(|(_, entry)| entry.is_expired()).count();

        Ok(CacheStats {
            total_entries,
            capacity,
            expired_entries: expired_count,
            hit_rate: 0.0, // TODO: 实现命中率统计
        })
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 总条目数
    pub total_entries: usize,
    /// 缓存容量
    pub capacity: usize,
    /// 过期条目数
    pub expired_entries: usize,
    /// 命中率
    pub hit_rate: f64,
}

impl Default for CompletionCache {
    fn default() -> Self {
        Self::new(1000, Duration::from_secs(300)).expect("创建默认缓存失败")
    }
}
