//! 补全引擎功能测试

use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use terminal_lib::completion::{
    CompletionContext, CompletionEngine, CompletionEngineConfig, CompletionItem,
    CompletionProvider, CompletionType,
};
use terminal_lib::utils::error::AppResult;

// 模拟补全提供者用于测试
struct MockProvider {
    name: &'static str,
    should_provide: bool,
    items: Vec<CompletionItem>,
}

impl MockProvider {
    fn new(name: &'static str, should_provide: bool, items: Vec<CompletionItem>) -> Self {
        Self {
            name,
            should_provide,
            items,
        }
    }
}

#[async_trait]
impl CompletionProvider for MockProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    fn should_provide(&self, _context: &CompletionContext) -> bool {
        self.should_provide
    }

    async fn provide_completions(
        &self,
        _context: &CompletionContext,
    ) -> AppResult<Vec<CompletionItem>> {
        Ok(self.items.clone())
    }

    fn priority(&self) -> i32 {
        0
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[tokio::test]
async fn test_completion_engine_creation() {
    let config = CompletionEngineConfig::default();
    let engine = CompletionEngine::new(config).unwrap();

    // 新创建的引擎应该没有提供者
    assert_eq!(engine.providers.len(), 0);
    assert!(engine.cache.is_some());
}

#[tokio::test]
async fn test_completion_engine_with_providers() {
    let config = CompletionEngineConfig::default();
    let mut engine = CompletionEngine::new(config).unwrap();

    // 创建模拟提供者
    let items = vec![
        CompletionItem::new("test1.txt".to_string(), CompletionType::File),
        CompletionItem::new("test2.txt".to_string(), CompletionType::File),
    ];

    let provider = Arc::new(MockProvider::new("test", true, items));
    engine.add_provider(provider);

    assert_eq!(engine.providers.len(), 1);
}

#[tokio::test]
async fn test_completion_engine_get_completions() {
    let config = CompletionEngineConfig::default();
    let mut engine = CompletionEngine::new(config).unwrap();

    // 创建模拟提供者
    let items = vec![
        CompletionItem::new("file1.txt".to_string(), CompletionType::File).with_score(1.0),
        CompletionItem::new("file2.txt".to_string(), CompletionType::File).with_score(0.8),
    ];

    let provider = Arc::new(MockProvider::new("test", true, items));
    engine.add_provider(provider);

    // 创建补全上下文
    let context = CompletionContext::new("file".to_string(), 4, PathBuf::from("/tmp"));

    // 获取补全建议
    let response = engine.get_completions(&context).await.unwrap();

    assert_eq!(response.items.len(), 2);
    assert_eq!(response.items[0].text, "file1.txt");
    assert_eq!(response.items[1].text, "file2.txt");
}

#[tokio::test]
async fn test_completion_engine_provider_priority() {
    let config = CompletionEngineConfig::default();
    let mut engine = CompletionEngine::new(config).unwrap();

    // 创建不同优先级的提供者
    struct PriorityProvider {
        name: &'static str,
        priority: i32,
        items: Vec<CompletionItem>,
    }

    #[async_trait]
    impl CompletionProvider for PriorityProvider {
        fn name(&self) -> &'static str {
            self.name
        }

        fn should_provide(&self, _context: &CompletionContext) -> bool {
            true
        }

        async fn provide_completions(
            &self,
            _context: &CompletionContext,
        ) -> AppResult<Vec<CompletionItem>> {
            Ok(self.items.clone())
        }

        fn priority(&self) -> i32 {
            self.priority
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    let low_priority = Arc::new(PriorityProvider {
        name: "low",
        priority: 1,
        items: vec![CompletionItem::new(
            "low.txt".to_string(),
            CompletionType::File,
        )],
    });

    let high_priority = Arc::new(PriorityProvider {
        name: "high",
        priority: 10,
        items: vec![CompletionItem::new(
            "high.txt".to_string(),
            CompletionType::File,
        )],
    });

    // 先添加低优先级，再添加高优先级
    engine.add_provider(low_priority);
    engine.add_provider(high_priority);

    // 验证提供者按优先级排序
    assert_eq!(engine.providers[0].name(), "high");
    assert_eq!(engine.providers[1].name(), "low");
}

#[tokio::test]
async fn test_completion_engine_max_results() {
    let mut config = CompletionEngineConfig::default();
    config.max_results = 2; // 限制最大结果数

    let mut engine = CompletionEngine::new(config).unwrap();

    // 创建返回多个结果的提供者
    let items = vec![
        CompletionItem::new("item1".to_string(), CompletionType::File),
        CompletionItem::new("item2".to_string(), CompletionType::File),
        CompletionItem::new("item3".to_string(), CompletionType::File),
        CompletionItem::new("item4".to_string(), CompletionType::File),
    ];

    let provider = Arc::new(MockProvider::new("test", true, items));
    engine.add_provider(provider);

    let context = CompletionContext::new("item".to_string(), 4, PathBuf::from("/tmp"));

    let response = engine.get_completions(&context).await.unwrap();

    // 应该只返回最大结果数
    assert_eq!(response.items.len(), 2);
    assert!(response.has_more);
}

#[test]
fn test_completion_engine_config() {
    let config = CompletionEngineConfig {
        max_results: 50,
        provider_timeout: Duration::from_millis(500),
        enable_cache: true,
        cache_capacity: 200,
        cache_ttl: Duration::from_secs(600),
    };

    assert_eq!(config.max_results, 50);
    assert_eq!(config.provider_timeout, Duration::from_millis(500));
    assert!(config.enable_cache);
    assert_eq!(config.cache_capacity, 200);
    assert_eq!(config.cache_ttl, Duration::from_secs(600));
}
