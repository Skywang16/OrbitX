/*!
 * AI适配器测试
 */

use std::sync::Arc;
use termx::ai::{AIAdapterManager, AIClient, AIModelConfig, AIProvider};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::test_data::TestModelConfigs;

    #[test]
    fn test_adapter_manager_creation() {
        let manager = AIAdapterManager::new();
        assert_eq!(manager.adapter_count(), 0);
    }

    #[tokio::test]
    async fn test_ai_client_creation() {
        let config = TestModelConfigs::openai();
        let client = AIClient::new(config);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_adapter_manager_operations() {
        let mut manager = AIAdapterManager::new();
        let config = TestModelConfigs::openai();
        let client = AIClient::new(config).unwrap();
        let adapter = Arc::new(client);

        // 测试添加适配器
        manager.register_adapter("test-id".to_string(), adapter.clone());
        assert_eq!(manager.adapter_count(), 1);

        // 测试获取适配器
        let retrieved = manager.get_adapter("test-id");
        assert!(retrieved.is_some());

        // 测试检查适配器存在
        assert!(manager.has_adapter("test-id"));

        // 测试获取适配器ID列表
        let ids = manager.get_adapter_ids();
        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&"test-id".to_string()));

        // 测试移除适配器
        let removed = manager.remove_adapter("test-id");
        assert!(removed);
        assert_eq!(manager.adapter_count(), 0);
    }

    #[test]
    fn test_adapter_manager_clear() {
        let mut manager = AIAdapterManager::new();
        let config = TestModelConfigs::openai();
        let client = AIClient::new(config).unwrap();
        let adapter = Arc::new(client);

        manager.register_adapter("test-1".to_string(), adapter.clone());
        manager.register_adapter("test-2".to_string(), adapter);
        assert_eq!(manager.adapter_count(), 2);

        manager.clear();
        assert_eq!(manager.adapter_count(), 0);
    }
}
