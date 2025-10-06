//! 统一缓存系统

use crate::utils::error::AppResult;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
struct CacheEntry {
    value: Value,
    expires_at: Option<Instant>,
    created_at: Instant,
    last_accessed: Instant,
    hit_count: u64,
}

impl CacheEntry {
    fn new(value: Value, ttl: Option<Duration>) -> Self {
        let now = Instant::now();
        Self {
            value,
            expires_at: ttl.and_then(|ttl| now.checked_add(ttl)),
            created_at: now,
            last_accessed: now,
            hit_count: 0,
        }
    }

    fn is_expired(&self) -> bool {
        self.expires_at
            .map(|instant| Instant::now() >= instant)
            .unwrap_or(false)
    }

    fn refresh_access(&mut self) {
        self.hit_count = self.hit_count.saturating_add(1);
        self.last_accessed = Instant::now();
    }

    fn remaining_ttl(&self) -> Option<Duration> {
        self.expires_at
            .and_then(|deadline| deadline.checked_duration_since(Instant::now()))
    }
}

/// 缓存条目快照
#[derive(Clone, Debug)]
pub struct CacheEntrySnapshot {
    pub value: Value,
    pub expires_at: Option<Instant>,
    pub created_at: Instant,
    pub last_accessed: Instant,
    pub hit_count: u64,
    pub remaining_ttl: Option<Duration>,
}

/// 统一缓存管理器
#[derive(Clone)]
pub struct UnifiedCache {
    data: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl UnifiedCache {
    /// 创建新的缓存实例
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取缓存值
    pub async fn get(&self, key: &str) -> Option<Value> {
        let mut data = self.data.write().await;
        match data.get_mut(key) {
            Some(entry) if !entry.is_expired() => {
                entry.refresh_access();
                Some(entry.value.clone())
            }
            Some(_) => {
                data.remove(key);
                None
            }
            None => None,
        }
    }

    /// 获取缓存条目信息
    pub async fn snapshot(&self, key: &str) -> Option<CacheEntrySnapshot> {
        let mut data = self.data.write().await;
        match data.get_mut(key) {
            Some(entry) if !entry.is_expired() => {
                entry.refresh_access();
                Some(CacheEntrySnapshot {
                    value: entry.value.clone(),
                    expires_at: entry.expires_at,
                    created_at: entry.created_at,
                    last_accessed: entry.last_accessed,
                    hit_count: entry.hit_count,
                    remaining_ttl: entry.remaining_ttl(),
                })
            }
            Some(_) => {
                data.remove(key);
                None
            }
            None => None,
        }
    }

    /// 设置缓存值
    pub async fn set(&self, key: &str, value: Value) -> AppResult<()> {
        self.set_with_policy(key, value, None).await
    }

    /// 设置带 TTL 的缓存值
    pub async fn set_with_ttl(&self, key: &str, value: Value, ttl: Duration) -> AppResult<()> {
        self.set_with_policy(key, value, Some(ttl)).await
    }

    /// 序列化并存储任意值
    pub async fn set_serialized<T>(&self, key: &str, value: &T) -> AppResult<()>
    where
        T: Serialize,
    {
        let json = serde_json::to_value(value)?;
        self.set(key, json).await
    }

    /// 序列化并存储带 TTL 的值
    pub async fn set_serialized_with_ttl<T>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> AppResult<()>
    where
        T: Serialize,
    {
        let json = serde_json::to_value(value)?;
        self.set_with_ttl(key, json, ttl).await
    }

    /// 以指定类型读取缓存
    pub async fn get_deserialized<T>(&self, key: &str) -> AppResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        match self.get(key).await {
            Some(value) => Ok(Some(serde_json::from_value(value)?)),
            None => Ok(None),
        }
    }

    async fn set_with_policy(
        &self,
        key: &str,
        value: Value,
        ttl: Option<Duration>,
    ) -> AppResult<()> {
        let mut data = self.data.write().await;
        data.insert(key.to_string(), CacheEntry::new(value, ttl));
        Ok(())
    }

    /// 更新指定键的 TTL
    pub async fn update_ttl(&self, key: &str, ttl: Option<Duration>) {
        let mut data = self.data.write().await;
        if let Some(entry) = data.get_mut(key) {
            entry.expires_at = ttl.and_then(|ttl| Instant::now().checked_add(ttl));
        }
    }

    /// 手动刷新命中记录
    pub async fn touch(&self, key: &str) -> bool {
        let mut data = self.data.write().await;
        match data.get_mut(key) {
            Some(entry) if !entry.is_expired() => {
                entry.refresh_access();
                true
            }
            Some(_) => {
                data.remove(key);
                false
            }
            None => false,
        }
    }

    /// 删除缓存值
    pub async fn remove(&self, key: &str) -> Option<Value> {
        self.data.write().await.remove(key).map(|entry| entry.value)
    }

    /// 清空所有缓存
    pub async fn clear(&self) -> AppResult<()> {
        self.data.write().await.clear();
        Ok(())
    }

    /// 检查键是否存在
    pub async fn contains_key(&self, key: &str) -> bool {
        let mut data = self.data.write().await;
        match data.get_mut(key) {
            Some(entry) if !entry.is_expired() => {
                entry.refresh_access();
                true
            }
            Some(_) => {
                data.remove(key);
                false
            }
            None => false,
        }
    }

    /// 获取缓存大小
    pub async fn len(&self) -> usize {
        self.purge_expired().await;
        self.data.read().await.len()
    }

    /// 获取所有键
    pub async fn keys(&self) -> Vec<String> {
        self.purge_expired().await;
        self.data.read().await.keys().cloned().collect()
    }

    /// 批量设置
    pub async fn set_batch(&self, items: HashMap<String, Value>) -> AppResult<()> {
        let mut data = self.data.write().await;
        for (key, value) in items {
            data.insert(key, CacheEntry::new(value, None));
        }
        Ok(())
    }

    /// 批量获取
    pub async fn get_batch(&self, keys: &[String]) -> HashMap<String, Value> {
        let mut data = self.data.write().await;
        let mut result = HashMap::new();

        for key in keys {
            match data.get_mut(key) {
                Some(entry) if !entry.is_expired() => {
                    entry.refresh_access();
                    result.insert(key.clone(), entry.value.clone());
                }
                Some(_) => {
                    data.remove(key);
                }
                None => {}
            }
        }

        result
    }

    /// 清理过期条目并返回清理数量
    pub async fn purge_expired(&self) -> usize {
        let mut data = self.data.write().await;
        let before = data.len();
        data.retain(|_, entry| !entry.is_expired());
        before - data.len()
    }
}

impl Default for UnifiedCache {
    fn default() -> Self {
        Self::new()
    }
}
