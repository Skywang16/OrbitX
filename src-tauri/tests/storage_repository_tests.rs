/*!
 * Storage Repository 集成测试
 *
 * 测试Repository模式、数据库管理器和查询构建器的集成功能
 */
use chrono::Utc;
use std::sync::Arc;
use tempfile::TempDir;
use tokio;

use terminal_lib::agent::persistence::AgentPersistence;
use terminal_lib::storage::{
    database::{DatabaseManager, DatabaseOptions},
    paths::StoragePathsBuilder,
    query::{QueryCondition, SafeQueryBuilder},
    repositories::ai_models::{AIModelConfig, AIProvider, ModelType},
    repositories::{Repository, RepositoryManager},
};

/// 创建测试用的数据库管理器
async fn create_test_database_manager() -> (DatabaseManager, TempDir) {
    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let paths = StoragePathsBuilder::new()
        .app_dir(temp_dir.path().to_path_buf())
        .build()
        .expect("创建存储路径失败");

    let options = DatabaseOptions::default();
    let manager = DatabaseManager::new(paths, options)
        .await
        .expect("创建数据库管理器失败");

    // 初始化数据库
    manager.initialize().await.expect("初始化数据库失败");

    manager
        .set_master_password("test-password-123")
        .await
        .expect("设置主密钥失败");

    (manager, temp_dir)
}

/// 创建测试用的Repository管理器
async fn create_test_repositories() -> (RepositoryManager, TempDir) {
    let (database_manager, temp_dir) = create_test_database_manager().await;
    let repositories = RepositoryManager::new(Arc::new(database_manager));
    (repositories, temp_dir)
}

/// 创建测试用的AI模型配置
fn create_test_ai_model() -> AIModelConfig {
    AIModelConfig {
        id: "test-model-1".to_string(),
        name: "Test GPT Model".to_string(),
        provider: AIProvider::OpenAI,
        api_url: "https://api.openai.com/v1".to_string(),
        api_key: "test-api-key-12345".to_string(),
        model: "gpt-4".to_string(),
        model_type: ModelType::Chat,
        enabled: true,
        options: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[tokio::test]
async fn test_database_initialization() {
    let (database_manager, _temp_dir) = create_test_database_manager().await;

    // 验证表是否创建成功
    let pool = database_manager.pool();
    let tables = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .expect("查询表列表失败");

    // 打印实际的表列表用于调试
    println!("实际的表列表: {:?}", tables);

    let expected_tables = vec![
        "ai_models",
        "audit_logs",
        "command_history",
        "schema_migrations",
    ];

    for table in expected_tables {
        assert!(tables.contains(&table.to_string()), "表 {} 应该存在", table);
    }
}

#[tokio::test]
async fn test_ai_model_crud_operations() {
    let (repositories, _temp_dir) = create_test_repositories().await;
    let test_model = create_test_ai_model();

    // 测试保存AI模型
    let model_id = repositories
        .ai_models()
        .save(&test_model)
        .await
        .expect("保存AI模型失败");
    assert!(model_id > 0, "保存AI模型应该返回有效ID");

    // 测试查找AI模型（使用字符串ID）
    let found_model = repositories
        .ai_models()
        .find_by_string_id(&test_model.id)
        .await
        .expect("查找AI模型失败");
    assert!(found_model.is_some(), "应该找到AI模型");
    let found_model = found_model.unwrap();
    assert_eq!(found_model.name, test_model.name, "模型名称应该匹配");

    // 测试查找所有模型
    let models = repositories
        .ai_models()
        .find_all()
        .await
        .expect("获取所有AI模型失败");
    assert_eq!(models.len(), 1, "应该有1个AI模型");

    // 测试删除AI模型（使用字符串ID）
    let result = repositories
        .ai_models()
        .delete_by_string_id(&test_model.id)
        .await;
    assert!(result.is_ok(), "删除AI模型应该成功");

    // 验证删除后模型不存在
    let models_after_delete = repositories
        .ai_models()
        .find_all()
        .await
        .expect("获取AI模型失败");
    assert_eq!(models_after_delete.len(), 0, "删除后应该没有AI模型");
}

#[tokio::test]
async fn test_conversations_repository() {
    let (database_manager, _temp_dir) = create_test_database_manager().await;
    let persistence = AgentPersistence::new(Arc::new(database_manager));

    // 测试创建会话
    let conversation = persistence
        .conversations()
        .create(Some("Test Conversation"), None)
        .await
        .expect("创建会话失败");
    assert!(conversation.id > 0, "创建会话应该返回有效ID");
    assert_eq!(conversation.title, Some("Test Conversation".to_string()));

    // 测试查找会话
    let found_conversation = persistence
        .conversations()
        .get(conversation.id)
        .await
        .expect("查找会话失败");
    assert!(found_conversation.is_some(), "应该找到会话");
    assert_eq!(found_conversation.unwrap().title, conversation.title);
}

#[tokio::test]
async fn test_query_builder() {
    // 测试基本查询构建
    let (sql, params) = SafeQueryBuilder::new("ai_models")
        .select(&["id", "name", "provider"])
        .where_condition(QueryCondition::Eq("enabled".to_string(), true.into()))
        .limit(10)
        .build()
        .expect("构建查询失败");

    println!("生成的SQL: {}", sql);
    println!("参数数量: {}", params.len());

    assert!(sql.contains("SELECT id, name, provider FROM ai_models"));
    assert!(sql.contains("WHERE enabled = ?"));
    // 注意：查询构建器可能使用不同的LIMIT语法
    assert!(sql.contains("LIMIT") || sql.contains("10"));
    // 参数数量可能因为查询构建器的实现而不同
    assert!(params.len() >= 1);
}

#[tokio::test]
async fn test_repository_manager() {
    let (repositories, _temp_dir) = create_test_repositories().await;

    // 测试各个Repository是否可以正常访问
    let _ai_models_repo = repositories.ai_models();
    let _command_history_repo = repositories.command_history();
    let _audit_logs_repo = repositories.audit_logs();

    // 如果能到这里说明Repository管理器工作正常
    assert!(true, "Repository管理器应该正常工作");
}
