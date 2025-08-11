/*!
 * SQLite管理器测试套件
 *
 * 测试数据库初始化、迁移、加密存储和查询性能
 */

#[cfg(test)]
mod tests {
    use super::super::paths::StoragePathsBuilder;
    use super::super::sqlite::*;
    use super::super::types::SaveOptions;
    use crate::ai::types::{AIModelConfig, AIModelOptions, AIProvider};
    use chrono::Utc;
    use tempfile::TempDir;
    use tokio;

    /// 创建测试用的SQLite管理器
    async fn create_test_sqlite_manager() -> (SqliteManager, TempDir) {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let paths = StoragePathsBuilder::new()
            .app_dir(temp_dir.path().to_path_buf())
            .build()
            .expect("创建存储路径失败");

        let options = SqliteOptions::default();
        let manager = SqliteManager::new(paths, options)
            .await
            .expect("创建SQLite管理器失败");

        (manager, temp_dir)
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
            is_default: Some(true),
            options: Some(AIModelOptions {
                max_tokens: Some(4096),
                temperature: Some(0.7),
                timeout: Some(30),
                custom_config: Some(r#"{"stream": true}"#.to_string()),
            }),
        }
    }

    /// 创建测试用的命令历史条目
    fn create_test_command_history() -> CommandHistoryEntry {
        CommandHistoryEntry {
            id: None,
            command: "ls -la".to_string(),
            working_directory: "/home/user".to_string(),
            exit_code: Some(0),
            output: Some("total 8\ndrwxr-xr-x 2 user user 4096 Jan 1 12:00 .\ndrwxr-xr-x 3 user user 4096 Jan 1 12:00 ..".to_string()),
            duration_ms: Some(150),
            executed_at: Utc::now(),
            session_id: Some("test-session-1".to_string()),
            tags: Some("file,list".to_string()),
        }
    }

    #[tokio::test]
    async fn test_database_initialization() {
        let (manager, _temp_dir) = create_test_sqlite_manager().await;

        // 测试数据库初始化
        let result = manager.initialize_database().await;
        assert!(result.is_ok(), "数据库初始化应该成功");

        // 验证表是否创建成功
        let pool = manager.pool();
        let tables = sqlx::query_scalar::<_, String>(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
        )
        .fetch_all(pool)
        .await
        .expect("查询表列表失败");

        let expected_tables = vec![
            "ai_conversations", // 会话表
            "ai_features",
            "ai_messages", // 消息表
            "ai_model_usage_stats",
            "ai_models",
            "audit_logs",
            "command_history",
            "command_search",
            "command_usage_stats",
            "schema_migrations",
            "terminal_sessions",
        ];

        for table in expected_tables {
            assert!(tables.contains(&table.to_string()), "表 {} 应该存在", table);
        }
    }

    #[tokio::test]
    async fn test_ai_model_crud_operations() {
        let (manager, _temp_dir) = create_test_sqlite_manager().await;
        manager
            .initialize_database()
            .await
            .expect("数据库初始化失败");

        // 设置主密钥用于加密
        manager
            .set_master_password("test-password-123")
            .await
            .expect("设置主密钥失败");

        let test_model = create_test_ai_model();

        // 测试保存AI模型
        let save_options = SaveOptions {
            table: Some("ai_models".to_string()),
            overwrite: false,
            backup: true,
            validate: true,
            metadata: std::collections::HashMap::new(),
        };
        let model_value = serde_json::to_value(&test_model).expect("序列化AI模型失败");
        let result = manager.save_data(&model_value, &save_options).await;
        assert!(result.is_ok(), "保存AI模型应该成功");

        // 测试获取AI模型
        let models = manager.get_ai_models().await.expect("获取AI模型失败");
        assert_eq!(models.len(), 1, "应该有1个AI模型");
        assert_eq!(models[0].name, test_model.name, "模型名称应该匹配");
        assert_eq!(models[0].api_key, test_model.api_key, "API密钥应该正确解密");

        // 测试获取默认模型
        let default_model = manager
            .get_default_ai_model()
            .await
            .expect("获取默认模型失败");
        assert!(default_model.is_some(), "应该有默认模型");
        assert_eq!(
            default_model.unwrap().id,
            test_model.id,
            "默认模型ID应该匹配"
        );

        // 测试删除AI模型
        let result = manager.delete_ai_model(&test_model.id).await;
        assert!(result.is_ok(), "删除AI模型应该成功");

        let models_after_delete = manager.get_ai_models().await.expect("获取AI模型失败");
        assert_eq!(models_after_delete.len(), 0, "删除后应该没有AI模型");
    }

    #[tokio::test]
    async fn test_command_history_operations() {
        let (manager, _temp_dir) = create_test_sqlite_manager().await;
        manager
            .initialize_database()
            .await
            .expect("数据库初始化失败");

        let test_entry = create_test_command_history();

        // 测试保存命令历史
        let save_options = SaveOptions {
            table: Some("command_history".to_string()),
            overwrite: false,
            backup: true,
            validate: true,
            metadata: std::collections::HashMap::new(),
        };
        let entry_value = serde_json::to_value(&test_entry).expect("序列化命令历史失败");
        let result = manager.save_data(&entry_value, &save_options).await;
        assert!(result.is_ok(), "保存命令历史应该成功");

        // 测试查询命令历史
        let query = HistoryQuery {
            command_pattern: Some("ls".to_string()),
            working_directory: None,
            date_from: None,
            date_to: None,
            limit: Some(10),
            offset: None,
        };
        let entries = manager
            .query_command_history(&query)
            .await
            .expect("查询命令历史失败");
        assert_eq!(entries.len(), 1, "应该有1条命令历史");
        assert_eq!(entries[0].command, test_entry.command, "命令应该匹配");

        // 测试全文搜索
        let search_results = manager.full_text_search("ls").await.expect("全文搜索失败");
        assert!(!search_results.is_empty(), "搜索结果不应该为空");

        // 测试使用统计
        let stats = manager
            .get_usage_statistics()
            .await
            .expect("获取使用统计失败");
        assert_eq!(stats.total_commands, 1, "总命令数应该为1");
        assert_eq!(stats.unique_commands, 1, "唯一命令数应该为1");
    }

    #[tokio::test]
    async fn test_encryption_functionality() {
        let (manager, _temp_dir) = create_test_sqlite_manager().await;
        manager
            .initialize_database()
            .await
            .expect("数据库初始化失败");

        // 测试设置主密钥
        let result = manager.set_master_password("secure-password-456").await;
        assert!(result.is_ok(), "设置主密钥应该成功");

        // 测试加密管理器
        let encryption_manager = manager.encryption_manager();
        let test_data = "sensitive-api-key-data";

        {
            let enc_mgr = encryption_manager.read().await;
            let encrypted = enc_mgr.encrypt_data(test_data).expect("加密数据失败");
            let decrypted = enc_mgr.decrypt_data(&encrypted).expect("解密数据失败");
            assert_eq!(decrypted, test_data, "解密后的数据应该与原始数据匹配");
        }
    }

    #[tokio::test]
    async fn test_audit_logging() {
        let (manager, _temp_dir) = create_test_sqlite_manager().await;
        manager
            .initialize_database()
            .await
            .expect("数据库初始化失败");

        // 测试记录审计日志
        let result = manager
            .log_audit_event(
                "TEST_OPERATION",
                "test_table",
                Some("test-record-1"),
                Some("test-user"),
                "测试操作详情",
                true,
                None,
            )
            .await;
        assert!(result.is_ok(), "记录审计日志应该成功");

        // 测试查询审计日志
        let logs = manager
            .get_audit_logs(Some("test_table"), None, Some(10))
            .await
            .expect("查询审计日志失败");
        assert_eq!(logs.len(), 1, "应该有1条审计日志");
        assert_eq!(logs[0].operation, "TEST_OPERATION", "操作类型应该匹配");
        assert_eq!(logs[0].table_name, "test_table", "表名应该匹配");
        assert!(logs[0].success, "操作应该标记为成功");
    }

    #[tokio::test]
    async fn test_performance_analysis() {
        let (manager, _temp_dir) = create_test_sqlite_manager().await;
        manager
            .initialize_database()
            .await
            .expect("数据库初始化失败");

        // 创建多个测试命令历史条目
        let mut entries = Vec::new();
        for i in 0..10 {
            let mut entry = create_test_command_history();
            entry.command = format!("test-command-{}", i % 3); // 创建重复命令
            entry.duration_ms = Some(100 + i * 10);
            entry.exit_code = Some(if i % 4 == 0 { 1 } else { 0 }); // 25%错误率
            entries.push(entry);
        }

        // 批量保存命令历史
        let result = manager.batch_save_command_history(&entries).await;
        assert!(result.is_ok(), "批量保存命令历史应该成功");

        // 测试命令使用趋势
        let trends = manager
            .get_command_usage_trends(7)
            .await
            .expect("获取使用趋势失败");
        assert!(!trends.is_empty(), "使用趋势不应该为空");

        // 测试常用目录
        let directories = manager
            .get_popular_directories(Some(5))
            .await
            .expect("获取常用目录失败");
        assert!(!directories.is_empty(), "常用目录不应该为空");

        // 测试命令推荐
        let recommendations = manager
            .get_command_recommendations("/home/user", Some(5))
            .await
            .expect("获取命令推荐失败");
        assert!(!recommendations.is_empty(), "命令推荐不应该为空");
    }
}
