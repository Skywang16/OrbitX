//! 统一缓存系统 - 带命名空间管理

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

/// 缓存命名空间 - 避免不同模块的 key 冲突
#[derive(Debug, Clone, Copy)]
pub enum CacheNamespace {
    Rules,      // 用户规则、项目规则
    Session,    // 会话状态
    UI,         // UI 状态
    Agent,      // Agent 临时数据
    Completion, // 补全缓存
    Terminal,   // 终端相关
    Global,     // 全局命名空间（默认）
}

impl CacheNamespace {
    fn prefix(&self) -> &'static str {
        match self {
            Self::Rules => "rules:",
            Self::Session => "session:",
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

    // ==================== 带命名空间的新 API ====================

    /// 获取缓存值（带命名空间）
    pub async fn get_ns(&self, namespace: CacheNamespace, key: &str) -> Option<Value> {
        self.get(&namespace.make_key(key)).await
    }

    /// 设置缓存值（带命名空间）
    pub async fn set_ns(
        &self,
        namespace: CacheNamespace,
        key: &str,
        value: Value,
    ) -> CacheResult<()> {
        self.set(&namespace.make_key(key), value).await
    }

    /// 设置带 TTL 的缓存值（带命名空间）
    pub async fn set_ns_with_ttl(
        &self,
        namespace: CacheNamespace,
        key: &str,
        value: Value,
        ttl: Duration,
    ) -> CacheResult<()> {
        self.set_with_ttl(&namespace.make_key(key), value, ttl)
            .await
    }

    /// 序列化并存储任意值（带命名空间）
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

    /// 序列化并存储带 TTL 的值（带命名空间）
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

    /// 以指定类型读取缓存（带命名空间）
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

    /// 删除缓存值（带命名空间）
    pub async fn remove_ns(&self, namespace: CacheNamespace, key: &str) -> Option<Value> {
        self.remove(&namespace.make_key(key)).await
    }

    /// 检查键是否存在（带命名空间）
    pub async fn contains_key_ns(&self, namespace: CacheNamespace, key: &str) -> bool {
        self.contains_key(&namespace.make_key(key)).await
    }

    /// 清空整个命名空间
    pub async fn clear_namespace(&self, namespace: CacheNamespace) -> usize {
        let prefix = namespace.prefix();
        if prefix.is_empty() {
            // Global namespace - 清空所有
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

    /// 获取命名空间下的所有 key（不含前缀）
    pub async fn keys_in_namespace(&self, namespace: CacheNamespace) -> Vec<String> {
        let prefix = namespace.prefix();
        let prefix_len = prefix.len();

        self.purge_expired().await;
        self.data
            .read()
            .await
            .keys()
            .filter_map(|key| {
                if key.starts_with(prefix) {
                    Some(key[prefix_len..].to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    // ==================== 便捷方法（常用的快捷方式）====================

    /// Rules: 获取用户规则
    pub async fn get_user_rules(&self) -> Option<String> {
        self.get_deserialized_ns(CacheNamespace::Rules, "user_rules")
            .await
            .ok()
            .flatten()
    }

    /// Rules: 设置用户规则
    pub async fn set_user_rules(&self, rules: Option<String>) -> CacheResult<()> {
        if let Some(r) = rules {
            self.set_serialized_ns(CacheNamespace::Rules, "user_rules", &r)
                .await
        } else {
            self.remove_ns(CacheNamespace::Rules, "user_rules").await;
            Ok(())
        }
    }

    /// Rules: 获取项目规则
    pub async fn get_project_rules(&self) -> Option<String> {
        self.get_deserialized_ns(CacheNamespace::Rules, "project_rules")
            .await
            .ok()
            .flatten()
    }

    /// Rules: 设置项目规则
    pub async fn set_project_rules(&self, rules: Option<String>) -> CacheResult<()> {
        if let Some(r) = rules {
            self.set_serialized_ns(CacheNamespace::Rules, "project_rules", &r)
                .await
        } else {
            self.remove_ns(CacheNamespace::Rules, "project_rules").await;
            Ok(())
        }
    }

    /// Session: 获取活跃会话
    pub async fn get_active_session(&self) -> Option<i64> {
        self.get_deserialized_ns(CacheNamespace::Session, "active_session")
            .await
            .ok()
            .flatten()
    }

    /// Session: 设置活跃会话
    pub async fn set_active_session(&self, id: Option<i64>) -> CacheResult<()> {
        if let Some(session_id) = id {
            self.set_serialized_ns(CacheNamespace::Session, "active_session", &session_id)
                .await
        } else {
            self.remove_ns(CacheNamespace::Session, "active_session")
                .await;
            Ok(())
        }
    }

    // ==================== 原有的无命名空间 API（保持向后兼容）====================

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
    pub async fn set(&self, key: &str, value: Value) -> CacheResult<()> {
        self.set_with_policy(key, value, None).await
    }

    /// 设置带 TTL 的缓存值
    pub async fn set_with_ttl(&self, key: &str, value: Value, ttl: Duration) -> CacheResult<()> {
        self.set_with_policy(key, value, Some(ttl)).await
    }

    /// 序列化并存储任意值
    pub async fn set_serialized<T>(&self, key: &str, value: &T) -> CacheResult<()>
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
    ) -> CacheResult<()>
    where
        T: Serialize,
    {
        let json = serde_json::to_value(value)?;
        self.set_with_ttl(key, json, ttl).await
    }

    /// 以指定类型读取缓存
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
    pub async fn clear(&self) -> CacheResult<()> {
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
    pub async fn set_batch(&self, items: HashMap<String, Value>) -> CacheResult<()> {
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
