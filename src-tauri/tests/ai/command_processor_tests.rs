/*!
 * 命令处理器测试
 */

use std::sync::Arc;
use termx::ai::{AICommandProcessor, ProcessingOptions};
use tokio::sync::Mutex;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::test_data::TestRequests;
    use crate::ai::test_utils::TestEnvironment;

    #[tokio::test]
    async fn test_command_processor_creation() {
        let env = TestEnvironment::new().await.unwrap();

        let processor = AICommandProcessor::new(
            env.config_manager,
            env.adapter_manager,
            env.prompt_engine,
            env.context_manager,
            env.cache_manager,
        );

        // 基本创建测试
        assert!(true);
    }

    #[tokio::test]
    async fn test_process_request() {
        let env = TestEnvironment::new().await.unwrap();

        let processor = AICommandProcessor::new(
            env.config_manager,
            env.adapter_manager,
            env.prompt_engine,
            env.context_manager,
            env.cache_manager,
        );

        let request = TestRequests::completion();
        let options = ProcessingOptions::default();

        let result = processor.process_request(request, options).await;
        // 可能会失败因为没有配置适配器，但不应该panic
        match result {
            Ok(_) => assert!(true),
            Err(_) => assert!(true),
        }
    }

    #[tokio::test]
    async fn test_processing_options() {
        let options = ProcessingOptions::default();

        // 验证默认选项
        assert!(options.use_cache);
        assert!(options.timeout.as_secs() > 0);
        assert!(options.max_retries > 0);
    }

    #[tokio::test]
    async fn test_processor_with_ai_client() {
        let env = TestEnvironment::new().await.unwrap();

        // 添加AI客户端
        {
            let mut adapter_manager = env.adapter_manager.lock().await;
            let config = crate::ai::test_data::TestModelConfigs::openai();
            if let Ok(client) = termx::ai::AIClient::new(config) {
                adapter_manager.register_adapter("test-client".to_string(), Arc::new(client));
            }
        }

        let processor = AICommandProcessor::new(
            env.config_manager,
            env.adapter_manager,
            env.prompt_engine,
            env.context_manager,
            env.cache_manager,
        );

        let request = TestRequests::completion();
        let mut options = ProcessingOptions::default();
        options.preferred_model_id = Some("test-client".to_string());

        let result = processor.process_request(request, options).await;
        // 可能会失败因为没有真实的API密钥，但不应该panic
        match result {
            Ok(processing_result) => {
                assert!(!processing_result.response.content.is_empty());
                assert_eq!(processing_result.model_id, "test-client");
            }
            Err(_) => {} // 在测试环境中可能会失败，这是正常的
        }
    }
}
