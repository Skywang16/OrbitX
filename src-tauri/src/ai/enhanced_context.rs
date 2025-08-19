/*!
 * 智能上下文管理系统 - 基于Claude Code逆向分析优化
 *
 * 核心特性：
 * 1. 92%触发阈值的智能压缩 (参考Claude Code实现)
 * 2. 30%上下文保留率与重要性评分
 * 3. 高效KV缓存与语义相似性匹配
 * 4. 分层压缩策略：轻量级 -> 深度压缩
 */

use crate::ai::types::Message;
use crate::storage::repositories::RepositoryManager;
use crate::utils::error::AppResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

// ============= 配置层 =============

/// 上下文管理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// 最大token数量
    pub max_tokens: usize,
    /// 压缩触发阈值(0.0-1.0)
    pub compress_threshold: f32,
    /// 保留最近消息数
    pub keep_recent: usize,
    /// 保留重要消息数
    pub keep_important: usize,
    /// 最小压缩批次大小
    pub min_compress_batch: usize,
    /// 摘要窗口大小
    pub summary_window_size: usize,
    /// 重要性阈值
    pub importance_threshold: f32,
    /// KV缓存配置
    pub kv_cache: KVCacheConfig,
}

/// KV缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KVCacheConfig {
    /// 是否启用KV缓存
    pub enabled: bool,
    /// 缓存TTL（秒）
    pub ttl_seconds: u64,
    /// 最大缓存条目数
    pub max_entries: usize,
    /// 稳定前缀最大长度
    pub stable_prefix_max_tokens: usize,
}

impl Default for KVCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_seconds: 3600, // 1小时
            max_entries: 100,
            stable_prefix_max_tokens: 1000,
        }
    }
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_tokens: 100000,   
            compress_threshold: 0.92, // 92%触发阈值 (Claude Code标准)
            keep_recent: 12,      // 保留最近12条消息
            keep_important: 8,    // 重要消息数量优化
            min_compress_batch: 3, // 减少最小批次
            summary_window_size: 8, // 优化窗口大小
            importance_threshold: 0.7, // 提高重要性阈值
            kv_cache: KVCacheConfig::default(),
        }
    }
}

// ============= KV缓存层 =============

/// KV缓存项
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// 缓存的上下文内容
    pub content: String,
    /// 消息列表的哈希值
    pub messages_hash: u64,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后访问时间
    pub last_accessed: DateTime<Utc>,
    /// 命中次数
    pub hit_count: u64,
    /// 稳定前缀长度
    pub stable_prefix_len: usize,
    /// 语义哈希（基于消息语义而非简单文本）
    pub semantic_hash: u64,
    /// 访问频率权重
    pub access_weight: f64,
}

/// KV缓存管理器 - 基于Manus原理
pub struct KVCache {
    /// 缓存存储
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    /// 配置
    config: KVCacheConfig,
}

impl KVCache {
    pub fn new(config: KVCacheConfig) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// 生成缓存键 - 基于对话ID和前缀哈希
    fn generate_cache_key(&self, conv_id: i64, prefix_hash: u64) -> String {
        format!("ctx_{}_{}", conv_id, prefix_hash)
    }

    /// 计算消息列表的哈希值（结构化哈希）
    fn hash_messages(&self, msgs: &[Message]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for msg in msgs {
            msg.id.hash(&mut hasher);
            msg.role.hash(&mut hasher);
            // 对内容进行规范化处理，忽略空白字符差异
            msg.content.trim().hash(&mut hasher);
            // 哈希工具调用信息
            if let Some(ref steps) = msg.steps_json {
                steps.hash(&mut hasher);
            }
        }
        hasher.finish()
    }


    /// 智能提取稳定前缀 - 考虑语义边界
    pub fn extract_stable_prefix(&self, msgs: &[Message]) -> Vec<Message> {
        if msgs.is_empty() {
            return Vec::new();
        }

        let mut stable_msgs = Vec::new();
        let mut token_count = 0;
        let mut last_system_index = None;

        // 首先找到最后一个系统消息位置作为潜在边界
        for (i, msg) in msgs.iter().enumerate() {
            if msg.role == "system" {
                last_system_index = Some(i);
            }
        }

        // 优先保留系统消息和早期对话建立上下文
        for (i, msg) in msgs.iter().enumerate() {
            let msg_tokens = self.estimate_message_tokens(msg);
            
            // 如果超出token限制，检查是否在语义边界
            if token_count + msg_tokens > self.config.stable_prefix_max_tokens {
                // 如果当前位置接近系统消息边界，继续到系统消息
                if let Some(sys_idx) = last_system_index {
                    if i <= sys_idx + 2 && token_count < self.config.stable_prefix_max_tokens * 2 {
                        // 允许适度超出以保持语义完整性
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            stable_msgs.push(msg.clone());
            token_count += msg_tokens;

            // 如果已经包含了系统消息及其后续2-3条消息，考虑停止
            if let Some(sys_idx) = last_system_index {
                if i > sys_idx + 3 && token_count > self.config.stable_prefix_max_tokens / 2 {
                    break;
                }
            }
        }

        debug!(
            "提取稳定前缀: {} -> {} 条消息, {} tokens", 
            msgs.len(), stable_msgs.len(), token_count
        );

        stable_msgs
    }

    /// 更准确的token估算
    fn estimate_message_tokens(&self, msg: &Message) -> usize {
        let content_tokens = msg.content.len() / 3; // 中英混合，3字符约1token
        let steps_tokens = msg.steps_json.as_ref()
            .map(|s| s.len() / 4)
            .unwrap_or(0);
        content_tokens + steps_tokens
    }

    /// 高效缓存获取 - 精确匹配优先
    pub fn get(&self, conv_id: i64, msgs: &[Message]) -> Option<String> {
        if !self.config.enabled || msgs.is_empty() {
            return None;
        }

        let stable_prefix = self.extract_stable_prefix(msgs);
        let prefix_hash = self.hash_messages(&stable_prefix);
        let cache_key = self.generate_cache_key(conv_id, prefix_hash);

        let mut cache = self.cache.lock().ok()?;

        if let Some(entry) = cache.get_mut(&cache_key) {
            if self.is_entry_valid(entry) {
                self.update_entry_access(entry);
                debug!("缓存命中: {} (hits: {})", cache_key, entry.hit_count);
                return Some(entry.content.clone());
            } else {
                cache.remove(&cache_key);
                debug!("清理过期缓存: {}", cache_key);
            }
        }

        debug!("缓存未命中: {}", cache_key);
        None
    }

    /// 检查缓存条目是否有效（TTL和其他条件）
    fn is_entry_valid(&self, entry: &CacheEntry) -> bool {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(entry.created_at).num_seconds();
        elapsed >= 0 && (elapsed as u64) <= self.config.ttl_seconds
    }

    /// 更新缓存条目的访问信息
    fn update_entry_access(&self, entry: &mut CacheEntry) {
        let now = Utc::now();
        entry.hit_count += 1;
        entry.last_accessed = now;
        
        // 动态调整访问权重：最近访问 + 频率
        let recency_weight = 1.0 / (1.0 + (now.signed_duration_since(entry.last_accessed).num_minutes() as f64 / 60.0));
        let frequency_weight = (entry.hit_count as f64).ln() / 10.0;
        entry.access_weight = recency_weight + frequency_weight;
    }


    /// 高效缓存存储 - 简化的LRU机制
    pub fn put(&self, conv_id: i64, msgs: &[Message], content: String) {
        if !self.config.enabled || msgs.is_empty() {
            return;
        }

        let stable_prefix = self.extract_stable_prefix(msgs);
        let prefix_hash = self.hash_messages(&stable_prefix);
        let cache_key = self.generate_cache_key(conv_id, prefix_hash);
        let messages_hash = self.hash_messages(msgs);

        let mut cache = self.cache.lock().unwrap();

        // 简化的缓存空间管理
        if cache.len() >= self.config.max_entries {
            let oldest_key = cache.iter()
                .min_by_key(|(_, entry)| entry.last_accessed)
                .map(|(k, _)| k.clone());
            if let Some(key) = oldest_key {
                cache.remove(&key);
                debug!("LRU清理缓存: {}", key);
            }
        }

        let now = Utc::now();
        let entry = CacheEntry {
            content,
            messages_hash,
            created_at: now,
            last_accessed: now,
            hit_count: 0,
            stable_prefix_len: stable_prefix.len(),
            semantic_hash: 0, // 简化，不使用语义哈希
            access_weight: 1.0,
        };

        cache.insert(cache_key.clone(), entry);
        debug!("缓存存储: {}", cache_key);
    }


    /// 获取缓存统计
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.lock().unwrap();
        let total_hits: u64 = cache.values().map(|e| e.hit_count).sum();
        let total_entries = cache.len();

        CacheStats {
            total_entries,
            total_hits,
            hit_rate: if total_entries > 0 {
                total_hits as f64 / (total_entries as f64 + total_hits as f64)
            } else {
                0.0
            },
        }
    }

    /// 清理过期缓存
    pub fn cleanup_expired(&self) {
        let mut cache = self.cache.lock().unwrap();
        let now = Utc::now();

        cache.retain(|key, entry| {
            let elapsed = now.signed_duration_since(entry.created_at).num_seconds();
            if elapsed as u64 > self.config.ttl_seconds {
                debug!("清理过期缓存: {}", key);
                false
            } else {
                true
            }
        });
    }
}

/// 缓存统计
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_hits: u64,
    pub hit_rate: f64,
}

// ============= 策略层 =============

/// 压缩策略特征
pub trait CompressionStrategy: Send + Sync {
    fn compress(&self, msgs: &[Message], config: &ContextConfig) -> AppResult<Vec<Message>>;
}

/// 高效压缩策略 - 基于Claude Code的30%保留率实现
pub struct EfficientCompressionStrategy;

/// 消息重要性评估
#[derive(Debug, Clone)]
struct MessageImportance {
    pub index: usize,
    pub message: Message,
    pub importance_score: f64,
    pub is_tool_execution: bool,
    pub is_system: bool,
}

impl CompressionStrategy for EfficientCompressionStrategy {
    fn compress(&self, msgs: &[Message], config: &ContextConfig) -> AppResult<Vec<Message>> {
        if msgs.len() < config.min_compress_batch {
            debug!("消息数量不足，跳过压缩: {}", msgs.len());
            return Ok(msgs.to_vec());
        }

        debug!("开始高效压缩: {} 条消息", msgs.len());

        // 计算30%保留目标数量 (Claude Code标准)
        let target_count = (msgs.len() as f64 * 0.30).max(config.min_compress_batch as f64) as usize;
        
        // 1. 分析消息重要性
        let importance_analysis = self.analyze_message_importance(msgs);
        
        // 2. 应用保留策略
        let preserved_messages = self.apply_retention_strategy(importance_analysis, target_count, config)?;

        // 3. 最终排序和去重
        self.finalize_result(preserved_messages, msgs.len())
    }
}

impl EfficientCompressionStrategy {
    /// 快速重要性分析
    fn analyze_message_importance(&self, msgs: &[Message]) -> Vec<MessageImportance> {
        let scorer = AdvancedMessageScorer::new();
        
        msgs.iter()
            .enumerate()
            .map(|(index, msg)| {
                let importance_score = scorer.compute_comprehensive_score(msg, index, msgs.len());
                
                MessageImportance {
                    index,
                    message: msg.clone(),
                    importance_score,
                    is_tool_execution: msg.steps_json.is_some(),
                    is_system: msg.role == "system",
                }
            })
            .collect()
    }

    /// 高效保留策略 - 基于30%目标
    fn apply_retention_strategy(&self, mut analysis: Vec<MessageImportance>, target_count: usize, config: &ContextConfig) -> AppResult<Vec<MessageImportance>> {
        let mut result = Vec::new();

        // 1. 强制保留系统消息
        for item in &analysis {
            if item.is_system {
                result.push(item.clone());
            }
        }

        // 2. 保留最近的消息（保证连续性）
        let recent_start = analysis.len().saturating_sub(config.keep_recent);
        for item in &analysis[recent_start..] {
            if !item.is_system && !result.iter().any(|r| r.index == item.index) {
                result.push(item.clone());
            }
        }

        // 3. 如果还需要更多消息，按重要性选择
        if result.len() < target_count {
            analysis.sort_by(|a, b| b.importance_score.partial_cmp(&a.importance_score).unwrap_or(std::cmp::Ordering::Equal));
            
            let needed = target_count - result.len();
            for item in analysis.iter().take(needed * 2) {
                if result.len() >= target_count { break; }
                if !result.iter().any(|r| r.index == item.index) {
                    result.push(item.clone());
                }
            }
        }

        // 排序和去重
        result.sort_by_key(|m| m.index);
        result.dedup_by_key(|m| m.message.id);

        debug!("保留策略结果: {} -> {} 条消息", analysis.len(), result.len());
        Ok(result)
    }

    /// 最终结果处理
    fn finalize_result(&self, mut messages: Vec<MessageImportance>, original_count: usize) -> AppResult<Vec<Message>> {
        // 按时间顺序排序
        messages.sort_by_key(|m| m.index);
        
        let result: Vec<Message> = messages.into_iter().map(|m| m.message).collect();
        let compression_rate = (1.0 - result.len() as f64 / original_count as f64) * 100.0;

        debug!(
            "高效压缩完成: {} -> {} 条消息 (压缩率: {:.1}%)",
            original_count, result.len(), compression_rate
        );

        Ok(result)
    }
}

/// 循环检测策略
pub struct LoopDetector {
    window_size: usize,
}

impl LoopDetector {
    pub fn new(window_size: usize) -> Self {
        Self { window_size }
    }

    pub fn remove_loops(&self, msgs: Vec<Message>) -> Vec<Message> {
        let mut result = Vec::new();
        let mut seen_hashes = HashMap::new();

        for (idx, msg) in msgs.into_iter().enumerate() {
            let hash = self.content_hash(&msg.content);

            if let Some(&last_idx) = seen_hashes.get(&hash) {
                if idx - last_idx <= self.window_size {
                    debug!("跳过循环消息: {:?}", msg.id);
                    continue;
                }
            }

            seen_hashes.insert(hash, idx);
            result.push(msg);
        }

        result
    }

    fn content_hash(&self, content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
}

// ============= 评分层 =============

/// 高级消息评分器 - 多维度评分
pub struct AdvancedMessageScorer {
    keyword_weights: HashMap<&'static str, f32>,
    semantic_weights: HashMap<&'static str, f32>,
}

/// 简单消息评分器（兼容性保留）
pub struct MessageScorer {
    keyword_weights: HashMap<&'static str, f32>,
}

impl AdvancedMessageScorer {
    pub fn new() -> Self {
        let mut keyword_weights = HashMap::new();
        keyword_weights.insert("error", 3.0);
        keyword_weights.insert("warning", 2.5);
        keyword_weights.insert("failed", 2.8);
        keyword_weights.insert("success", 2.0);
        keyword_weights.insert("config", 1.8);
        keyword_weights.insert("install", 1.6);
        keyword_weights.insert("debug", 1.4);
        keyword_weights.insert("important", 2.2);
        keyword_weights.insert("critical", 3.2);

        let mut semantic_weights = HashMap::new();
        semantic_weights.insert("question", 1.5);
        semantic_weights.insert("solution", 2.0);
        semantic_weights.insert("problem", 1.8);
        semantic_weights.insert("implement", 1.6);
        semantic_weights.insert("fix", 2.2);
        semantic_weights.insert("optimize", 1.7);

        Self {
            keyword_weights,
            semantic_weights,
        }
    }

    /// 计算综合重要性评分
    pub fn compute_comprehensive_score(&self, msg: &Message, index: usize, total_msgs: usize) -> f64 {
        let mut score = 0.0;

        // 1. 基础角色权重
        score += match msg.role.as_str() {
            "system" => 4.0,
            "assistant" => 1.8,
            "user" => 1.2,
            _ => 0.8,
        };

        // 2. 工具执行高权重（确保保留重要执行结果）
        if msg.steps_json.is_some() {
            score += self.evaluate_tool_execution_importance(msg);
        }

        // 3. 内容语义评分
        score += self.evaluate_content_semantics(&msg.content);

        // 4. 位置权重（最近和最早的消息更重要）
        let position_weight = self.compute_position_weight(index, total_msgs);
        score *= position_weight;

        // 5. 内容长度评分（适中长度最优）
        score += self.evaluate_content_length(&msg.content);

        // 6. 时间衰减（较新的消息权重更高）
        score *= self.compute_time_decay(msg);

        // 7. 对话连续性评分（考虑上下文关联）
        score += self.evaluate_conversational_continuity(msg);

        score.max(0.0).min(10.0)
    }

    /// 评估工具执行重要性
    fn evaluate_tool_execution_importance(&self, msg: &Message) -> f64 {
        let base_score = 4.0; // 工具执行基础高分

        if let Some(ref steps_json) = msg.steps_json {
            // 检查是否有错误
            if steps_json.contains("ToolError") || steps_json.contains("failed") {
                return base_score + 2.0; // 错误信息很重要
            }

            // 检查工具类型重要性
            if steps_json.contains("Read") || steps_json.contains("Write") || steps_json.contains("Edit") {
                return base_score + 1.5; // 文件操作重要
            }

            if steps_json.contains("Bash") || steps_json.contains("Execute") {
                return base_score + 1.0; // 命令执行重要
            }
        }

        base_score
    }

    /// 评估内容语义
    fn evaluate_content_semantics(&self, content: &str) -> f64 {
        let mut score = 0.0f64;
        let content_lower = content.to_lowercase();

        // 关键词权重
        for (&keyword, &weight) in &self.keyword_weights {
            if content_lower.contains(keyword) {
                score += weight as f64;
            }
        }

        // 语义模式权重
        for (&pattern, &weight) in &self.semantic_weights {
            if content_lower.contains(pattern) {
                score += weight as f64;
            }
        }

        // 问号表示问题，重要性较高
        let question_count = content.matches('?').count() as f64;
        score += question_count * 0.5;

        // 代码块表示技术内容，适度加分
        let code_block_count = content.matches("```").count() as f64 / 2.0;
        score += code_block_count * 0.8;

        score
    }

    /// 计算位置权重
    fn compute_position_weight(&self, index: usize, total_msgs: usize) -> f64 {
        if total_msgs <= 1 {
            return 1.0;
        }

        let relative_position = index as f64 / (total_msgs - 1) as f64;

        // 最近的消息权重最高，最早的消息也有一定权重
        if relative_position > 0.8 {
            1.4 // 最近20%的消息
        } else if relative_position < 0.2 {
            1.2 // 最早20%的消息
        } else {
            1.0 // 中间消息正常权重
        }
    }

    /// 评估内容长度
    fn evaluate_content_length(&self, content: &str) -> f64 {
        match content.len() {
            0..=20 => 0.3,      // 太短，信息量少
            21..=100 => 1.2,    // 适中，信息密度高
            101..=300 => 1.5,   // 较长，信息丰富
            301..=800 => 1.0,   // 很长，可能有冗余
            _ => 0.6,           // 过长，可能信息冗余
        }
    }

    /// 计算时间衰减
    fn compute_time_decay(&self, msg: &Message) -> f64 {
        let created_at = chrono::DateTime::parse_from_rfc3339(&msg.created_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        
        let hours_ago = chrono::Utc::now()
            .signed_duration_since(created_at)
            .num_hours() as f64;

        // 48小时半衰期，但最低保持30%权重
        let decay_factor = (-hours_ago / 48.0).exp();
        decay_factor.max(0.3)
    }

    /// 评估对话连续性
    fn evaluate_conversational_continuity(&self, msg: &Message) -> f64 {
        let mut score = 0.0;

        // 回复指示词加分
        let content_lower = msg.content.to_lowercase();
        if content_lower.contains("thanks") || content_lower.contains("thank you") {
            score += 0.5;
        }

        if content_lower.contains("please") || content_lower.contains("help") {
            score += 0.8;
        }

        // 确认或否定词汇
        if content_lower.contains("yes") || content_lower.contains("no") 
            || content_lower.contains("ok") || content_lower.contains("sure") {
            score += 0.3;
        }

        score
    }
}

impl MessageScorer {
    pub fn new() -> Self {
        let mut weights = HashMap::new();
        weights.insert("error", 2.0);
        weights.insert("warning", 1.5);
        weights.insert("config", 1.3);
        weights.insert("install", 1.2);
        weights.insert("debug", 1.1);

        Self {
            keyword_weights: weights,
        }
    }

    pub fn score(&self, msg: &Message) -> f32 {
        let mut score = 0.0;

        // 基础分：角色权重
        score += match msg.role.as_str() {
            "system" => 3.0,
            "assistant" => 1.5,
            "user" => 1.0,
            _ => 0.5,
        };

        // 工具调用加分（大幅提高分数，确保重要工具结果不被丢失）
        if msg.steps_json.is_some() {
            score += 5.0; // 从2.0提高到5.0，确保工具执行消息优先保留
        }

        // 长度分 (适中长度得分高)
        let len_score = match msg.content.len() {
            0..=50 => 0.5,
            51..=200 => 1.5,
            201..=500 => 2.0,
            501..=1000 => 1.0,
            _ => 0.5,
        };
        score += len_score;

        // 关键词分
        let content_lower = msg.content.to_lowercase();
        for (&keyword, &weight) in &self.keyword_weights {
            if content_lower.contains(keyword) {
                score += weight;
            }
        }

        // 时间衰减 (24小时半衰期)
        let created_at = chrono::DateTime::parse_from_rfc3339(&msg.created_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let hours = chrono::Utc::now()
            .signed_duration_since(created_at)
            .num_hours() as f32;
        score *= (-hours / 24.0).exp();

        score.max(0.0).min(10.0)
    }
}

// ============= 管理层 =============

/// 压缩决策类型
#[derive(Debug, Clone, PartialEq)]
enum CompressionDecision {
    /// 无需压缩
    NoCompression,
    /// 轻量压缩（去除冗余，保留核心）
    LightCompression,
    /// 深度压缩（使用智能策略大幅压缩）
    HeavyCompression,
}

/// 智能上下文管理器 - 主要入口
pub struct ContextManager {
    config: ContextConfig,
    strategy: Box<dyn CompressionStrategy>,
    loop_detector: LoopDetector,
    kv_cache: KVCache,
}

impl ContextManager {
    pub fn new(config: ContextConfig) -> Self {
        let kv_cache = KVCache::new(config.kv_cache.clone());
        Self {
            loop_detector: LoopDetector::new(6), // 优化循环检测窗口
            strategy: Box::new(EfficientCompressionStrategy), // 使用高效压缩策略
            kv_cache,
            config,
        }
    }

    /// 创建带自定义策略的管理器
    pub fn with_strategy(config: ContextConfig, strategy: Box<dyn CompressionStrategy>) -> Self {
        let kv_cache = KVCache::new(config.kv_cache.clone());
        Self {
            loop_detector: LoopDetector::new(8),
            strategy,
            kv_cache,
            config,
        }
    }

    /// 构建智能上下文 - 主要API
    pub async fn build_context(
        &self,
        repos: &RepositoryManager,
        conv_id: i64,
        up_to_msg_id: Option<i64>,
    ) -> AppResult<ContextResult> {
        info!("构建智能上下文: conv={}, up_to={:?}", conv_id, up_to_msg_id);

        // 1. 获取原始消息
        let raw_msgs = self.fetch_messages(repos, conv_id, up_to_msg_id).await?;
        if raw_msgs.is_empty() {
            debug!("消息列表为空");
            return Ok(ContextResult {
                messages: Vec::new(),
                original_count: 0,
                token_count: 0,
                compressed: false,
            });
        }

        let token_count = self.estimate_tokens(&raw_msgs);
        let original_count = raw_msgs.len();

        // 2. 智能压缩判断逻辑
        let compression_decision = self.make_compression_decision(&raw_msgs, token_count);
        
        let processed_msgs = match compression_decision {
            CompressionDecision::NoCompression => {
                debug!("无需压缩，执行循环检测");
                self.loop_detector.remove_loops(raw_msgs)
            },
            CompressionDecision::LightCompression => {
                debug!("执行轻量压缩");
                self.apply_light_compression(&raw_msgs)?
            },
            CompressionDecision::HeavyCompression => {
                info!("执行深度压缩: tokens={}/{}, 消息数={}", 
                      token_count, self.config.max_tokens, original_count);
                let compressed = self.strategy.compress(&raw_msgs, &self.config)?;
                self.loop_detector.remove_loops(compressed)
            },
        };

        let final_token_count = self.estimate_tokens(&processed_msgs);
        
        debug!(
            "上下文构建完成: {} -> {} 条消息, tokens: {} -> {}",
            original_count, processed_msgs.len(), token_count, final_token_count
        );

        Ok(ContextResult {
            messages: processed_msgs,
            original_count,
            token_count: final_token_count,
            compressed: !matches!(compression_decision, CompressionDecision::NoCompression),
        })
    }

    /// 智能压缩决策
    fn make_compression_decision(&self, msgs: &[Message], token_count: usize) -> CompressionDecision {
        let token_ratio = token_count as f32 / self.config.max_tokens as f32;
        let msg_count = msgs.len();
        
        // 计算压力指标
        let token_pressure = token_ratio > self.config.compress_threshold;
        let message_pressure = msg_count > self.config.keep_recent + self.config.keep_important;
        let tool_message_ratio = msgs.iter().filter(|m| m.steps_json.is_some()).count() as f32 / msg_count as f32;

        match (token_pressure, message_pressure, tool_message_ratio > 0.6) {
            (false, false, _) => CompressionDecision::NoCompression,
            (false, true, false) => CompressionDecision::LightCompression,
            (true, _, _) | (_, true, true) => CompressionDecision::HeavyCompression,
        }
    }

    /// 轻量压缩 - 保留核心信息，移除冗余
    fn apply_light_compression(&self, msgs: &[Message]) -> AppResult<Vec<Message>> {
        let mut result = Vec::new();
        let _scorer = AdvancedMessageScorer::new();

        // 1. 保留所有系统消息
        result.extend(msgs.iter().filter(|m| m.role == "system").cloned());

        // 2. 保留所有工具执行消息（工具链完整性重要）
        result.extend(msgs.iter().filter(|m| m.steps_json.is_some()).cloned());

        // 3. 保留最近的对话
        let recent_start = msgs.len().saturating_sub(self.config.keep_recent);
        for msg in &msgs[recent_start..] {
            if !result.iter().any(|m| m.id == msg.id) {
                result.push(msg.clone());
            }
        }

        // 4. 移除低质量重复消息
        result.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        result.dedup_by(|a, b| {
            if a.id == b.id {
                return true;
            }
            // 内容相似度去重
            let similarity = self.content_similarity(&a.content, &b.content);
            similarity > 0.9 && (a.content.len() < b.content.len())
        });

        debug!("轻量压缩: {} -> {} 条消息", msgs.len(), result.len());
        Ok(result)
    }

    /// 计算内容相似度
    fn content_similarity(&self, content1: &str, content2: &str) -> f64 {
        let words1: std::collections::HashSet<_> = content1.split_whitespace().collect();
        let words2: std::collections::HashSet<_> = content2.split_whitespace().collect();
        
        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();
        
        if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
    }

    /// 构建带摘要的prompt - 集成KV Cache
    pub async fn build_prompt(
        &self,
        repos: &RepositoryManager,
        conv_id: i64,
        current_msg: &str,
        up_to_msg_id: Option<i64>,
    ) -> AppResult<String> {
        // 1. 获取历史消息用于缓存键计算（排除当前正在处理的消息）
        let raw_msgs = self.fetch_messages(repos, conv_id, up_to_msg_id).await?;
        
        // 排除最后一条消息（当前用户消息），只对历史消息做缓存
        let history_msgs = if raw_msgs.len() > 1 {
            &raw_msgs[..raw_msgs.len()-1]
        } else {
            &raw_msgs[..]
        };

        // 2. 尝试从KV缓存获取
        if let Some(cached_prompt) = self.kv_cache.get(conv_id, history_msgs) {
            info!("KV缓存命中: conv={}", conv_id);

            // 缓存命中时，只需要添加当前消息
            return Ok(format!(
                "{}\n\n【当前问题】\n{}",
                cached_prompt, current_msg
            ));
        }

        // 3. 缓存未命中，构建完整prompt
        info!("KV缓存未命中，构建新prompt: conv={}", conv_id);

        let ctx = self.build_context(repos, conv_id, up_to_msg_id).await?;

        // 获取用户前置提示词
        let prefix = repos
            .ai_models()
            .get_user_prefix_prompt()
            .await?
            .unwrap_or_default();

        let mut parts = Vec::new();

        // 添加前置提示词 (稳定前缀)
        if !prefix.trim().is_empty() {
            parts.push(format!("【前置提示】\n{}\n", prefix));
        }

        // 构建历史对话 (可变部分)
        if !ctx.messages.is_empty() {
            let history = ctx
                .messages
                .iter()
                .map(|m| self.format_message(m))
                .collect::<Vec<_>>()
                .join("\n");

            let compression_info = if ctx.compressed {
                format!("，已智能压缩至{}条", ctx.messages.len())
            } else {
                String::new()
            };

            parts.push(format!(
                "【对话历史】(共{}条消息{})\n{}\n",
                ctx.original_count, compression_info, history
            ));
        }

        // 4. 缓存稳定部分 (前缀 + 历史对话)
        let stable_content = parts.join("\n");
        self.kv_cache
            .put(conv_id, history_msgs, stable_content.clone());

        // 5. 添加当前问题并返回
        parts.push(format!("【当前问题】\n{}", current_msg));
        Ok(parts.join("\n"))
    }

    // ============= 私有方法 =============

    async fn fetch_messages(
        &self,
        repos: &RepositoryManager,
        conv_id: i64,
        _up_to_msg_id: Option<i64>,
    ) -> AppResult<Vec<Message>> {
        // TODO: 实现up_to_message_id逻辑
        repos
            .conversations()
            .get_messages(conv_id, None, None)
            .await
    }

    /// 智能token估算 - 考虑不同内容类型
    fn estimate_tokens(&self, msgs: &[Message]) -> usize {
        msgs.iter()
            .map(|msg| self.estimate_single_message_tokens(msg))
            .sum()
    }

    /// 估算单条消息的token数
    fn estimate_single_message_tokens(&self, msg: &Message) -> usize {
        let mut tokens = 0;

        // 基础内容token估算
        tokens += self.estimate_text_tokens(&msg.content);

        // 角色标识token
        tokens += match msg.role.as_str() {
            "system" => 10,     // 系统消息有额外结构化开销
            "assistant" => 5,   // 助手回复有工具调用可能
            "user" => 3,        // 用户消息相对简单
            _ => 2,
        };

        // 工具执行JSON的token估算
        if let Some(ref steps_json) = msg.steps_json {
            tokens += self.estimate_json_tokens(steps_json);
        }

        // 消息元数据开销
        tokens += 8; // ID、时间戳等元数据

        tokens
    }

    /// 估算文本内容的token数
    fn estimate_text_tokens(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }

        let char_count = text.chars().count();
        let word_count = text.split_whitespace().count();
        
        // 中英文混合文本的token估算
        let chinese_chars = text.chars().filter(|c| c.is_ascii_punctuation() || (*c as u32) > 127).count();
        let english_chars = char_count - chinese_chars;

        // 中文字符约1个token，英文单词约0.75个token
        let estimated_tokens = (chinese_chars as f64 * 1.0) + (word_count as f64 * 0.75);

        // 代码块和特殊格式的token成本更高
        let code_blocks = text.matches("```").count() / 2;
        let json_objects = text.matches('{').count().min(text.matches('}').count());
        
        (estimated_tokens + code_blocks as f64 * 5.0 + json_objects as f64 * 2.0) as usize
    }

    /// 估算JSON内容的token数
    fn estimate_json_tokens(&self, json_str: &str) -> usize {
        // JSON结构化数据的token成本通常比纯文本高
        let base_tokens = self.estimate_text_tokens(json_str);
        
        // JSON结构开销
        let object_count = json_str.matches('{').count();
        let array_count = json_str.matches('[').count();
        let string_count = json_str.matches('"').count() / 2;
        
        base_tokens + object_count * 2 + array_count + string_count
    }

    fn format_message(&self, msg: &Message) -> String {
        if msg.role == "assistant" && msg.steps_json.is_some() {
            let steps_json = msg.steps_json.as_ref().unwrap();
            info!("🔍 原始steps_json: {}", steps_json);

            if let Ok(steps_value) = serde_json::from_str(steps_json) {
                let tool_summary = self.extract_tool_summary(&steps_value);

                // AbortError特殊处理: 只保留工具信息，不显示中断文本
                if msg.content.contains("AbortError") {
                    return format!("assistant: {}", tool_summary);
                }

                // 正常工具消息: 结合工具摘要和最终内容
                return format!("assistant: {}\n{}", tool_summary, msg.content.trim());
            }
        }

        // 默认格式化
        format!("{}: {}", msg.role, msg.content)
    }

    fn extract_tool_summary(&self, steps: &serde_json::Value) -> String {
        if let Some(array) = steps.as_array() {
            let mut tool_calls = Vec::new();
            
            for step in array {
                if step.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    if let Some(tool_exec) = step.get("toolExecution") {
                        let tool_name = tool_exec.get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown");
                        
                        // 检查是否执行失败
                        let is_error = tool_exec.get("result")
                            .and_then(|r| r.get("content"))
                            .and_then(|c| c.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|item| item.get("text"))
                            .and_then(|text| text.as_str())
                            .map(|text| text.contains("ToolError:"))
                            .unwrap_or(false);
                        
                        let tool_display = if is_error {
                            format!("{}(failed)", tool_name)
                        } else {
                            tool_name.to_string()
                        };
                        
                        tool_calls.push(tool_display);
                    }
                }
            }

            if !tool_calls.is_empty() {
                return format!("Tools: {}", tool_calls.join(" → "));
            }
        }

        "Completed".to_string()
    }

    /// 获取KV缓存统计
    pub fn cache_stats(&self) -> CacheStats {
        self.kv_cache.stats()
    }

    /// 清理过期缓存
    pub fn cleanup_cache(&self) {
        self.kv_cache.cleanup_expired()
    }

    /// 手动失效缓存 (当对话被修改时调用)
    pub fn invalidate_cache(&self, conv_id: i64) {
        let mut cache = self.kv_cache.cache.lock().unwrap();
        cache.retain(|key, _| !key.contains(&format!("ctx_{}_", conv_id)));
        info!("手动失效缓存: conv={}", conv_id);
    }
}

// ============= 结果类型 =============

/// 上下文构建结果
#[derive(Debug)]
pub struct ContextResult {
    pub messages: Vec<Message>,
    pub original_count: usize,
    pub token_count: usize,
    pub compressed: bool,
}

impl ContextResult {
    /// 转为AI上下文格式
    pub fn to_ai_context(self) -> crate::ai::types::AIContext {
        crate::ai::types::AIContext {
            chat_history: Some(self.messages),
            ..Default::default()
        }
    }
}

// ============= 工厂方法 =============

/// 创建默认上下文管理器
pub fn create_context_manager() -> ContextManager {
    ContextManager::new(ContextConfig::default())
}

/// 创建自定义配置的上下文管理器
pub fn create_context_manager_with_config(config: ContextConfig) -> ContextManager {
    ContextManager::new(config)
}
