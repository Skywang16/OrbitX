//! 系统命令补全提供者测试

use std::path::PathBuf;
use terminal_lib::completion::{
    SystemCommandsProvider, CompletionContext, CompletionProvider
};

#[tokio::test]
async fn test_system_commands_provider_creation() {
    let provider = SystemCommandsProvider::new();
    assert_eq!(provider.name(), "system_commands");
    assert_eq!(provider.priority(), 8);
    assert_eq!(provider.command_count().await, 0);
}

#[tokio::test]
async fn test_add_and_find_commands() {
    let provider = SystemCommandsProvider::new();

    // 手动添加一些测试命令
    provider.add_command("ls".to_string()).await.unwrap();
    provider.add_command("grep".to_string()).await.unwrap();
    provider.add_command("find".to_string()).await.unwrap();

    assert_eq!(provider.command_count().await, 3);
    assert!(provider.has_command("ls").await);
    assert!(provider.has_command("grep").await);
    assert!(!provider.has_command("nonexistent").await);

    // 测试模糊匹配
    let matches = provider.get_matching_commands("l").await.unwrap();
    assert!(!matches.is_empty());
    
    let commands: Vec<&String> = matches.iter().map(|item| &item.text).collect();
    assert!(commands.contains(&&"ls".to_string()));
}

#[test]
fn test_should_provide() {
    let provider = SystemCommandsProvider::new();
    
    // 应该提供补全的情况（第一个词）
    let context = CompletionContext::new(
        "ls".to_string(),
        2,
        PathBuf::from("/tmp")
    );
    assert!(provider.should_provide(&context));
    
    // 应该提供补全的情况（命令开始）
    let context = CompletionContext::new(
        "gre".to_string(),
        3,
        PathBuf::from("/tmp")
    );
    assert!(provider.should_provide(&context));
    
    // 不应该提供补全的情况（包含路径）
    let context = CompletionContext::new(
        "./script".to_string(),
        8,
        PathBuf::from("/tmp")
    );
    assert!(!provider.should_provide(&context));
    
    // 不应该提供补全的情况（不是第一个词）
    let context = CompletionContext::new(
        "ls -la /home".to_string(),
        11,
        PathBuf::from("/tmp")
    );
    assert!(!provider.should_provide(&context));
}

#[tokio::test]
async fn test_provide_completions() {
    let provider = SystemCommandsProvider::new();

    // 添加测试命令
    provider.add_command("ls".to_string()).await.unwrap();
    provider.add_command("less".to_string()).await.unwrap();
    provider.add_command("locate".to_string()).await.unwrap();
    
    let context = CompletionContext::new(
        "l".to_string(),
        1,
        PathBuf::from("/tmp")
    );
    
    let completions = provider.provide_completions(&context).await.unwrap();
    // 由于provide_completions会初始化系统命令，所以结果会包含系统中的所有l开头的命令
    // 我们只验证结果不为空，并且包含我们添加的命令
    assert!(!completions.is_empty());

    // 验证包含我们添加的命令
    let command_texts: Vec<&String> = completions.iter().map(|item| &item.text).collect();
    assert!(command_texts.contains(&&"ls".to_string()));
    assert!(command_texts.contains(&&"less".to_string()));
    assert!(command_texts.contains(&&"locate".to_string()));
    
    // 验证所有补全项都是命令类型
    for item in &completions {
        assert_eq!(item.completion_type, "command");
    }
}

#[tokio::test]
async fn test_empty_input() {
    let provider = SystemCommandsProvider::new();
    
    let context = CompletionContext::new(
        "".to_string(),
        0,
        PathBuf::from("/tmp")
    );
    
    let completions = provider.provide_completions(&context).await.unwrap();
    assert!(completions.is_empty());
}

#[tokio::test]
async fn test_fuzzy_matching_scores() {
    let provider = SystemCommandsProvider::new();

    // 添加测试命令
    provider.add_command("list".to_string()).await.unwrap();
    provider.add_command("ls".to_string()).await.unwrap();
    provider.add_command("less".to_string()).await.unwrap();
    
    let context = CompletionContext::new(
        "ls".to_string(),
        2,
        PathBuf::from("/tmp")
    );
    
    let completions = provider.provide_completions(&context).await.unwrap();
    
    // 验证结果按分数排序（精确匹配应该在前面）
    if completions.len() >= 2 {
        // "ls" 应该比 "less" 或 "list" 有更高的分数
        let ls_item = completions.iter().find(|item| item.text == "ls");
        let other_items: Vec<_> = completions.iter().filter(|item| item.text != "ls").collect();
        
        if let Some(ls) = ls_item {
            for other in other_items {
                assert!(ls.score >= other.score, 
                    "ls (score: {}) should have higher score than {} (score: {})", 
                    ls.score, other.text, other.score);
            }
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn test_is_executable() {
    use tempfile::NamedTempFile;
    use std::os::unix::fs::PermissionsExt;
    use tokio::fs;
    
    let provider = SystemCommandsProvider::new();
    
    // 创建一个临时文件
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();
    
    // 默认情况下不可执行
    assert!(!provider.is_executable(path).await);
    
    // 设置可执行权限
    let mut perms = fs::metadata(path).await.unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).await.unwrap();
    
    // 现在应该是可执行的
    assert!(provider.is_executable(path).await);
}

#[tokio::test]
async fn test_command_deduplication() {
    let provider = SystemCommandsProvider::new();

    // 添加重复的命令
    provider.add_command("ls".to_string()).await.unwrap();
    provider.add_command("ls".to_string()).await.unwrap();
    provider.add_command("grep".to_string()).await.unwrap();

    // 应该只有2个唯一命令
    assert_eq!(provider.command_count().await, 2);
    assert!(provider.has_command("ls").await);
    assert!(provider.has_command("grep").await);
}
