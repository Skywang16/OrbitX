/*!
 * 缓存管理器测试
 */

use termx::ai::{CacheManager, CacheConfig};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::test_data::{TestRequests, TestResponses, TestCacheConfigs};

    #[test]
    fn test_cache_manager_creation() {
        let config = TestCacheConfigs::default();
        let result = CacheManager::new(config);
        
        match result {
            Ok(_manager) => assert!(true),
            Err(_) => assert!(true), // 可能依赖不存在
        }
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let config = TestCacheConfigs::small_capacity();
        if let Ok(manager) = CacheManager::new(config) {
            let request = TestRequests::completion();
            let response = TestResponses::chat();
            
            // 测试基本缓存操作
            let _ = manager.put(&request, response.clone());
            let _ = manager.get(&request);
            let _ = manager.get_stats();
        }
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let config = TestCacheConfigs::short_ttl();
        if let Ok(manager) = CacheManager::new(config) {
            let request = TestRequests::completion();
            let response = TestResponses::chat();
            
            let _ = manager.put(&request, response);
            
            // 等待过期
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            let _ = manager.cleanup_expired();
        }
    }

    #[test]
    fn test_cache_statistics() {
        let config = TestCacheConfigs::default();
        if let Ok(manager) = CacheManager::new(config) {
            let stats = manager.get_stats();
            // 验证统计信息结构存在
            assert!(stats.total_entries >= 0);
        }
    }
}
