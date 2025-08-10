/*!
 * 配置缓存系统独立测试
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tempfile::TempDir;
use tokio::fs;
use tokio::sync::Notify;

// 简化的配置结构用于测试
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestConfig {
    pub name: String,
    pub value: i32,
}

// 简化的错误类型
#[derive(Debug)]
pub enum TestError {
    IoError(String),
    SerializationError(String),
}

impl From<std::io::Error> for TestError {
    fn from(error: std::io::Error) -> Self {
        TestError::IoError(error.to_string())
    }
}

impl From<bincode::Error> for TestError {
    fn from(error: bincode::Error) -> Self {
        TestError::SerializationError(error.to_string())
    }
}

type TestResult<T> = Result<T, TestError>;

/// 缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub created_at: SystemTime,
    pub last_accessed: SystemTime,
    pub access_count: u64,
    pub expires_at: Option<SystemTime>,
    pub version: u64,
}

impl<T> CacheEntry<T> {
    pub fn new(data: T, ttl: Option<Duration>) -> Self {
        let now = SystemTime::now();
        Self {
            data,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            expires_at: ttl.map(|duration| now + duration),
            version: 1,
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }

    pub fn touch(&mut self) {
        self.last_accessed = SystemTime::now();
        self.access_count += 1;
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
    pub hit_rate: f64,
}

impl CacheStats {
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            entries: 0,
            hit_rate: 0.0,
        }
    }

    pub fn update_hit_rate(&mut self) {
        let total = self.hits + self.misses;
        self.hit_rate = if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        };
    }
}

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_memory_entries: usize,
    pub memory_ttl: Duration,
    pub enable_disk_cache: bool,
    pub disk_cache_dir: PathBuf,
    pub disk_ttl: Duration,
    pub cleanup_interval: Duration,
}

/// 简化的配置缓存系统
pub struct TestConfigCache {
    memory_cache: Arc<RwLock<HashMap<String, CacheEntry<TestConfig>>>>,
    config: CacheConfig,
    stats: Arc<RwLock<CacheStats>>,
    invalidation_notify: Arc<Notify>,
    cleanup_handle: Option<tokio::task::JoinHandle<()>>,
}

impl TestConfigCache {
    pub fn new(config: CacheConfig) -> TestResult<Self> {
        Ok(Self {
            memory_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(CacheStats::new())),
            invalidation_notify: Arc::new(Notify::new()),
            cleanup_handle: None,
        })
    }

    pub async fn start(&mut self) -> TestResult<()> {
        if self.config.enable_disk_cache {
            fs::create_dir_all(&self.config.disk_cache_dir).await?;
        }
        self.start_cleanup_task().await;
        Ok(())
    }

    pub async fn get(&self, key: &str) -> TestResult<Option<TestConfig>> {
        if let Some(config) = self.get_from_memory(key).await? {
            return Ok(Some(config));
        }

        if self.config.enable_disk_cache {
            if let Some(config) = self.get_from_disk(key).await? {
                self.put_to_memory(key, &config).await?;
                return Ok(Some(config));
            }
        }

        {
            let mut stats = self.stats.write().unwrap();
            stats.misses += 1;
            stats.update_hit_rate();
        }

        Ok(None)
    }

    pub async fn put(&self, key: &str, config: &TestConfig) -> TestResult<()> {
        self.put_to_memory(key, config).await?;

        if self.config.enable_disk_cache {
            self.put_to_disk(key, config).await?;
        }

        Ok(())
    }

    async fn get_from_memory(&self, key: &str) -> TestResult<Option<TestConfig>> {
        let mut cache = self.memory_cache.write().unwrap();

        if let Some(entry) = cache.get_mut(key) {
            if entry.is_expired() {
                cache.remove(key);
                return Ok(None);
            }

            entry.touch();

            {
                let mut stats = self.stats.write().unwrap();
                stats.hits += 1;
                stats.update_hit_rate();
            }

            return Ok(Some(entry.data.clone()));
        }

        Ok(None)
    }

    async fn put_to_memory(&self, key: &str, config: &TestConfig) -> TestResult<()> {
        let mut cache = self.memory_cache.write().unwrap();

        if cache.len() >= self.config.max_memory_entries {
            self.evict_lru_entry(&mut cache);
        }

        let entry = CacheEntry::new(config.clone(), Some(self.config.memory_ttl));
        cache.insert(key.to_string(), entry);

        {
            let mut stats = self.stats.write().unwrap();
            stats.entries = cache.len();
        }

        Ok(())
    }

    async fn get_from_disk(&self, key: &str) -> TestResult<Option<TestConfig>> {
        let cache_file = self.get_disk_cache_path(key);

        if !cache_file.exists() {
            return Ok(None);
        }

        let data = fs::read(&cache_file).await?;
        let entry: CacheEntry<TestConfig> = bincode::deserialize(&data)?;

        if entry.is_expired() {
            let _ = fs::remove_file(&cache_file).await;
            return Ok(None);
        }

        {
            let mut stats = self.stats.write().unwrap();
            stats.hits += 1;
            stats.update_hit_rate();
        }

        Ok(Some(entry.data))
    }

    async fn put_to_disk(&self, key: &str, config: &TestConfig) -> TestResult<()> {
        let cache_file = self.get_disk_cache_path(key);
        let entry = CacheEntry::new(config.clone(), Some(self.config.disk_ttl));
        let data = bincode::serialize(&entry)?;
        fs::write(&cache_file, data).await?;
        Ok(())
    }

    fn get_disk_cache_path(&self, key: &str) -> PathBuf {
        let filename = format!("{}.cache", self.hash_key(key));
        self.config.disk_cache_dir.join(filename)
    }

    fn hash_key(&self, key: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn evict_lru_entry(&self, cache: &mut HashMap<String, CacheEntry<TestConfig>>) {
        if cache.is_empty() {
            return;
        }

        let lru_key = cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone());

        if let Some(key) = lru_key {
            cache.remove(&key);
        }
    }

    async fn start_cleanup_task(&mut self) {
        let memory_cache = Arc::clone(&self.memory_cache);
        let config = self.config.clone();
        let stats = Arc::clone(&self.stats);
        let notify = Arc::clone(&self.invalidation_notify);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.cleanup_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        Self::cleanup_expired_entries(&memory_cache, &stats).await;
                    }
                    _ = notify.notified() => {
                        Self::cleanup_expired_entries(&memory_cache, &stats).await;
                    }
                }
            }
        });

        self.cleanup_handle = Some(handle);
    }

    async fn cleanup_expired_entries(
        memory_cache: &Arc<RwLock<HashMap<String, CacheEntry<TestConfig>>>>,
        stats: &Arc<RwLock<CacheStats>>,
    ) {
        let mut cache = memory_cache.write().unwrap();
        let initial_count = cache.len();

        cache.retain(|_, entry| !entry.is_expired());

        let final_count = cache.len();

        if initial_count != final_count {
            let mut stats = stats.write().unwrap();
            stats.entries = final_count;
        }
    }

    pub async fn invalidate(&self, key: &str) -> TestResult<()> {
        {
            let mut cache = self.memory_cache.write().unwrap();
            cache.remove(key);

            let mut stats = self.stats.write().unwrap();
            stats.entries = cache.len();
        }

        if self.config.enable_disk_cache {
            let cache_file = self.get_disk_cache_path(key);
            if cache_file.exists() {
                fs::remove_file(&cache_file).await?;
            }
        }

        Ok(())
    }

    pub fn get_stats(&self) -> CacheStats {
        let stats = self.stats.read().unwrap();
        stats.clone()
    }

    pub async fn stop(&mut self) {
        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
        }
    }
}

impl Drop for TestConfigCache {
    fn drop(&mut self) {
        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_cache_config() -> CacheConfig {
        let temp_dir = TempDir::new().unwrap();
        CacheConfig {
            max_memory_entries: 10,
            memory_ttl: Duration::from_millis(100),
            enable_disk_cache: true,
            disk_cache_dir: temp_dir.path().to_path_buf(),
            disk_ttl: Duration::from_millis(200),
            cleanup_interval: Duration::from_millis(50),
        }
    }

    #[tokio::test]
    async fn test_memory_cache_basic_operations() {
        let config = create_test_cache_config();
        let mut cache = TestConfigCache::new(config).unwrap();
        cache.start().await.unwrap();

        let test_config = TestConfig {
            name: "test".to_string(),
            value: 42,
        };
        let key = "test_key";

        // 测试存储和获取
        cache.put(key, &test_config).await.unwrap();
        let retrieved = cache.get(key).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), test_config);

        // 测试缓存失效
        cache.invalidate(key).await.unwrap();
        let retrieved = cache.get(key).await.unwrap();
        assert!(retrieved.is_none());

        cache.stop().await;
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let mut config = create_test_cache_config();
        // 关闭磁盘缓存，避免从磁盘回填导致过期用例失败
        config.enable_disk_cache = false;
        let mut cache = TestConfigCache::new(config).unwrap();
        cache.start().await.unwrap();

        let test_config = TestConfig {
            name: "test".to_string(),
            value: 42,
        };
        let key = "test_key";

        // 存储配置
        cache.put(key, &test_config).await.unwrap();

        // 等待过期
        tokio::time::sleep(Duration::from_millis(150)).await;

        // 应该已经过期
        let retrieved = cache.get(key).await.unwrap();
        assert!(retrieved.is_none());

        cache.stop().await;
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let config = create_test_cache_config();
        let mut cache = TestConfigCache::new(config).unwrap();
        cache.start().await.unwrap();

        let test_config = TestConfig {
            name: "test".to_string(),
            value: 42,
        };
        let key = "test_key";

        // 初始统计
        let stats = cache.get_stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);

        // 缓存未命中
        cache.get(key).await.unwrap();
        let stats = cache.get_stats();
        assert_eq!(stats.misses, 1);

        // 存储并命中
        cache.put(key, &test_config).await.unwrap();
        cache.get(key).await.unwrap();
        let stats = cache.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);

        cache.stop().await;
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let mut config = create_test_cache_config();
        config.max_memory_entries = 2; // 限制为 2 个条目
                                       // 关闭磁盘缓存，确保被驱逐的条目不会从磁盘回填
        config.enable_disk_cache = false;

        let mut cache = TestConfigCache::new(config).unwrap();
        cache.start().await.unwrap();

        let test_config = TestConfig {
            name: "test".to_string(),
            value: 42,
        };

        // 添加 3 个条目，应该触发 LRU 驱逐
        cache.put("key1", &test_config).await.unwrap();
        cache.put("key2", &test_config).await.unwrap();
        cache.put("key3", &test_config).await.unwrap();

        // key1 应该被驱逐
        let retrieved = cache.get("key1").await.unwrap();
        assert!(retrieved.is_none());

        // key2 和 key3 应该还在
        let retrieved = cache.get("key2").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = cache.get("key3").await.unwrap();
        assert!(retrieved.is_some());

        cache.stop().await;
    }

    #[tokio::test]
    async fn test_disk_cache() {
        let config = create_test_cache_config();
        let mut cache = TestConfigCache::new(config).unwrap();
        cache.start().await.unwrap();

        let test_config = TestConfig {
            name: "test".to_string(),
            value: 42,
        };
        let key = "test_key";

        // 存储到缓存
        cache.put(key, &test_config).await.unwrap();

        // 清空内存缓存
        {
            let mut memory_cache = cache.memory_cache.write().unwrap();
            memory_cache.clear();
        }

        // 应该能从磁盘缓存获取
        let retrieved = cache.get(key).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), test_config);

        cache.stop().await;
    }
}
