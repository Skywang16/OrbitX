//! Unified cache system - with namespace management

use crate::storage::error::CacheResult;
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

/// Cache entry snapshot
#[derive(Clone, Debug)]
pub struct CacheEntrySnapshot {
    pub value: Value,
    pub expires_at: Option<Instant>,
    pub created_at: Instant,
    pub last_accessed: Instant,
    pub hit_count: u64,
    pub remaining_ttl: Option<Duration>,
}

/// Cache namespace - avoid key conflicts between different modules
#[derive(Debug, Clone, Copy)]
pub enum CacheNamespace {
    Rules,      // Global rules, project rules
    Thread,     // Thread state
    UI,         // UI state
    Agent,      // Agent temporary data
    Completion, // Completion cache
    Terminal,   // Terminal related
    Global,     // Global namespace (default)
}

impl CacheNamespace {
    fn prefix(&self) -> &'static str {
        match self {
            Self::Rules => "rules:",
            Self::Thread => "thread:",
            Self::UI => "ui:",
            Self::Agent => "agent:",
            Self::Completion => "completion:",
            Self::Terminal => "terminal:",
            Self::Global => "",
        }
    }

    fn make_key(&self, key: &str) -> String {
        format!("{}{}", self.prefix(), key)
    }
}

/// Unified cache manager
#[derive(Clone)]
pub struct UnifiedCache {
    data: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl UnifiedCache {
    /// Create new cache instance
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ==================== New API with namespace ====================

    /// Serialize and store arbitrary value (with namespace)
    pub async fn set_serialized_ns<T>(
        &self,
        namespace: CacheNamespace,
        key: &str,
        value: &T,
    ) -> CacheResult<()>
    where
        T: Serialize,
    {
        self.set_serialized(&namespace.make_key(key), value).await
    }

    /// Serialize and store value with TTL (with namespace)
    pub async fn set_serialized_ns_with_ttl<T>(
        &self,
        namespace: CacheNamespace,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> CacheResult<()>
    where
        T: Serialize,
    {
        self.set_serialized_with_ttl(&namespace.make_key(key), value, ttl)
            .await
    }

    /// Read cache as specified type (with namespace)
    pub async fn get_deserialized_ns<T>(
        &self,
        namespace: CacheNamespace,
        key: &str,
    ) -> CacheResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        self.get_deserialized(&namespace.make_key(key)).await
    }

    /// Remove cache value (with namespace)
    pub async fn remove_ns(&self, namespace: CacheNamespace, key: &str) -> Option<Value> {
        self.remove(&namespace.make_key(key)).await
    }

    /// Clear entire namespace
    pub async fn clear_namespace(&self, namespace: CacheNamespace) -> usize {
        let prefix = namespace.prefix();
        if prefix.is_empty() {
            // Global namespace - clear all
            let len = self.data.read().await.len();
            self.data.write().await.clear();
            return len;
        }

        let mut data = self.data.write().await;
        let keys_to_remove: Vec<String> = data
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();

        let removed = keys_to_remove.len();
        for key in keys_to_remove {
            data.remove(&key);
        }
        removed
    }

    // ==================== Convenience methods (common shortcuts) ====================

    /// Rules: Set global rules
    pub async fn set_global_rules(&self, rules: Option<String>) -> CacheResult<()> {
        if let Some(r) = rules {
            self.set_serialized_ns(CacheNamespace::Rules, "global_rules", &r)
                .await
        } else {
            self.remove_ns(CacheNamespace::Rules, "global_rules").await;
            Ok(())
        }
    }

    /// Rules: Set project rules
    pub async fn set_project_rules(&self, rules: Option<String>) -> CacheResult<()> {
        if let Some(r) = rules {
            self.set_serialized_ns(CacheNamespace::Rules, "project_rules", &r)
                .await
        } else {
            self.remove_ns(CacheNamespace::Rules, "project_rules").await;
            Ok(())
        }
    }

    /// Thread: Get active thread
    pub async fn get_active_thread(&self) -> Option<i64> {
        match self
            .get_deserialized_ns(CacheNamespace::Thread, "active_thread")
            .await
        {
            Ok(thread_id) => thread_id,
            Err(err) => {
                tracing::warn!("Failed to load active thread from cache: {}", err);
                None
            }
        }
    }

    /// Thread: Set active thread
    pub async fn set_active_thread(&self, id: Option<i64>) -> CacheResult<()> {
        if let Some(thread_id) = id {
            self.set_serialized_ns(CacheNamespace::Thread, "active_thread", &thread_id)
                .await
        } else {
            self.remove_ns(CacheNamespace::Thread, "active_thread")
                .await;
            Ok(())
        }
    }

    // ==================== Original API without namespace (maintain backward compatibility) ====================

    /// Get cache value
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

    /// Get cache entry information
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

    /// Set cache value
    pub async fn set(&self, key: &str, value: Value) -> CacheResult<()> {
        self.set_with_policy(key, value, None).await
    }

    /// Set cache value with TTL
    pub async fn set_with_ttl(&self, key: &str, value: Value, ttl: Duration) -> CacheResult<()> {
        self.set_with_policy(key, value, Some(ttl)).await
    }

    /// Serialize and store arbitrary value
    pub async fn set_serialized<T>(&self, key: &str, value: &T) -> CacheResult<()>
    where
        T: Serialize,
    {
        let json = serde_json::to_value(value)?;
        self.set(key, json).await
    }

    /// Serialize and store value with TTL
    pub async fn set_serialized_with_ttl<T>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> CacheResult<()>
    where
        T: Serialize,
    {
        let json = serde_json::to_value(value)?;
        self.set_with_ttl(key, json, ttl).await
    }

    /// Read cache as specified type
    pub async fn get_deserialized<T>(&self, key: &str) -> CacheResult<Option<T>>
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
    ) -> CacheResult<()> {
        let mut data = self.data.write().await;
        data.insert(key.to_string(), CacheEntry::new(value, ttl));
        Ok(())
    }

    /// Update TTL for specified key
    pub async fn update_ttl(&self, key: &str, ttl: Option<Duration>) {
        let mut data = self.data.write().await;
        if let Some(entry) = data.get_mut(key) {
            entry.expires_at = ttl.and_then(|ttl| Instant::now().checked_add(ttl));
        }
    }

    /// Manually refresh hit record
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

    /// Remove cache value
    pub async fn remove(&self, key: &str) -> Option<Value> {
        self.data.write().await.remove(key).map(|entry| entry.value)
    }

    /// Clear all cache
    pub async fn clear(&self) -> CacheResult<()> {
        self.data.write().await.clear();
        Ok(())
    }

    /// Check if key exists
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

    /// Get cache size
    pub async fn len(&self) -> usize {
        self.purge_expired().await;
        self.data.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.purge_expired().await;
        self.data.read().await.is_empty()
    }

    /// Get all keys
    pub async fn keys(&self) -> Vec<String> {
        self.purge_expired().await;
        self.data.read().await.keys().cloned().collect()
    }

    /// Batch get
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

    /// Clean expired entries and return cleanup count
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
