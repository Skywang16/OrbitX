/*!
 * 配置缓存系统
 *
 * 提供多级缓存机制，包括内存缓存和可选的磁盘缓存，
 * 支持基于时间和事件的缓存失效策略。
 */

use crate::{config::AppConfig, utils::error::AppResult};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tokio::fs;
use tokio::sync::Notify;

/// 缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    /// 缓存的数据
    pub data: T,
    /// 创建时间
    pub created_at: SystemTime,
    /// 最后访问时间
    pub last_accessed: SystemTime,
    /// 访问次数
    pub access_count: u64,
    /// 过期时间
    pub expires_at: Option<SystemTime>,
    /// 版本号
    pub version: u64,
}

impl<T> CacheEntry<T> {
    /// 创建新的缓存条目
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

    /// 检查缓存条目是否过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }

    /// 更新访问信息
    pub fn touch(&mut self) {
        self.last_accessed = SystemTime::now();
        self.access_count += 1;
    }

    /// 更新数据和版本
    pub fn update(&mut self, data: T, ttl: Option<Duration>) {
        self.data = data;
        self.version += 1;
        self.last_accessed = SystemTime::now();
        self.access_count += 1;

        if let Some(duration) = ttl {
            self.expires_at = Some(SystemTime::now() + duration);
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// 缓存命中次数
    pub hits: u64,
    /// 缓存未命中次数
    pub misses: u64,
    /// 缓存条目数量
    pub entries: usize,
    /// 内存使用量（字节）
    pub memory_usage: usize,
    /// 磁盘使用量（字节）
    pub disk_usage: usize,
    /// 命中率
    pub hit_rate: f64,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheStats {
    /// 创建新的统计信息
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            entries: 0,
            memory_usage: 0,
            disk_usage: 0,
            hit_rate: 0.0,
        }
    }

    /// 更新命中率
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
    /// 内存缓存最大条目数
    pub max_memory_entries: usize,
    /// 内存缓存 TTL
    pub memory_ttl: Duration,
    /// 启用磁盘缓存
    pub enable_disk_cache: bool,
    /// 磁盘缓存目录
    pub disk_cache_dir: PathBuf,
    /// 磁盘缓存 TTL
    pub disk_ttl: Duration,
    /// 磁盘缓存最大大小（字节）
    pub max_disk_size: usize,
    /// 清理间隔
    pub cleanup_interval: Duration,
    /// 启用压缩
    pub enable_compression: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_memory_entries: 1000,
            memory_ttl: Duration::from_secs(300), // 5 minutes
            enable_disk_cache: true,
            disk_cache_dir: PathBuf::from("cache"),
            disk_ttl: Duration::from_secs(3600),       // 1 hour
            max_disk_size: 100 * 1024 * 1024,          // 100MB
            cleanup_interval: Duration::from_secs(60), // 1 minute
            enable_compression: true,
        }
    }
}

/// 缓存失效策略
#[derive(Debug, Clone)]
pub enum InvalidationStrategy {
    /// 基于时间的失效
    TimeBasedTTL(Duration),
    /// 基于访问时间的失效
    LastAccessTTL(Duration),
    /// 基于事件的失效
    EventBased(Vec<String>),
    /// LRU 策略
    Lru(usize),
    /// 手动失效
    Manual,
}

/// 配置缓存系统
pub struct ConfigCache {
    /// 内存缓存
    memory_cache: Arc<RwLock<HashMap<String, CacheEntry<AppConfig>>>>,
    /// 缓存配置
    config: CacheConfig,
    /// 缓存统计
    stats: Arc<RwLock<CacheStats>>,
    /// 失效通知
    invalidation_notify: Arc<Notify>,
    /// 后台任务句柄
    cleanup_handle: Option<tokio::task::JoinHandle<()>>,
}
impl ConfigCache {
    /// 创建新的配置缓存实例
    pub fn new(config: CacheConfig) -> AppResult<Self> {
        let cache = Self {
            memory_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(CacheStats::new())),
            invalidation_notify: Arc::new(Notify::new()),
            cleanup_handle: None,
        };

        Ok(cache)
    }

    /// 启动缓存系统
    pub async fn start(&mut self) -> AppResult<()> {
        // 创建磁盘缓存目录
        if self.config.enable_disk_cache {
            fs::create_dir_all(&self.config.disk_cache_dir)
                .await
                .map_err(|e| anyhow!(e.to_string()))?;
        }

        // 启动清理任务
        self.start_cleanup_task().await;

        Ok(())
    }

    /// 获取缓存的配置
    pub async fn get(&self, key: &str) -> AppResult<Option<AppConfig>> {
        // 首先尝试从内存缓存获取
        if let Some(config) = self.get_from_memory(key).await? {
            return Ok(Some(config));
        }

        // 如果内存缓存未命中，尝试从磁盘缓存获取
        if self.config.enable_disk_cache {
            if let Some(config) = self.get_from_disk(key).await? {
                // 将磁盘缓存的数据加载到内存缓存
                self.put_to_memory(key, &config).await?;
                return Ok(Some(config));
            }
        }

        // 更新统计信息
        {
            let mut stats = self.stats.write().unwrap();
            stats.misses += 1;
            stats.update_hit_rate();
        }

        Ok(None)
    }

    /// 存储配置到缓存
    pub async fn put(&self, key: &str, config: &AppConfig) -> AppResult<()> {
        // 存储到内存缓存
        self.put_to_memory(key, config).await?;

        // 如果启用磁盘缓存，也存储到磁盘
        if self.config.enable_disk_cache {
            self.put_to_disk(key, config).await?;
        }

        Ok(())
    }

    /// 从内存缓存获取配置
    async fn get_from_memory(&self, key: &str) -> AppResult<Option<AppConfig>> {
        let mut cache = self.memory_cache.write().unwrap();

        if let Some(entry) = cache.get_mut(key) {
            // 检查是否过期
            if entry.is_expired() {
                cache.remove(key);
                return Ok(None);
            }

            // 更新访问信息
            entry.touch();

            // 更新统计信息
            {
                let mut stats = self.stats.write().unwrap();
                stats.hits += 1;
                stats.update_hit_rate();
            }

            return Ok(Some(entry.data.clone()));
        }

        Ok(None)
    }

    /// 存储配置到内存缓存
    async fn put_to_memory(&self, key: &str, config: &AppConfig) -> AppResult<()> {
        let mut cache = self.memory_cache.write().unwrap();

        // 检查缓存大小限制
        if cache.len() >= self.config.max_memory_entries {
            self.evict_lru_entry(&mut cache);
        }

        // 创建缓存条目
        let entry = CacheEntry::new(config.clone(), Some(self.config.memory_ttl));
        cache.insert(key.to_string(), entry);

        // 更新统计信息
        {
            let mut stats = self.stats.write().unwrap();
            stats.entries = cache.len();
        }

        Ok(())
    }

    /// 从磁盘缓存获取配置
    async fn get_from_disk(&self, key: &str) -> AppResult<Option<AppConfig>> {
        let cache_file = self.get_disk_cache_path(key);

        if !cache_file.exists() {
            return Ok(None);
        }

        // 读取缓存文件
        let data = fs::read(&cache_file)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        // 解压缩（如果启用）
        let data = if self.config.enable_compression {
            self.decompress_data(&data)?
        } else {
            data
        };

        // 反序列化缓存条目
        let entry: CacheEntry<AppConfig> =
            bincode::deserialize(&data).map_err(|e| anyhow!(e.to_string()))?;

        // 检查是否过期
        if entry.is_expired() {
            // 删除过期的缓存文件
            let _ = fs::remove_file(&cache_file).await;
            return Ok(None);
        }

        // 更新统计信息
        {
            let mut stats = self.stats.write().unwrap();
            stats.hits += 1;
            stats.update_hit_rate();
        }

        Ok(Some(entry.data))
    }

    /// 存储配置到磁盘缓存
    async fn put_to_disk(&self, key: &str, config: &AppConfig) -> AppResult<()> {
        let cache_file = self.get_disk_cache_path(key);

        // 创建缓存条目
        let entry = CacheEntry::new(config.clone(), Some(self.config.disk_ttl));

        // 序列化缓存条目
        let data = bincode::serialize(&entry).map_err(|e| anyhow!(e.to_string()))?;

        // 压缩（如果启用）
        let data = if self.config.enable_compression {
            self.compress_data(&data)?
        } else {
            data
        };

        // 写入缓存文件
        fs::write(&cache_file, data)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        Ok(())
    }

    /// 获取磁盘缓存文件路径
    fn get_disk_cache_path(&self, key: &str) -> PathBuf {
        let filename = format!("{}.cache", self.hash_key(key));
        self.config.disk_cache_dir.join(filename)
    }

    /// 对键进行哈希处理
    fn hash_key(&self, key: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// 压缩数据
    fn compress_data(&self, data: &[u8]) -> AppResult<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(data)
            .map_err(|e| anyhow!(e.to_string()))?;

        encoder.finish().map_err(|e| anyhow!(e.to_string()))
    }

    /// 解压缩数据
    fn decompress_data(&self, data: &[u8]) -> AppResult<Vec<u8>> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| anyhow!(e.to_string()))?;

        Ok(decompressed)
    }

    /// 驱逐 LRU 条目
    fn evict_lru_entry(&self, cache: &mut HashMap<String, CacheEntry<AppConfig>>) {
        if cache.is_empty() {
            return;
        }

        // 找到最久未访问的条目
        let lru_key = cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone());

        if let Some(key) = lru_key {
            cache.remove(&key);
        }
    }

    /// 启动清理任务
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
                        Self::cleanup_expired_entries(&memory_cache, &config, &stats).await;
                        if config.enable_disk_cache {
                            Self::cleanup_disk_cache(&config).await;
                        }
                    }
                    _ = notify.notified() => {
                        // 处理失效通知
                        Self::cleanup_expired_entries(&memory_cache, &config, &stats).await;
                    }
                }
            }
        });

        self.cleanup_handle = Some(handle);
    }

    /// 清理过期的内存缓存条目
    async fn cleanup_expired_entries(
        memory_cache: &Arc<RwLock<HashMap<String, CacheEntry<AppConfig>>>>,
        _config: &CacheConfig,
        stats: &Arc<RwLock<CacheStats>>,
    ) {
        let mut cache = memory_cache.write().unwrap();
        let initial_count = cache.len();

        cache.retain(|_, entry| !entry.is_expired());

        let final_count = cache.len();
        let removed_count = initial_count - final_count;

        if removed_count > 0 {
            let mut stats = stats.write().unwrap();
            stats.entries = final_count;
        }
    }

    /// 清理磁盘缓存
    async fn cleanup_disk_cache(config: &CacheConfig) {
        if let Ok(mut entries) = fs::read_dir(&config.disk_cache_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("cache") {
                    // 检查文件修改时间
                    if let Ok(metadata) = entry.metadata().await {
                        if let Ok(modified) = metadata.modified() {
                            let age = SystemTime::now()
                                .duration_since(modified)
                                .unwrap_or(Duration::ZERO);

                            if age > config.disk_ttl {
                                let _ = fs::remove_file(&path).await;
                            }
                        }
                    }
                }
            }
        }
    }

    /// 使缓存失效
    pub async fn invalidate(&self, key: &str) -> AppResult<()> {
        // 从内存缓存中移除
        {
            let mut cache = self.memory_cache.write().unwrap();
            cache.remove(key);

            let mut stats = self.stats.write().unwrap();
            stats.entries = cache.len();
        }

        // 从磁盘缓存中移除
        if self.config.enable_disk_cache {
            let cache_file = self.get_disk_cache_path(key);
            if cache_file.exists() {
                fs::remove_file(&cache_file)
                    .await
                    .map_err(|e| anyhow!(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// 清空所有缓存
    pub async fn clear(&self) -> AppResult<()> {
        // 清空内存缓存
        {
            let mut cache = self.memory_cache.write().unwrap();
            cache.clear();

            let mut stats = self.stats.write().unwrap();
            stats.entries = 0;
            stats.hits = 0;
            stats.misses = 0;
            stats.hit_rate = 0.0;
        }

        // 清空磁盘缓存
        if self.config.enable_disk_cache {
            if let Ok(mut entries) = fs::read_dir(&self.config.disk_cache_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("cache") {
                        let _ = fs::remove_file(&path).await;
                    }
                }
            }
        }

        Ok(())
    }

    /// 获取缓存统计信息
    pub fn get_stats(&self) -> CacheStats {
        let stats = self.stats.read().unwrap();
        stats.clone()
    }

    /// 预热缓存
    pub async fn warm_up(&self, configs: Vec<(String, AppConfig)>) -> AppResult<()> {
        for (key, config) in configs {
            self.put(&key, &config).await?;
        }
        Ok(())
    }

    /// 检查缓存健康状态
    pub async fn health_check(&self) -> AppResult<bool> {
        // 检查内存缓存
        let memory_ok = {
            let cache = self.memory_cache.read().unwrap();
            cache.len() <= self.config.max_memory_entries
        };

        // 检查磁盘缓存
        let disk_ok = if self.config.enable_disk_cache {
            self.config.disk_cache_dir.exists()
        } else {
            true
        };

        Ok(memory_ok && disk_ok)
    }

    /// 获取缓存大小信息
    pub async fn get_cache_size(&self) -> AppResult<(usize, usize)> {
        // 内存缓存大小
        let memory_size = {
            let cache = self.memory_cache.read().unwrap();
            cache.len()
        };

        // 磁盘缓存大小
        let mut disk_size = 0;
        if self.config.enable_disk_cache {
            if let Ok(mut entries) = fs::read_dir(&self.config.disk_cache_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Ok(metadata) = entry.metadata().await {
                        disk_size += metadata.len() as usize;
                    }
                }
            }
        }

        Ok((memory_size, disk_size))
    }

    /// 触发缓存失效通知
    pub fn notify_invalidation(&self) {
        self.invalidation_notify.notify_waiters();
    }

    /// 停止缓存系统
    pub async fn stop(&mut self) {
        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
        }
    }
}

impl Drop for ConfigCache {
    fn drop(&mut self) {
        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
        }
    }
}

/// 缓存管理器
pub struct CacheManager {
    /// 配置缓存
    config_cache: ConfigCache,
    /// 失效策略
    invalidation_strategies: Vec<InvalidationStrategy>,
}

impl CacheManager {
    /// 创建新的缓存管理器
    pub fn new(config: CacheConfig) -> AppResult<Self> {
        let config_cache = ConfigCache::new(config)?;

        Ok(Self {
            config_cache,
            invalidation_strategies: vec![
                InvalidationStrategy::TimeBasedTTL(Duration::from_secs(300)),
                InvalidationStrategy::Lru(1000),
            ],
        })
    }

    /// 启动缓存管理器
    pub async fn start(&mut self) -> AppResult<()> {
        self.config_cache.start().await
    }

    /// 获取配置
    pub async fn get_config(&self, key: &str) -> AppResult<Option<AppConfig>> {
        self.config_cache.get(key).await
    }

    /// 存储配置
    pub async fn put_config(&self, key: &str, config: &AppConfig) -> AppResult<()> {
        self.config_cache.put(key, config).await
    }

    /// 使配置失效
    pub async fn invalidate_config(&self, key: &str) -> AppResult<()> {
        self.config_cache.invalidate(key).await
    }

    /// 清空所有缓存
    pub async fn clear_all(&self) -> AppResult<()> {
        self.config_cache.clear().await
    }

    /// 获取缓存统计信息
    pub fn get_stats(&self) -> CacheStats {
        self.config_cache.get_stats()
    }

    /// 添加失效策略
    pub fn add_invalidation_strategy(&mut self, strategy: InvalidationStrategy) {
        self.invalidation_strategies.push(strategy);
    }

    /// 应用失效策略
    pub async fn apply_invalidation_strategies(&self) -> AppResult<()> {
        for strategy in &self.invalidation_strategies {
            match strategy {
                InvalidationStrategy::TimeBasedTTL(_) => {
                    // TTL 策略由清理任务自动处理
                }
                InvalidationStrategy::LastAccessTTL(_) => {
                    // 访问时间策略由清理任务自动处理
                }
                InvalidationStrategy::EventBased(_events) => {
                    // 事件策略需要外部触发
                    self.config_cache.notify_invalidation();
                }
                InvalidationStrategy::Lru(_) => {
                    // LRU 策略在存储时自动处理
                }
                InvalidationStrategy::Manual => {
                    // 手动策略不需要自动处理
                }
            }
        }
        Ok(())
    }

    /// 停止缓存管理器
    pub async fn stop(&mut self) {
        self.config_cache.stop().await;
    }
}
