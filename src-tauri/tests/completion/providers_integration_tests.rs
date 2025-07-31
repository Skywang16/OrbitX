//! 补全提供者集成测试

use std::path::PathBuf;
use terminal_lib::completion::{CompletionContext, CompletionEngine, CompletionEngineConfig};

#[tokio::test]
async fn test_default_providers_count() {
    let config = CompletionEngineConfig::default();
    let engine = CompletionEngine::with_default_providers(config)
        .await
        .unwrap();

    // 应该只有3个提供者：文件系统、系统命令、历史记录
    // 不应该包含环境变量提供者
    assert_eq!(engine.providers.len(), 3);

    let provider_names: Vec<&str> = engine.providers.iter().map(|p| p.name()).collect();

    assert!(provider_names.contains(&"filesystem"));
    assert!(provider_names.contains(&"system_commands"));
    assert!(provider_names.contains(&"history"));

    // 确保没有环境变量提供者
    assert!(!provider_names.contains(&"environment"));
}

#[tokio::test]
async fn test_providers_priority_order() {
    let config = CompletionEngineConfig::default();
    let engine = CompletionEngine::with_default_providers(config)
        .await
        .unwrap();

    // 验证提供者按优先级排序
    let priorities: Vec<i32> = engine.providers.iter().map(|p| p.priority()).collect();

    // 应该按降序排列
    for i in 1..priorities.len() {
        assert!(
            priorities[i - 1] >= priorities[i],
            "提供者优先级应该按降序排列，但发现 {} < {}",
            priorities[i - 1],
            priorities[i]
        );
    }
}

#[tokio::test]
async fn test_history_provider_has_highest_priority() {
    let config = CompletionEngineConfig::default();
    let engine = CompletionEngine::with_default_providers(config)
        .await
        .unwrap();

    // 验证历史命令提供者具有最高优先级
    let provider_info: Vec<(&str, i32)> = engine
        .providers
        .iter()
        .map(|p| (p.name(), p.priority()))
        .collect();

    // 历史命令提供者应该排在第一位（最高优先级）
    assert_eq!(
        provider_info[0].0, "history",
        "历史命令提供者应该具有最高优先级"
    );
    assert_eq!(provider_info[0].1, 15, "历史命令提供者优先级应该是15");

    // 验证其他提供者的优先级
    let filesystem_priority = provider_info
        .iter()
        .find(|(name, _)| *name == "filesystem")
        .map(|(_, priority)| *priority)
        .unwrap();
    let system_commands_priority = provider_info
        .iter()
        .find(|(name, _)| *name == "system_commands")
        .map(|(_, priority)| *priority)
        .unwrap();

    assert_eq!(filesystem_priority, 10, "文件系统提供者优先级应该是10");
    assert_eq!(system_commands_priority, 8, "系统命令提供者优先级应该是8");

    // 确保历史命令优先级最高
    assert!(
        provider_info[0].1 > filesystem_priority,
        "历史命令优先级应该高于文件系统"
    );
    assert!(
        provider_info[0].1 > system_commands_priority,
        "历史命令优先级应该高于系统命令"
    );
}

#[tokio::test]
async fn test_completion_without_environment_variables() {
    let config = CompletionEngineConfig::default();
    let engine = CompletionEngine::with_default_providers(config)
        .await
        .unwrap();

    // 测试包含$符号的输入，应该不会触发环境变量补全
    let context = CompletionContext::new("echo $HOME".to_string(), 9, PathBuf::from("/tmp"));

    let response = engine.get_completions(&context).await.unwrap();

    // 验证没有环境变量类型的补全项
    for item in &response.items {
        assert_ne!(
            item.completion_type, "environment",
            "不应该有环境变量类型的补全项"
        );
        assert!(
            !item.text.starts_with("$"),
            "补全项不应该以$开头: {}",
            item.text
        );
    }
}

#[tokio::test]
async fn test_filesystem_provider_works() {
    let config = CompletionEngineConfig::default();
    let engine = CompletionEngine::with_default_providers(config)
        .await
        .unwrap();

    // 测试文件系统补全
    let context = CompletionContext::new("/".to_string(), 1, PathBuf::from("/tmp"));

    let response = engine.get_completions(&context).await.unwrap();

    // 应该有文件系统相关的补全
    let has_filesystem_items = response
        .items
        .iter()
        .any(|item| item.completion_type == "file" || item.completion_type == "directory");

    assert!(has_filesystem_items, "应该有文件系统补全项");
}

#[tokio::test]
async fn test_system_commands_provider_works() {
    let config = CompletionEngineConfig::default();
    let engine = CompletionEngine::with_default_providers(config)
        .await
        .unwrap();

    // 测试系统命令补全
    let context = CompletionContext::new("l".to_string(), 1, PathBuf::from("/tmp"));

    let response = engine.get_completions(&context).await.unwrap();

    // 应该有命令类型的补全
    let has_command_items = response
        .items
        .iter()
        .any(|item| item.completion_type == "command");

    assert!(has_command_items, "应该有系统命令补全项");
}
