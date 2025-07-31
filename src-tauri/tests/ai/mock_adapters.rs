/*!
 * AI模块模拟适配器
 * 
 * 提供用于测试的模拟AI适配器实现
 */

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

use termx::ai::{
    AIAdapter, AIError, AIRequest, AIResponse, AIResult, AIStreamResponse, 
    BatchRequest, BatchResponse, AdapterCapabilities, ModelInfo
};

/// 模拟适配器配置
#[derive(Debug, Clone)]
pub struct MockAdapterConfig {
    pub name: String,
    pub should_fail: bool,
    pub response_delay: Duration,
    pub fixed_response: Option<String>,
    pub call_count_limit: Option<usize>,
}

impl Default for MockAdapterConfig {
    fn default() -> Self {
        Self {
            name: "Mock Adapter".to_string(),
            should_fail: false,
            response_delay: Duration::from_millis(10),
            fixed_response: None,
            call_count_limit: None,
        }
    }
}

/// 模拟AI适配器
pub struct MockAdapter {
    config: MockAdapterConfig,
    call_count: Arc<Mutex<usize>>,
    call_history: Arc<Mutex<Vec<AIRequest>>>,
}

impl MockAdapter {
    pub fn new(config: MockAdapterConfig) -> Self {
        Self {
            config,
            call_count: Arc::new(Mutex::new(0)),
            call_history: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// 创建成功的模拟适配器
    pub fn success(name: &str) -> Self {
        Self::new(MockAdapterConfig {
            name: name.to_string(),
            should_fail: false,
            ..Default::default()
        })
    }
    
    /// 创建失败的模拟适配器
    pub fn failure(name: &str) -> Self {
        Self::new(MockAdapterConfig {
            name: name.to_string(),
            should_fail: true,
            ..Default::default()
        })
    }
    
    /// 创建带延迟的模拟适配器
    pub fn with_delay(name: &str, delay: Duration) -> Self {
        Self::new(MockAdapterConfig {
            name: name.to_string(),
            response_delay: delay,
            ..Default::default()
        })
    }
    
    /// 创建带固定响应的模拟适配器
    pub fn with_fixed_response(name: &str, response: &str) -> Self {
        Self::new(MockAdapterConfig {
            name: name.to_string(),
            fixed_response: Some(response.to_string()),
            ..Default::default()
        })
    }
    
    /// 创建带调用次数限制的模拟适配器
    pub fn with_call_limit(name: &str, limit: usize) -> Self {
        Self::new(MockAdapterConfig {
            name: name.to_string(),
            call_count_limit: Some(limit),
            ..Default::default()
        })
    }
    
    /// 获取调用次数
    pub fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
    
    /// 获取调用历史
    pub fn call_history(&self) -> Vec<AIRequest> {
        self.call_history.lock().unwrap().clone()
    }
    
    /// 重置调用统计
    pub fn reset_stats(&self) {
        *self.call_count.lock().unwrap() = 0;
        self.call_history.lock().unwrap().clear();
    }
    
    /// 设置是否失败
    pub fn set_should_fail(&mut self, should_fail: bool) {
        self.config.should_fail = should_fail;
    }
    
    /// 设置固定响应
    pub fn set_fixed_response(&mut self, response: Option<String>) {
        self.config.fixed_response = response;
    }
}

#[async_trait]
impl AIAdapter for MockAdapter {
    async fn send_request(&self, request: &AIRequest) -> AIResult<AIResponse> {
        // 记录调用
        {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
            
            // 检查调用次数限制
            if let Some(limit) = self.config.call_count_limit {
                if *count > limit {
                    return Err(AIError::rate_limit(
                        format!("Call limit exceeded: {}", limit),
                        Some(self.config.name.clone())
                    ));
                }
            }
        }
        
        // 记录调用历史
        self.call_history.lock().unwrap().push(request.clone());
        
        // 模拟延迟
        if self.config.response_delay > Duration::ZERO {
            sleep(self.config.response_delay).await;
        }
        
        // 检查是否应该失败
        if self.config.should_fail {
            return Err(AIError::model_error(
                "Mock adapter configured to fail",
                Some(self.config.name.clone())
            ));
        }
        
        // 生成响应
        let content = if let Some(ref fixed) = self.config.fixed_response {
            fixed.clone()
        } else {
            format!("Mock response for: {}", request.content)
        };
        
        Ok(AIResponse {
            content,
            model_id: self.config.name.clone(),
            usage: None,
            metadata: None,
        })
    }
    
    async fn test_connection(&self) -> AIResult<bool> {
        if self.config.should_fail {
            Err(AIError::network(
                "Mock connection test failed",
                Some(self.config.name.clone())
            ))
        } else {
            Ok(true)
        }
    }
    
    fn name(&self) -> &str {
        &self.config.name
    }
    
    fn supported_features(&self) -> Vec<String> {
        vec![
            "completion".to_string(),
            "chat".to_string(),
            "mock".to_string(),
        ]
    }
    
    async fn send_stream_request(&self, request: &AIRequest) -> AIResult<AIStreamResponse> {
        // 简单的流式响应模拟
        if self.config.should_fail {
            return Err(AIError::model_error(
                "Mock streaming failed",
                Some(self.config.name.clone())
            ));
        }
        
        // 这里应该返回一个实际的流，但为了简化测试，我们返回错误
        Err(AIError::unknown("Streaming not implemented in mock"))
    }
    
    async fn send_batch_request(&self, batch: &BatchRequest) -> AIResult<BatchResponse> {
        if self.config.should_fail {
            return Err(AIError::model_error(
                "Mock batch request failed",
                Some(self.config.name.clone())
            ));
        }
        
        // 模拟批量处理
        let mut responses = Vec::new();
        for request in &batch.requests {
            let response = self.send_request(request).await?;
            responses.push(response);
        }
        
        Ok(BatchResponse {
            responses,
            batch_id: batch.batch_id.clone(),
            metadata: None,
        })
    }
    
    fn get_capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            supports_streaming: false,
            supports_batch: true,
            supports_function_calling: false,
            supports_vision: false,
            max_tokens: Some(4096),
            max_batch_size: Some(10),
            supported_models: vec![self.config.name.clone()],
        }
    }
    
    async fn get_model_info(&self) -> AIResult<ModelInfo> {
        Ok(ModelInfo {
            name: self.config.name.clone(),
            version: Some("mock-1.0".to_string()),
            context_length: Some(4096),
            capabilities: Some(self.get_capabilities()),
        })
    }
}

/// 创建多个模拟适配器的工厂函数
pub fn create_mock_adapters() -> HashMap<String, Box<dyn AIAdapter>> {
    let mut adapters: HashMap<String, Box<dyn AIAdapter>> = HashMap::new();
    
    adapters.insert(
        "mock-success".to_string(),
        Box::new(MockAdapter::success("mock-success"))
    );
    
    adapters.insert(
        "mock-failure".to_string(),
        Box::new(MockAdapter::failure("mock-failure"))
    );
    
    adapters.insert(
        "mock-slow".to_string(),
        Box::new(MockAdapter::with_delay("mock-slow", Duration::from_millis(100)))
    );
    
    adapters.insert(
        "mock-fixed".to_string(),
        Box::new(MockAdapter::with_fixed_response("mock-fixed", "Fixed response"))
    );
    
    adapters.insert(
        "mock-limited".to_string(),
        Box::new(MockAdapter::with_call_limit("mock-limited", 3))
    );
    
    adapters
}

#[cfg(test)]
mod tests {
    use super::*;
    use termx::ai::{AIRequestType, AIRequest};

    #[tokio::test]
    async fn test_mock_adapter_success() {
        let adapter = MockAdapter::success("test");
        let request = AIRequest::new(AIRequestType::Completion, "test".to_string());
        
        let result = adapter.send_request(&request).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.content, "Mock response for: test");
        assert_eq!(adapter.call_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_adapter_failure() {
        let adapter = MockAdapter::failure("test");
        let request = AIRequest::new(AIRequestType::Completion, "test".to_string());
        
        let result = adapter.send_request(&request).await;
        assert!(result.is_err());
        assert_eq!(adapter.call_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_adapter_fixed_response() {
        let adapter = MockAdapter::with_fixed_response("test", "Custom response");
        let request = AIRequest::new(AIRequestType::Completion, "test".to_string());
        
        let result = adapter.send_request(&request).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.content, "Custom response");
    }

    #[tokio::test]
    async fn test_mock_adapter_call_limit() {
        let adapter = MockAdapter::with_call_limit("test", 2);
        let request = AIRequest::new(AIRequestType::Completion, "test".to_string());
        
        // 前两次调用应该成功
        assert!(adapter.send_request(&request).await.is_ok());
        assert!(adapter.send_request(&request).await.is_ok());
        
        // 第三次调用应该失败
        let result = adapter.send_request(&request).await;
        assert!(result.is_err());
        assert_eq!(adapter.call_count(), 3);
    }

    #[tokio::test]
    async fn test_connection_test() {
        let success_adapter = MockAdapter::success("test");
        assert!(success_adapter.test_connection().await.is_ok());
        
        let failure_adapter = MockAdapter::failure("test");
        assert!(failure_adapter.test_connection().await.is_err());
    }

    #[test]
    fn test_adapter_features() {
        let adapter = MockAdapter::success("test");
        let features = adapter.supported_features();
        assert!(features.contains(&"completion".to_string()));
        assert!(features.contains(&"chat".to_string()));
        assert!(features.contains(&"mock".to_string()));
    }

    #[test]
    fn test_create_mock_adapters() {
        let adapters = create_mock_adapters();
        assert_eq!(adapters.len(), 5);
        assert!(adapters.contains_key("mock-success"));
        assert!(adapters.contains_key("mock-failure"));
        assert!(adapters.contains_key("mock-slow"));
        assert!(adapters.contains_key("mock-fixed"));
        assert!(adapters.contains_key("mock-limited"));
    }
}
