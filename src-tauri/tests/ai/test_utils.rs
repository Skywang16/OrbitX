/*!
 * AI模块测试工具
 *
 * 提供测试所需的通用工具函数、断言宏和测试数据生成器
 */

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use tokio::sync::Mutex;

use termx::ai::{
    AIAdapterManager, AIConfigManager, AIContext, AIError, AIModelConfig, AIProvider, AIRequest,
    AIRequestType, AIResponse, AISettings, CacheConfig, CacheManager, ContextManager, PromptEngine,
    SecureStorage, SystemInfo,
};

/// 测试环境配置
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub storage_path: PathBuf,
    pub config_manager: Arc<Mutex<AIConfigManager>>,
    pub adapter_manager: Arc<Mutex<AIAdapterManager>>,
    pub context_manager: Arc<Mutex<ContextManager>>,
    pub cache_manager: Arc<Mutex<CacheManager>>,
    pub prompt_engine: Arc<Mutex<PromptEngine>>,
}

impl TestEnvironment {
    /// 创建新的测试环境
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let storage_path = temp_dir.path().join("test_ai_settings.json");

        // 创建安全存储
        let storage = SecureStorage::new(storage_path.clone());

        // 创建配置管理器
        let config_manager = Arc::new(Mutex::new(AIConfigManager::with_storage(storage)));

        // 创建适配器管理器
        let adapter_manager = Arc::new(Mutex::new(AIAdapterManager::new()));

        // 创建上下文管理器
        let context_manager = Arc::new(Mutex::new(ContextManager::new(100)));

        // 创建缓存管理器
        let cache_config = CacheConfig::default();
        let cache_manager = Arc::new(Mutex::new(CacheManager::new(cache_config)?));

        // 创建提示词引擎
        let prompt_engine = Arc::new(Mutex::new(PromptEngine::new()));

        Ok(Self {
            temp_dir,
            storage_path,
            config_manager,
            adapter_manager,
            context_manager,
            cache_manager,
            prompt_engine,
        })
    }

    /// 获取临时存储路径
    pub fn storage_path(&self) -> &PathBuf {
        &self.storage_path
    }

    /// 清理测试环境
    pub async fn cleanup(self) {
        // TempDir会在drop时自动清理
        drop(self.temp_dir);
    }
}

/// 创建测试用的AI模型配置
pub fn create_test_model_config(id: &str, provider: AIProvider) -> AIModelConfig {
    AIModelConfig {
        id: id.to_string(),
        name: format!("Test Model {}", id),
        provider,
        api_url: "https://api.test.com".to_string(),
        api_key: "test_api_key".to_string(),
        model: "test-model".to_string(),
        is_default: Some(false),
        options: None,
    }
}

/// 创建测试用的AI请求
pub fn create_test_request(request_type: AIRequestType, content: &str) -> AIRequest {
    AIRequest::new(request_type, content.to_string()).with_context(create_test_context())
}

/// 创建测试用的AI上下文
pub fn create_test_context() -> AIContext {
    let mut env = HashMap::new();
    env.insert("USER".to_string(), "test_user".to_string());
    env.insert("HOME".to_string(), "/home/test_user".to_string());

    AIContext {
        working_directory: Some("/home/test_user/project".to_string()),
        command_history: Some(vec![
            "ls -la".to_string(),
            "cd project".to_string(),
            "git status".to_string(),
        ]),
        environment: Some(env),
        current_command: Some("npm test".to_string()),
        last_output: Some("Test output".to_string()),
        system_info: Some(SystemInfo {
            platform: "linux".to_string(),
            shell: "bash".to_string(),
            user: "test_user".to_string(),
        }),
    }
}

/// 创建测试用的AI响应
pub fn create_test_response(content: &str) -> AIResponse {
    AIResponse {
        content: content.to_string(),
        model_id: "test-model".to_string(),
        usage: None,
        metadata: None,
    }
}

/// 创建测试用的AI设置
pub fn create_test_settings() -> AISettings {
    let mut settings = AISettings::default();

    // 添加测试模型
    settings
        .models
        .push(create_test_model_config("openai-test", AIProvider::OpenAI));
    settings
        .models
        .push(create_test_model_config("claude-test", AIProvider::Claude));
    settings
        .models
        .push(create_test_model_config("local-test", AIProvider::Local));

    settings.default_model_id = Some("openai-test".to_string());

    settings
}

/// 等待异步操作完成的辅助函数
pub async fn wait_for_condition<F, Fut>(mut condition: F, timeout: Duration) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        if condition().await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    false
}

/// 获取当前时间戳
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// 断言AI错误类型的宏
#[macro_export]
macro_rules! assert_ai_error {
    ($result:expr, $error_type:pat) => {
        match $result {
            Err($error_type) => {}
            Err(other) => panic!("Expected error type, got: {:?}", other),
            Ok(val) => panic!("Expected error, got Ok: {:?}", val),
        }
    };
}

/// 断言AI响应内容的宏
#[macro_export]
macro_rules! assert_response_content {
    ($response:expr, $expected:expr) => {
        assert_eq!($response.content, $expected, "Response content mismatch");
    };
}

/// 断言缓存命中的宏
#[macro_export]
macro_rules! assert_cache_hit {
    ($stats:expr) => {
        assert!($stats.hit_count > 0, "Expected cache hit but got miss");
    };
}

/// 断言缓存未命中的宏
#[macro_export]
macro_rules! assert_cache_miss {
    ($stats:expr) => {
        assert!($stats.miss_count > 0, "Expected cache miss but got hit");
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_environment_creation() {
        let env = TestEnvironment::new().await.unwrap();
        assert!(env.storage_path.exists() == false); // 文件还未创建
        env.cleanup().await;
    }

    #[test]
    fn test_model_config_creation() {
        let config = create_test_model_config("test", AIProvider::OpenAI);
        assert_eq!(config.id, "test");
        assert_eq!(config.provider, AIProvider::OpenAI);
    }

    #[test]
    fn test_request_creation() {
        let request = create_test_request(AIRequestType::Completion, "test content");
        assert_eq!(request.content, "test content");
        assert_eq!(request.request_type, AIRequestType::Completion);
        assert!(request.context.is_some());
    }

    #[test]
    fn test_context_creation() {
        let context = create_test_context();
        assert!(context.working_directory.is_some());
        assert!(context.command_history.is_some());
        assert!(context.environment.is_some());
        assert!(context.system_info.is_some());
    }

    #[test]
    fn test_settings_creation() {
        let settings = create_test_settings();
        assert_eq!(settings.models.len(), 3);
        assert_eq!(settings.default_model_id, Some("openai-test".to_string()));
    }

    #[tokio::test]
    async fn test_wait_for_condition() {
        let mut counter = 0;
        let result = wait_for_condition(
            || {
                counter += 1;
                async move { counter >= 3 }
            },
            Duration::from_millis(100),
        )
        .await;

        assert!(result);
        assert!(counter >= 3);
    }
}
