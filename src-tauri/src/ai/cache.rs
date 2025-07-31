/*!
 * 简化的AI缓存管理
 *
 * 使用基础的TTL缓存，移除复杂的缓存清理策略
 */

use crate::ai::{AIRequest, AIResponse};
use crate::utils::error::AppResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// 缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub response: AIResponse,
    pub created_at: DateTime<Utc>,
    pub ttl_seconds: u64,
    pub hits: u64,
}

impl CacheEntry {
    pub fn new(response: AIResponse, ttl_seconds: u64) -> Self {
        Self {
            response,
            created_at: Utc::now(),
            ttl_seconds,
            hits: 0,
        }
    }

    pub fn is_expired(&self) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.created_at)
            .num_seconds() as u64;
        elapsed > self.ttl_seconds
    }

    pub fn increment_hits(&mut self) {
        self.hits += 1;
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
    pub memory_usage_estimate: usize,
}

/// 简化的缓存管理器
pub struct SimpleCache {
    pub(crate) entries: HashMap<String, CacheEntry>,
    default_ttl: u64,
    max_entries: usize,
    pub(crate) hit_count: u64,
    pub(crate) miss_count: u64,
    last_cleanup: Instant,
    cleanup_interval: Duration,
}

impl Default for SimpleCache {
    fn default() -> Self {
        Self::new(3600, 1000) // 默认1小时TTL，最大1000条目
    }
}

impl SimpleCache {
    /// 创建新的缓存管理器
    pub fn new(default_ttl: u64, max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl,
            max_entries,
            hit_count: 0,
            miss_count: 0,
            last_cleanup: Instant::now(),
            cleanup_interval: Duration::from_secs(300), // 5分钟清理一次
        }
    }

    /// 生成缓存键
    pub fn generate_cache_key(&self, request: &AIRequest, model_id: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        request.request_type.hash(&mut hasher);
        request.content.hash(&mut hasher);
        model_id.hash(&mut hasher);

        // 包含请求选项
        if let Some(options) = &request.options {
            options.max_tokens.hash(&mut hasher);
            if let Some(temp) = options.temperature {
                ((temp * 1000.0) as u32).hash(&mut hasher); // 避免浮点数精度问题
            }
        }

        format!("{:x}", hasher.finish())
    }

    /// 获取缓存响应
    pub fn get(&mut self, request: &AIRequest, model_id: &str) -> Option<AIResponse> {
        self.cleanup_if_needed();

        let key = self.generate_cache_key(request, model_id);

        if let Some(entry) = self.entries.get_mut(&key) {
            if entry.is_expired() {
                self.entries.remove(&key);
                self.miss_count += 1;
                None
            } else {
                entry.increment_hits();
                self.hit_count += 1;
                Some(entry.response.clone())
            }
        } else {
            self.miss_count += 1;
            None
        }
    }

    /// 存储响应到缓存
    pub fn put(
        &mut self,
        request: &AIRequest,
        model_id: &str,
        response: AIResponse,
    ) -> AppResult<()> {
        self.cleanup_if_needed();

        // 如果缓存已满，移除最旧的条目
        if self.entries.len() >= self.max_entries {
            self.evict_oldest();
        }

        let key = self.generate_cache_key(request, model_id);
        let entry = CacheEntry::new(response, self.default_ttl);

        self.entries.insert(key, entry);
        Ok(())
    }

    /// 清空缓存
    pub fn clear(&mut self) -> AppResult<()> {
        self.entries.clear();
        self.hit_count = 0;
        self.miss_count = 0;
        Ok(())
    }

    /// 获取缓存统计信息
    pub fn get_stats(&self) -> AppResult<CacheStats> {
        let total_requests = self.hit_count + self.miss_count;
        let hit_rate = if total_requests > 0 {
            self.hit_count as f64 / total_requests as f64
        } else {
            0.0
        };

        // 估算内存使用量（粗略估计）
        let memory_usage_estimate = self.entries.len() * 1024; // 假设每个条目平均1KB

        Ok(CacheStats {
            total_entries: self.entries.len(),
            hit_count: self.hit_count,
            miss_count: self.miss_count,
            hit_rate,
            memory_usage_estimate,
        })
    }

    /// 清理过期条目
    pub fn cleanup_expired(&mut self) -> AppResult<usize> {
        let initial_count = self.entries.len();

        self.entries.retain(|_, entry| !entry.is_expired());

        let removed_count = initial_count - self.entries.len();
        Ok(removed_count)
    }

    /// 如果需要则执行清理
    fn cleanup_if_needed(&mut self) {
        if self.last_cleanup.elapsed() >= self.cleanup_interval {
            let _ = self.cleanup_expired();
            self.last_cleanup = Instant::now();
        }
    }

    /// 移除最旧的条目（LRU策略）
    /// 优先移除命中次数最少且最旧的条目
    fn evict_oldest(&mut self) {
        if let Some((oldest_key, _)) = self
            .entries
            .iter()
            .min_by(|(_, a), (_, b)| {
                // 首先按命中次数排序，然后按创建时间排序
                a.hits.cmp(&b.hits).then(a.created_at.cmp(&b.created_at))
            })
            .map(|(k, v)| (k.clone(), v.clone()))
        {
            self.entries.remove(&oldest_key);
        }
    }

    /// 设置默认TTL
    pub fn set_default_ttl(&mut self, ttl_seconds: u64) {
        self.default_ttl = ttl_seconds;
    }

    /// 设置最大条目数
    pub fn set_max_entries(&mut self, max_entries: usize) {
        self.max_entries = max_entries;

        // 如果当前条目数超过新的限制，清理多余的条目
        while self.entries.len() > max_entries {
            self.evict_oldest();
        }
    }

    /// 获取缓存大小
    pub fn size(&self) -> usize {
        self.entries.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 为了向后兼容性保留的类型别名和结构
pub type CacheManager = SimpleCache;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AICacheStats {
    pub total_entries: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
    pub memory_usage_estimate: usize,
}

impl From<CacheStats> for AICacheStats {
    fn from(stats: CacheStats) -> Self {
        Self {
            total_entries: stats.total_entries,
            hit_count: stats.hit_count,
            miss_count: stats.miss_count,
            hit_rate: stats.hit_rate,
            memory_usage_estimate: stats.memory_usage_estimate,
        }
    }
}

/// 缓存监控统计（简化版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMonitorStats {
    pub hit_rate: f64,
    pub total_requests: u64,
    pub cache_size: usize,
    pub memory_usage: usize,
}

/// 缓存策略枚举（简化版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheStrategy {
    TimeBasedTTL,
    FrequencyBased,
    ContentSimilarity,
    Hybrid,
}

impl SimpleCache {
    /// 获取监控统计信息
    pub fn get_monitor_stats(&self) -> AppResult<CacheMonitorStats> {
        let total_requests = self.hit_count + self.miss_count;
        let hit_rate = if total_requests > 0 {
            self.hit_count as f64 / total_requests as f64
        } else {
            0.0
        };

        // 更精确的内存使用估算
        let memory_usage = self
            .entries
            .iter()
            .map(|(key, entry)| {
                key.len() + entry.response.content.len() + 256 // 基础结构体大小估算
            })
            .sum::<usize>();

        Ok(CacheMonitorStats {
            hit_rate,
            total_requests,
            cache_size: self.entries.len(),
            memory_usage,
        })
    }

    /// 重置监控统计
    pub fn reset_monitor(&mut self) -> AppResult<()> {
        self.hit_count = 0;
        self.miss_count = 0;
        Ok(())
    }

    /// 智能清理（移除使用频率低的条目）
    /// 优化的LRU策略：优先移除过期条目，然后移除命中次数最少且最旧的条目
    pub fn smart_cleanup(&mut self) -> AppResult<usize> {
        let initial_count = self.entries.len();

        // 第一步：移除过期条目
        self.entries.retain(|_, entry| !entry.is_expired());

        // 第二步：如果仍然超过容量的75%，进行LRU清理
        if self.entries.len() > self.max_entries * 3 / 4 {
            let target_size = self.max_entries * 3 / 4;
            let remove_count = self.entries.len() - target_size;

            // 按照LRU策略排序：命中次数少的优先，然后是创建时间早的
            let mut entries: Vec<_> = self.entries.iter().collect();
            entries.sort_by(|(_, a), (_, b)| {
                a.hits.cmp(&b.hits).then(a.created_at.cmp(&b.created_at))
            });

            // 移除最不常用的条目
            let keys_to_remove: Vec<String> = entries
                .iter()
                .take(remove_count)
                .map(|(k, _)| (*k).clone())
                .collect();

            for key in keys_to_remove {
                self.entries.remove(&key);
            }
        }

        let removed_count = initial_count - self.entries.len();
        Ok(removed_count)
    }

    /// 设置缓存策略（简化实现）
    pub fn set_strategy(&mut self, _strategy: CacheStrategy) {
        // 简化实现，所有策略都使用TTL
    }

    /// 获取缓存策略
    pub fn get_strategy(&self) -> CacheStrategy {
        CacheStrategy::TimeBasedTTL
    }

    /// 获取详细的缓存性能指标
    pub fn get_performance_metrics(&self) -> AppResult<CachePerformanceMetrics> {
        let total_requests = self.hit_count + self.miss_count;
        let hit_rate = if total_requests > 0 {
            self.hit_count as f64 / total_requests as f64
        } else {
            0.0
        };

        // 计算平均命中次数
        let avg_hits = if !self.entries.is_empty() {
            self.entries.values().map(|entry| entry.hits).sum::<u64>() as f64
                / self.entries.len() as f64
        } else {
            0.0
        };

        // 计算过期条目数量
        let expired_count = self
            .entries
            .values()
            .filter(|entry| entry.is_expired())
            .count();

        // 计算内存使用效率
        let memory_usage = self
            .entries
            .iter()
            .map(|(key, entry)| key.len() + entry.response.content.len() + 256)
            .sum::<usize>();

        let memory_efficiency = if memory_usage > 0 {
            (self.hit_count as f64) / (memory_usage as f64 / 1024.0) // 每KB的命中次数
        } else {
            0.0
        };

        Ok(CachePerformanceMetrics {
            hit_rate,
            total_requests,
            cache_size: self.entries.len(),
            max_capacity: self.max_entries,
            memory_usage,
            avg_hits_per_entry: avg_hits,
            expired_entries: expired_count,
            memory_efficiency,
            default_ttl: self.default_ttl,
        })
    }

    /// 优化缓存配置建议
    pub fn get_optimization_suggestions(&self) -> Vec<String> {
        let mut suggestions = Vec::new();

        let total_requests = self.hit_count + self.miss_count;
        let hit_rate = if total_requests > 0 {
            self.hit_count as f64 / total_requests as f64
        } else {
            0.0
        };

        // 命中率建议
        if hit_rate < 0.3 {
            suggestions.push("缓存命中率较低，考虑增加TTL时间或缓存容量".to_string());
        } else if hit_rate > 0.8 {
            suggestions.push("缓存命中率很高，可以考虑减少TTL时间以节省内存".to_string());
        }

        // 容量建议
        let usage_ratio = self.entries.len() as f64 / self.max_entries as f64;
        if usage_ratio > 0.9 {
            suggestions.push("缓存使用率接近满载，建议增加最大容量".to_string());
        } else if usage_ratio < 0.3 {
            suggestions.push("缓存使用率较低，可以考虑减少最大容量以节省内存".to_string());
        }

        // 过期条目建议
        let expired_count = self
            .entries
            .values()
            .filter(|entry| entry.is_expired())
            .count();
        if expired_count > self.entries.len() / 4 {
            suggestions.push("存在较多过期条目，建议执行清理操作".to_string());
        }

        if suggestions.is_empty() {
            suggestions.push("缓存配置良好，无需调整".to_string());
        }

        suggestions
    }
}

/// 缓存性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePerformanceMetrics {
    pub hit_rate: f64,
    pub total_requests: u64,
    pub cache_size: usize,
    pub max_capacity: usize,
    pub memory_usage: usize,
    pub avg_hits_per_entry: f64,
    pub expired_entries: usize,
    pub memory_efficiency: f64, // 每KB内存的命中次数
    pub default_ttl: u64,
}
