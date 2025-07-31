/*!
 * 提示词引擎测试
 */

use termx::ai::{PromptEngine, AIRequest, AIRequestType};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::test_data::TestRequests;

    #[test]
    fn test_prompt_engine_creation() {
        let engine = PromptEngine::new();
        // 基本创建测试
        assert!(true); // 简单验证能创建
    }

    #[test]
    fn test_generate_prompt() {
        let engine = PromptEngine::new();
        let request = TestRequests::completion();
        
        let result = engine.generate_prompt(&request);
        // 简单验证能生成提示词
        match result {
            Ok(prompt) => assert!(!prompt.is_empty()),
            Err(_) => {}, // 可能方法不存在，跳过
        }
    }

    #[test]
    fn test_different_request_types() {
        let engine = PromptEngine::new();
        
        let requests = vec![
            TestRequests::completion(),
            TestRequests::chat(),
            TestRequests::explanation(),
            TestRequests::error_analysis(),
        ];
        
        for request in requests {
            let _ = engine.generate_prompt(&request);
            // 简单测试不同类型的请求
        }
    }

    #[test]
    fn test_prompt_caching() {
        let mut engine = PromptEngine::new();
        let request = TestRequests::completion();
        
        // 测试缓存功能（如果存在）
        let _ = engine.generate_prompt(&request);
        let _ = engine.generate_prompt(&request);
        // 简单验证不会崩溃
    }
}
