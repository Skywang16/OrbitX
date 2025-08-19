/*!
 * 智能上下文管理系统单元测试
 * 
 * 测试覆盖：
 * - KV缓存机制
 * - 消息评分算法
 * - 压缩策略
 * - 循环检测
 * - 边界条件处理
 */

#[cfg(test)]
mod tests {
    use super::super::enhanced_context::*;
    use crate::ai::types::Message;
    use chrono::Utc;

    // ============= 测试辅助函数 =============

    /// 创建测试用的消息
    fn create_test_message(id: i64, conv_id: i64, role: &str, content: &str) -> Message {
        Message {
            id: Some(id),
            conversation_id: conv_id,
            role: role.to_string(),
            content: content.to_string(),
            steps_json: None,
            status: None,
            duration_ms: None,
            created_at: Utc::now().to_rfc3339(),
        }
    }

    /// 创建带工具调用的消息
    fn create_tool_message(id: i64, conv_id: i64, tool_name: &str) -> Message {
        let steps_json = format!(r#"[{{"type":"tool_use","toolExecution":{{"name":"{}","params":{{}},"result":"success"}}}}]"#, tool_name);
        Message {
            id: Some(id),
            conversation_id: conv_id,
            role: "assistant".to_string(),
            content: format!("使用了{}工具", tool_name),
            steps_json: Some(steps_json),
            status: None,
            duration_ms: None,
            created_at: Utc::now().to_rfc3339(),
        }
    }

    /// 创建旧消息（24小时前）
    fn create_old_message(id: i64, conv_id: i64, content: &str) -> Message {
        let old_time = Utc::now() - chrono::Duration::hours(25);
        Message {
            id: Some(id),
            conversation_id: conv_id,
            role: "user".to_string(),
            content: content.to_string(),
            steps_json: None,
            status: None,
            duration_ms: None,
            created_at: old_time.to_rfc3339(),
        }
    }

    // ============= KV缓存测试 =============

    #[test]
    fn test_kv_cache_basic_operations() {
        let config = KVCacheConfig::default();
        let cache = KVCache::new(config);
        
        let messages = vec![
            create_test_message(1, 100, "user", "Hello"),
            create_test_message(2, 100, "assistant", "Hi there!"),
        ];

        // 测试缓存未命中
        assert!(cache.get(100, &messages).is_none());

        // 测试缓存存储
        cache.put(100, &messages, "cached_content".to_string());

        // 测试缓存命中
        let result = cache.get(100, &messages);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "cached_content");

        // 测试统计
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.total_hits, 1);
    }

    #[test]
    fn test_kv_cache_hash_invalidation() {
        let config = KVCacheConfig::default();
        let cache = KVCache::new(config);
        
        let messages1 = vec![
            create_test_message(1, 100, "user", "Hello"),
            create_test_message(2, 100, "assistant", "Hi there!"),
        ];

        let messages2 = vec![
            create_test_message(1, 100, "user", "Hello"),
            create_test_message(2, 100, "assistant", "Hi there!"),
            create_test_message(3, 100, "user", "How are you?"), // 新消息
        ];

        // 缓存第一个消息列表
        cache.put(100, &messages1, "cached_content_1".to_string());
        assert!(cache.get(100, &messages1).is_some());

        // 新消息列表应该缓存未命中（哈希不同）
        assert!(cache.get(100, &messages2).is_none());
    }

    #[test]
    fn test_kv_cache_ttl_expiration() {
        let mut config = KVCacheConfig::default();
        config.ttl_seconds = 1; // 1秒过期
        let cache = KVCache::new(config);
        
        let messages = vec![create_test_message(1, 100, "user", "Hello")];
        
        cache.put(100, &messages, "cached_content".to_string());
        assert!(cache.get(100, &messages).is_some());

        // 模拟等待过期（实际测试中可能需要sleep）
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        // 应该过期
        assert!(cache.get(100, &messages).is_none());
    }

    #[test]
    fn test_kv_cache_lru_eviction() {
        let mut config = KVCacheConfig::default();
        config.max_entries = 2; // 最多2个条目
        let cache = KVCache::new(config);
        
        let messages1 = vec![create_test_message(1, 100, "user", "Message 1")];
        let messages2 = vec![create_test_message(2, 200, "user", "Message 2")];
        let messages3 = vec![create_test_message(3, 300, "user", "Message 3")];

        // 添加两个缓存条目
        cache.put(100, &messages1, "content_1".to_string());
        cache.put(200, &messages2, "content_2".to_string());
        
        assert!(cache.get(100, &messages1).is_some());
        assert!(cache.get(200, &messages2).is_some());

        // 添加第三个应该触发LRU清理
        cache.put(300, &messages3, "content_3".to_string());
        
        // 最旧的应该被清理
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);
    }

    #[test]
    fn test_kv_cache_stable_prefix_extraction() {
        let mut config = KVCacheConfig::default();
        config.stable_prefix_max_tokens = 10; // 限制为10个token
        let cache = KVCache::new(config);
        
        let messages = vec![
            create_test_message(1, 100, "user", "Short"), // ~1 token
            create_test_message(2, 100, "assistant", "This is a longer message"), // ~6 tokens
            create_test_message(3, 100, "user", "This should be excluded"), // 会被排除
        ];

        let prefix = cache.extract_stable_prefix(&messages);
        
        // 应该只包含前两条消息（总计约7个token < 10）
        assert_eq!(prefix.len(), 2);
        assert_eq!(prefix[0].content, "Short");
        assert_eq!(prefix[1].content, "This is a longer message");
    }

    // ============= 消息评分测试 =============

    #[test]
    fn test_message_scorer_role_weights() {
        let scorer = MessageScorer::new();
        
        let system_msg = create_test_message(1, 100, "system", "System message");
        let assistant_msg = create_test_message(2, 100, "assistant", "Assistant message");
        let user_msg = create_test_message(3, 100, "user", "User message");
        
        let system_score = scorer.score(&system_msg);
        let assistant_score = scorer.score(&assistant_msg);
        let user_score = scorer.score(&user_msg);
        
        assert!(system_score > assistant_score);
        assert!(assistant_score > user_score);
    }

    #[test]
    fn test_message_scorer_tool_bonus() {
        let scorer = MessageScorer::new();
        
        let normal_msg = create_test_message(1, 100, "assistant", "Normal response");
        let tool_msg = create_tool_message(2, 100, "file_read");
        
        let normal_score = scorer.score(&normal_msg);
        let tool_score = scorer.score(&tool_msg);
        
        assert!(tool_score > normal_score);
    }

    #[test]
    fn test_message_scorer_length_optimization() {
        let scorer = MessageScorer::new();
        
        let short_msg = create_test_message(1, 100, "user", "Hi"); // 2 chars
        let optimal_msg = create_test_message(2, 100, "user", "This is an optimal length message for scoring purposes."); // ~300 chars
        let long_msg = create_test_message(3, 100, "user", &"Very long message ".repeat(100)); // >1000 chars
        
        let short_score = scorer.score(&short_msg);
        let optimal_score = scorer.score(&optimal_msg);
        let long_score = scorer.score(&long_msg);
        
        assert!(optimal_score > short_score);
        assert!(optimal_score > long_score);
    }

    #[test]
    fn test_message_scorer_keyword_detection() {
        let scorer = MessageScorer::new();
        
        let normal_msg = create_test_message(1, 100, "user", "How are you today?");
        let error_msg = create_test_message(2, 100, "user", "I got an error message");
        let config_msg = create_test_message(3, 100, "user", "Please update the config file");
        
        let normal_score = scorer.score(&normal_msg);
        let error_score = scorer.score(&error_msg);
        let config_score = scorer.score(&config_msg);
        
        assert!(error_score > normal_score);
        assert!(config_score > normal_score);
    }

    #[test]
    fn test_message_scorer_time_decay() {
        let scorer = MessageScorer::new();
        
        let recent_msg = create_test_message(1, 100, "user", "Recent message");
        let old_msg = create_old_message(2, 100, "Old message");
        
        let recent_score = scorer.score(&recent_msg);
        let old_score = scorer.score(&old_msg);
        
        assert!(recent_score > old_score);
    }

    // ============= 压缩策略测试 =============

    #[test]
    fn test_hybrid_strategy_preserves_system_messages() {
        let strategy = HybridStrategy;
        let config = ContextConfig::default();
        
        let messages = vec![
            create_test_message(1, 100, "system", "System prompt"),
            create_test_message(2, 100, "user", "User message 1"),
            create_test_message(3, 100, "assistant", "Assistant response 1"),
            create_test_message(4, 100, "user", "User message 2"),
            create_test_message(5, 100, "assistant", "Assistant response 2"),
        ];
        
        let compressed = strategy.compress(&messages, &config).unwrap();
        
        // 系统消息应该被保留
        assert!(compressed.iter().any(|m| m.role == "system"));
        assert!(compressed.iter().any(|m| m.content == "System prompt"));
    }

    #[test]
    fn test_hybrid_strategy_preserves_recent_messages() {
        let strategy = HybridStrategy;
        let mut config = ContextConfig::default();
        config.keep_recent = 2;
        config.keep_important = 0; // 不保留重要消息，只测试最近消息
        
        let messages = vec![
            create_test_message(1, 100, "user", "Old message 1"),
            create_test_message(2, 100, "user", "Old message 2"),
            create_test_message(3, 100, "user", "Recent message 1"),
            create_test_message(4, 100, "user", "Recent message 2"),
        ];
        
        let compressed = strategy.compress(&messages, &config).unwrap();
        
        // 最近的2条消息应该被保留
        assert!(compressed.iter().any(|m| m.content == "Recent message 1"));
        assert!(compressed.iter().any(|m| m.content == "Recent message 2"));
    }

    #[test]
    fn test_hybrid_strategy_preserves_important_messages() {
        let strategy = HybridStrategy;
        let mut config = ContextConfig::default();
        config.keep_recent = 0; // 不保留最近消息，只测试重要消息
        config.keep_important = 2;
        
        let messages = vec![
            create_test_message(1, 100, "user", "Normal message"),
            create_tool_message(2, 100, "file_read"), // 工具调用，应该有高分
            create_test_message(3, 100, "user", "Another normal message"),
            create_test_message(4, 100, "system", "System message"), // 系统消息，高分
        ];
        
        let compressed = strategy.compress(&messages, &config).unwrap();
        
        // 重要消息应该被保留
        assert!(compressed.iter().any(|m| m.steps_json.is_some())); // 工具调用消息
        assert!(compressed.iter().any(|m| m.role == "system")); // 系统消息
    }

    #[test]
    fn test_hybrid_strategy_deduplication() {
        let strategy = HybridStrategy;
        let config = ContextConfig::default();
        
        let duplicate_msg = create_test_message(1, 100, "user", "Duplicate message");
        let messages = vec![
            duplicate_msg.clone(),
            create_test_message(2, 100, "assistant", "Response"),
            duplicate_msg.clone(), // 重复消息
        ];
        
        let compressed = strategy.compress(&messages, &config).unwrap();
        
        // 重复消息应该被去除
        let duplicate_count = compressed.iter()
            .filter(|m| m.content == "Duplicate message")
            .count();
        assert_eq!(duplicate_count, 1);
    }

    // ============= 循环检测测试 =============

    #[test]
    fn test_loop_detector_removes_loops() {
        let detector = LoopDetector::new(3); // 3条消息窗口
        
        let messages = vec![
            create_test_message(1, 100, "user", "Message 1"),
            create_test_message(2, 100, "user", "Message 2"),
            create_test_message(3, 100, "user", "Message 1"), // 循环：重复消息1
            create_test_message(4, 100, "user", "Message 3"),
            create_test_message(5, 100, "user", "Message 1"), // 不在窗口内，不算循环
        ];
        
        let filtered = detector.remove_loops(messages);
        
        assert_eq!(filtered.len(), 4); // 应该移除1条循环消息
        assert_eq!(filtered[0].content, "Message 1");
        assert_eq!(filtered[1].content, "Message 2");
        assert_eq!(filtered[2].content, "Message 3");
        assert_eq!(filtered[3].content, "Message 1"); // 最后这个不算循环
    }

    #[test]
    fn test_loop_detector_window_size() {
        let detector = LoopDetector::new(2); // 2条消息窗口
        
        let messages = vec![
            create_test_message(1, 100, "user", "Message A"),
            create_test_message(2, 100, "user", "Message B"),
            create_test_message(3, 100, "user", "Message C"),
            create_test_message(4, 100, "user", "Message A"), // 超出窗口，不算循环
        ];
        
        let filtered = detector.remove_loops(messages);
        
        assert_eq!(filtered.len(), 4); // 没有消息被移除
    }

    #[test]
    fn test_loop_detector_exact_duplicates() {
        let detector = LoopDetector::new(5);
        
        let identical_content = "Identical message";
        let messages = vec![
            create_test_message(1, 100, "user", identical_content),
            create_test_message(2, 100, "assistant", "Response"),
            create_test_message(3, 100, "user", identical_content), // 完全相同
        ];
        
        let filtered = detector.remove_loops(messages);
        
        assert_eq!(filtered.len(), 2); // 移除重复消息
        assert!(filtered.iter().filter(|m| m.content == identical_content).count() == 1);
    }

    // ============= 配置测试 =============

    #[test]
    fn test_context_config_default_values() {
        let config = ContextConfig::default();
        
        assert_eq!(config.max_tokens, 32000);
        assert_eq!(config.compress_threshold, 0.7);
        assert_eq!(config.keep_recent, 10);
        assert_eq!(config.keep_important, 5);
        assert!(config.kv_cache.enabled);
        assert_eq!(config.kv_cache.ttl_seconds, 3600);
        assert_eq!(config.kv_cache.max_entries, 100);
    }

    #[test]
    fn test_kv_cache_config_default_values() {
        let config = KVCacheConfig::default();
        
        assert!(config.enabled);
        assert_eq!(config.ttl_seconds, 3600);
        assert_eq!(config.max_entries, 100);
        assert_eq!(config.stable_prefix_max_tokens, 1000);
    }

    // ============= 边界条件测试 =============

    #[test]
    fn test_empty_message_list() {
        let strategy = HybridStrategy;
        let config = ContextConfig::default();
        let messages: Vec<Message> = vec![];
        
        let result = strategy.compress(&messages, &config);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_single_message() {
        let strategy = HybridStrategy;
        let config = ContextConfig::default();
        let messages = vec![create_test_message(1, 100, "user", "Single message")];
        
        let compressed = strategy.compress(&messages, &config).unwrap();
        assert_eq!(compressed.len(), 1);
        assert_eq!(compressed[0].content, "Single message");
    }

    #[test]
    fn test_cache_disabled() {
        let mut config = KVCacheConfig::default();
        config.enabled = false;
        let cache = KVCache::new(config);
        
        let messages = vec![create_test_message(1, 100, "user", "Test")];
        
        // 禁用时应该总是返回None
        assert!(cache.get(100, &messages).is_none());
        
        cache.put(100, &messages, "content".to_string());
        assert!(cache.get(100, &messages).is_none());
    }

    #[test]
    fn test_very_long_content() {
        let scorer = MessageScorer::new();
        let very_long_content = "x".repeat(10000);
        let long_msg = create_test_message(1, 100, "user", &very_long_content);
        
        let score = scorer.score(&long_msg);
        
        // 应该能处理很长的内容而不崩溃
        assert!(score >= 0.0);
        assert!(score <= 10.0);
    }

    #[test]
    fn test_invalid_json_in_steps() {
        let mut msg = create_test_message(1, 100, "assistant", "Test");
        msg.steps_json = Some("invalid json".to_string());
        
        let scorer = MessageScorer::new();
        let score = scorer.score(&msg);
        
        // 应该能处理无效JSON而不崩溃
        assert!(score >= 0.0);
    }

    // ============= 性能测试 =============

    #[test]
    fn test_large_message_list_performance() {
        let strategy = HybridStrategy;
        let config = ContextConfig::default();
        
        // 创建1000条消息
        let messages: Vec<Message> = (0..1000)
            .map(|i| create_test_message(i, 100, "user", &format!("Message {}", i)))
            .collect();
        
        let start = std::time::Instant::now();
        let compressed = strategy.compress(&messages, &config).unwrap();
        let duration = start.elapsed();
        
        // 应该在合理时间内完成（比如100ms）
        assert!(duration.as_millis() < 100);
        assert!(compressed.len() <= messages.len());
    }

    #[test]
    fn test_cache_concurrent_access() {
        use std::sync::Arc;
        use std::thread;
        
        let config = KVCacheConfig::default();
        let cache = Arc::new(KVCache::new(config));
        let messages = vec![create_test_message(1, 100, "user", "Test")];
        
        let mut handles = vec![];
        
        // 启动多个线程同时访问缓存
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let msgs = messages.clone();
            
            let handle = thread::spawn(move || {
                cache_clone.put(i, &msgs, format!("content_{}", i));
                cache_clone.get(i, &msgs)
            });
            
            handles.push(handle);
        }
        
        // 等待所有线程完成
        for handle in handles {
            let result = handle.join().unwrap();
            assert!(result.is_some());
        }
    }
}