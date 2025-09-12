use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;

use terminal_lib::ai::{create_context_manager, create_context_manager_with_config, ContextConfig};
use terminal_lib::storage::database::{DatabaseManager, DatabaseOptions};
use terminal_lib::storage::paths::StoragePaths;
use terminal_lib::storage::repositories::conversations::{Conversation, Message};
use terminal_lib::storage::repositories::Repository;
use terminal_lib::storage::repositories::RepositoryManager;

// 测试使用真实的 SQLite + SQL 脚本初始化，但将数据写入临时目录下，避免污染用户环境
async fn setup_repos() -> (Arc<DatabaseManager>, RepositoryManager) {
    // 使用系统临时目录下的独立子目录
    let mut app_dir: PathBuf = std::env::temp_dir();
    let unique = format!(
        "orbitx_test_{}_{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );
    app_dir.push(unique);

    let paths = StoragePaths::new(app_dir).expect("failed to build storage paths");

    let db = Arc::new(
        DatabaseManager::new(paths, DatabaseOptions::default())
            .await
            .expect("failed to create DatabaseManager"),
    );

    db.initialize().await.expect("failed to initialize db");

    let repos = RepositoryManager::new(Arc::clone(&db));
    (db, repos)
}

async fn create_conversation_with_messages(
    repos: &RepositoryManager,
    msgs: &[(&str, &str)], // (role, content)
) -> i64 {
    // 先创建会话
    let conv = Conversation::new("Test Conversation".to_string());
    let conv_id = repos
        .conversations()
        .save(&conv)
        .await
        .expect("failed to save conversation");

    // 插入消息
    for (role, content) in msgs {
        let mut m = Message::new(conv_id, role.to_string(), content.to_string());
        // 与实现保持一致：status 使用允许的值
        m.status = Some("complete".to_string());
        repos
            .conversations()
            .ai_conversation_save_message(&m)
            .await
            .expect("failed to save message");
    }

    conv_id
}

#[tokio::test]
async fn test_build_context_without_compression() {
    let (_db, repos) = setup_repos().await;

    // 少量短消息，不触发压缩
    let conv_id = create_conversation_with_messages(
        &repos,
        &[
            ("user", "你好"),
            ("assistant", "你好，有什么可以帮你？"),
            ("user", "请总结一下今天的待办"),
        ],
    )
    .await;

    let ctx_mgr = create_context_manager();
    let result = ctx_mgr
        .build_context(&repos, conv_id, None)
        .await
        .expect("build_context failed");

    assert_eq!(result.original_count, 3);
    assert_eq!(result.messages.len(), 3);
    assert!(
        !result.compressed,
        "should not be compressed for small history"
    );
    assert!(result.token_count > 0);
}

#[tokio::test]
async fn test_build_context_with_compression_path_executes() {
    let (_db, repos) = setup_repos().await;

    // 构造较多消息，确保触发压缩逻辑
    // 通过降低 max_tokens/threshold 来更容易触发压缩
    let cfg = ContextConfig {
        max_tokens: 200,          // 非常小的 token 上限
        compress_threshold: 0.50, // 一半就压缩
    };
    let ctx_mgr = create_context_manager_with_config(cfg);

    // 插入 20 条较长消息
    let mut data: Vec<(&str, &str)> = Vec::new();
    for i in 0..20 {
        if i % 2 == 0 {
            data.push(("user", "这是一段较长的测试消息，用于触发上下文压缩逻辑。"));
        } else {
            data.push((
                "assistant",
                "收到，我们会对早期消息进行摘要压缩并保留最近消息。",
            ));
        }
    }
    let conv_id = create_conversation_with_messages(&repos, &data).await;

    let result = ctx_mgr
        .build_context(&repos, conv_id, None)
        .await
        .expect("build_context failed");

    assert_eq!(result.original_count, 20);
    // 触发压缩后，消息数应 <= 原始数量
    assert!(result.messages.len() <= 20);
    assert!(result.compressed, "expected compression to be triggered");
}

#[tokio::test]
async fn test_build_prompt_contains_sections() {
    let (_db, repos) = setup_repos().await;

    let conv_id = create_conversation_with_messages(
        &repos,
        &[
            ("user", "如何在Rust中写测试？"),
            (
                "assistant",
                "你可以使用 #[tokio::test] 或 #[test] 创建异步/同步测试。",
            ),
        ],
    )
    .await;

    let ctx_mgr = create_context_manager();
    let prompt = ctx_mgr
        .build_prompt(&repos, conv_id, "请给出一个简单的示例", None, Some("/tmp"))
        .await
        .expect("build_prompt failed");

    // 关键段落大致存在即可
    assert!(prompt.contains("【当前环境】"));
    assert!(prompt.contains("【对话历史】"));
    assert!(prompt.contains("【当前问题】"));
}
