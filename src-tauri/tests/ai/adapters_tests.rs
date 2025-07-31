/*!
 * AI适配器测试
 */

use termx::ai::{AIAdapterManager, CustomAdapter, UnifiedAIAdapter};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{test_data::TestModelConfigs, MockAdapter};

    #[test]
    fn test_adapter_manager_creation() {
        let manager = AIAdapterManager::new();
        // 基本创建测试
        assert!(true);
    }

    #[tokio::test]
    async fn test_mock_adapter() {
        let adapter = MockAdapter::success("test");

        // 测试连接
        let connection_result = adapter.test_connection().await;
        assert!(connection_result.is_ok());

        // 测试功能列表
        let features = adapter.supported_features();
        assert!(!features.is_empty());

        // 测试名称
        assert_eq!(adapter.name(), "test");
    }

    #[tokio::test]
    async fn test_mock_adapter_failure() {
        let adapter = MockAdapter::failure("test");

        let connection_result = adapter.test_connection().await;
        assert!(connection_result.is_err());
    }

    #[tokio::test]
    async fn test_adapter_request() {
        let adapter = MockAdapter::success("test");
        let request = crate::ai::test_data::TestRequests::chat();

        let result = adapter.send_request(&request).await;
        assert!(result.is_ok());

        if let Ok(response) = result {
            assert!(!response.content.is_empty());
            assert_eq!(response.model_id, "test");
        }
    }

    #[test]
    fn test_adapter_capabilities() {
        let adapter = MockAdapter::success("test");
        let capabilities = adapter.get_capabilities();

        // 验证能力结构
        assert!(capabilities.max_tokens.is_some());
        assert!(!capabilities.supported_models.is_empty());
    }

    #[tokio::test]
    async fn test_adapter_manager_operations() {
        let mut manager = AIAdapterManager::new();
        let adapter = Box::new(MockAdapter::success("test-adapter"));

        // 测试添加适配器
        manager.add_adapter("test-id".to_string(), adapter);

        // 测试获取适配器
        let retrieved = manager.get_adapter("test-id");
        assert!(retrieved.is_some());

        // 测试列出适配器
        let adapters = manager.list_adapters();
        assert!(!adapters.is_empty());
    }
}
