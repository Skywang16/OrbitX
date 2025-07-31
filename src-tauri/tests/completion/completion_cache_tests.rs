//! 补全缓存功能测试

use std::time::Duration;
use std::thread;
use terminal_lib::completion::{CompletionCache, CompletionItem, CompletionType};

#[test]
fn test_cache_basic_operations() {
    let cache = CompletionCache::new(10, Duration::from_secs(1)).unwrap();
    
    let items = vec![
        CompletionItem::new("test1.txt".to_string(), CompletionType::File),
        CompletionItem::new("test2.txt".to_string(), CompletionType::File),
    ];
    
    // 存储
    cache.put("ls test", "/home/user", "filesystem", items.clone()).unwrap();
    
    // 获取
    let cached_items = cache.get("ls test", "/home/user", "filesystem");
    assert!(cached_items.is_some());
    assert_eq!(cached_items.unwrap().len(), 2);
    
    // 不同的键应该返回None
    let no_items = cache.get("ls other", "/home/user", "filesystem");
    assert!(no_items.is_none());
}

#[test]
fn test_cache_expiration() {
    let cache = CompletionCache::new(10, Duration::from_millis(50)).unwrap();
    
    let items = vec![
        CompletionItem::new("test.txt".to_string(), CompletionType::File),
    ];
    
    // 存储
    cache.put("ls", "/home", "filesystem", items).unwrap();
    
    // 立即获取应该成功
    assert!(cache.get("ls", "/home", "filesystem").is_some());
    
    // 等待过期
    thread::sleep(Duration::from_millis(100));
    
    // 过期后应该返回None
    assert!(cache.get("ls", "/home", "filesystem").is_none());
}

#[test]
fn test_cache_cleanup() {
    let cache = CompletionCache::new(10, Duration::from_millis(50)).unwrap();
    
    let items = vec![
        CompletionItem::new("test.txt".to_string(), CompletionType::File),
    ];
    
    // 存储多个项
    cache.put("ls1", "/home", "filesystem", items.clone()).unwrap();
    cache.put("ls2", "/home", "filesystem", items.clone()).unwrap();
    cache.put("ls3", "/home", "filesystem", items).unwrap();
    
    // 等待过期
    thread::sleep(Duration::from_millis(100));
    
    // 清理过期项
    let removed = cache.cleanup_expired().unwrap();
    assert_eq!(removed, 3);
    
    // 统计信息应该显示0个条目
    let stats = cache.stats().unwrap();
    assert_eq!(stats.total_entries, 0);
}

#[test]
fn test_cache_stats() {
    let cache = CompletionCache::new(5, Duration::from_secs(1)).unwrap();
    
    let items = vec![
        CompletionItem::new("test.txt".to_string(), CompletionType::File),
    ];
    
    // 存储一些项
    cache.put("ls1", "/home", "filesystem", items.clone()).unwrap();
    cache.put("ls2", "/home", "filesystem", items).unwrap();
    
    let stats = cache.stats().unwrap();
    assert_eq!(stats.total_entries, 2);
    assert_eq!(stats.capacity, 5);
    assert_eq!(stats.expired_entries, 0);
}

#[test]
fn test_cache_key_generation() {
    let cache = CompletionCache::default();
    
    let key1 = cache.generate_key("ls test", "/home/user", "filesystem");
    let key2 = cache.generate_key("ls test", "/home/user", "filesystem");
    let key3 = cache.generate_key("ls other", "/home/user", "filesystem");
    
    // 相同输入应该生成相同的键
    assert_eq!(key1, key2);
    
    // 不同输入应该生成不同的键
    assert_ne!(key1, key3);
}

#[test]
fn test_cache_lru_eviction() {
    let cache = CompletionCache::new(2, Duration::from_secs(10)).unwrap();
    
    let items = vec![
        CompletionItem::new("test.txt".to_string(), CompletionType::File),
    ];
    
    // 填满缓存
    cache.put("key1", "/home", "filesystem", items.clone()).unwrap();
    cache.put("key2", "/home", "filesystem", items.clone()).unwrap();
    
    // 验证两个项都存在
    assert!(cache.get("key1", "/home", "filesystem").is_some());
    assert!(cache.get("key2", "/home", "filesystem").is_some());
    
    // 添加第三个项，应该驱逐最旧的
    cache.put("key3", "/home", "filesystem", items).unwrap();
    
    // key1应该被驱逐，key2和key3应该存在
    assert!(cache.get("key1", "/home", "filesystem").is_none());
    assert!(cache.get("key2", "/home", "filesystem").is_some());
    assert!(cache.get("key3", "/home", "filesystem").is_some());
}
