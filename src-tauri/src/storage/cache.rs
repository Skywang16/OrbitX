//! 统一缓存系统

use crate::utils::error::AppResult;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 统一缓存管理器
#[derive(Clone)]
pub struct UnifiedCache {
    data: Arc<RwLock<HashMap<String, Value>>>,
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
        self.data.read().await.get(key).cloned()
    }

    /// 设置缓存值
    pub async fn set(&self, key: &str, value: Value) -> AppResult<()> {
        self.data.write().await.insert(key.to_string(), value);
        Ok(())
    }

    /// 删除缓存值
    pub async fn remove(&self, key: &str) -> Option<Value> {
        self.data.write().await.remove(key)
    }

    /// 清空所有缓存
    pub async fn clear(&self) -> AppResult<()> {
        self.data.write().await.clear();
        Ok(())
    }

    /// 检查键是否存在
    pub async fn contains_key(&self, key: &str) -> bool {
        self.data.read().await.contains_key(key)
    }

    /// 获取缓存大小
    pub async fn len(&self) -> usize {
        self.data.read().await.len()
    }

    /// 获取所有键
    pub async fn keys(&self) -> Vec<String> {
        self.data.read().await.keys().cloned().collect()
    }

    /// 批量设置
    pub async fn set_batch(&self, items: HashMap<String, Value>) -> AppResult<()> {
        let mut data = self.data.write().await;
        for (key, value) in items {
            data.insert(key, value);
        }
        Ok(())
    }

    /// 批量获取
    pub async fn get_batch(&self, keys: &[String]) -> HashMap<String, Value> {
        let data = self.data.read().await;
        keys.iter()
            .filter_map(|key| data.get(key).map(|value| (key.clone(), value.clone())))
            .collect()
    }
}

impl Default for UnifiedCache {
    fn default() -> Self {
        Self::new()
    }
}
