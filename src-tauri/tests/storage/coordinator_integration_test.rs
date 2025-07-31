/*!
 * 存储协调器集成测试
 *
 * 测试三层存储的协调工作，验证缓存策略和性能优化，
 * 测试错误恢复和系统稳定性
 */

use serde_json::json;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

use terminal_lib::storage::types::{DataQuery, SaveOptions, SessionState};
use terminal_lib::storage::{
    CacheConfig, MessagePackOptions, SqliteOptions, StorageCoordinator, StorageCoordinatorOptions,
    StorageLayer, StoragePaths, TomlConfigOptions,
};

/// 创建测试用的存储协调器
async fn create_test_coordinator() -> (StorageCoordinator, TempDir) {
    let temp_dir = TempDir::new().expect("创建临时目录失败");
    let paths = StoragePaths::new(temp_dir.path().to_path_buf()).expect("创建存储路径失败");

    let options = StorageCoordinatorOptions {
        cache_enabled: true,
        cache_size_limit: 1024 * 1024, // 1MB
        events_enabled: true,
        toml_options: TomlConfigOptions::default(),
        messagepack_options: MessagePackOptions::default(),
        sqlite_options: SqliteOptions::default(),
        cache_config: CacheConfig {
            memory_limit: 512 * 1024, // 512KB
            lru_capacity: 100,
            default_ttl: Duration::from_secs(300),
            disk_cache_dir: paths.cache_dir.clone(),
            disk_cache_enabled: true,
            stats_update_interval: Duration::from_secs(10),
        },
    };

    let coordinator = StorageCoordinator::new(paths, options)
        .await
        .expect("创建存储协调器失败");

    (coordinator, temp_dir)
}

#[tokio::test]
async fn test_coordinator_initialization() {
    let (coordinator, _temp_dir) = create_test_coordinator().await;

    // 测试健康检查
    let health = coordinator.health_check().await.expect("健康检查失败");
    println!("健康检查结果: {:?}", health);

    // 测试缓存统计
    let cache_stats = coordinator.get_cache_stats().await;
    println!("缓存统计: {:?}", cache_stats);

    // 测试存储统计
    let storage_stats = coordinator
        .get_storage_stats()
        .await
        .expect("获取存储统计失败");
    println!("存储统计: {:?}", storage_stats);
}

#[tokio::test]
async fn test_config_operations() {
    let (coordinator, _temp_dir) = create_test_coordinator().await;

    // 测试获取默认配置
    let app_config = coordinator.get_config("app").await.expect("获取配置失败");
    println!("默认应用配置: {:?}", app_config);

    // 测试更新配置
    let new_config = json!({
        "name": "TermX Test",
        "version": "1.0.1",
        "debug": true
    });

    coordinator
        .update_config("app", new_config.clone())
        .await
        .expect("更新配置失败");

    // 验证配置已更新
    let updated_config = coordinator
        .get_config("app")
        .await
        .expect("获取更新后配置失败");
    assert_eq!(updated_config, new_config);

    // 测试缓存命中
    let cached_config = coordinator
        .get_config("app")
        .await
        .expect("从缓存获取配置失败");
    assert_eq!(cached_config, new_config);
}

#[tokio::test]
async fn test_session_state_operations() {
    let (coordinator, _temp_dir) = create_test_coordinator().await;

    // 创建测试会话状态
    let mut session_state = SessionState::default();
    session_state.window_state.size = (1920, 1080);
    session_state.ui_state.current_theme = "light".to_string();

    // 测试保存会话状态
    coordinator
        .save_session_state(&session_state)
        .await
        .expect("保存会话状态失败");

    // 测试加载会话状态
    let loaded_state = coordinator
        .load_session_state()
        .await
        .expect("加载会话状态失败")
        .expect("会话状态应该存在");

    assert_eq!(loaded_state.window_state.size, (1920, 1080));
    assert_eq!(loaded_state.ui_state.current_theme, "light");
}

#[tokio::test]
async fn test_data_operations() {
    let (coordinator, _temp_dir) = create_test_coordinator().await;

    // 测试保存数据
    let test_data = json!({
        "id": "test-001",
        "name": "Test Command",
        "command": "echo hello",
        "timestamp": "2024-01-01T00:00:00Z"
    });

    let save_options = SaveOptions::new()
        .table("command_history")
        .validate(true)
        .backup(true);

    coordinator
        .save_data(&test_data, &save_options)
        .await
        .expect("保存数据失败");

    // 测试查询数据
    let query = DataQuery::new("SELECT * FROM command_history WHERE id = ?")
        .with_param("id", json!("test-001"))
        .with_limit(10);

    let results = coordinator.query_data(&query).await.expect("查询数据失败");

    assert!(!results.is_empty());
    println!("查询结果: {:?}", results);
}

#[tokio::test]
async fn test_cache_performance() {
    let (coordinator, _temp_dir) = create_test_coordinator().await;

    // 预加载缓存
    coordinator.preload_cache().await.expect("预加载缓存失败");

    let start_time = std::time::Instant::now();

    // 多次访问相同配置，测试缓存性能
    for _ in 0..100 {
        let _ = coordinator.get_config("app").await.expect("获取配置失败");
    }

    let cache_access_time = start_time.elapsed();
    println!("缓存访问时间 (100次): {:?}", cache_access_time);

    // 清理缓存后重新访问
    coordinator.clear_cache().await.expect("清理缓存失败");

    let start_time = std::time::Instant::now();

    // 访问配置，测试无缓存性能
    for _ in 0..10 {
        let _ = coordinator.get_config("app").await.expect("获取配置失败");
    }

    let no_cache_access_time = start_time.elapsed();
    println!("无缓存访问时间 (10次): {:?}", no_cache_access_time);

    // 获取缓存统计
    let cache_stats = coordinator.get_cache_stats().await;
    println!("缓存统计: {:?}", cache_stats);
}

#[tokio::test]
async fn test_error_recovery() {
    let (coordinator, temp_dir) = create_test_coordinator().await;

    // 测试健康检查
    let health = coordinator.health_check().await.expect("健康检查失败");
    println!("初始健康状态: {:?}", health);

    // 模拟配置文件损坏
    let config_path = temp_dir.path().join("config").join("config.toml");
    if config_path.exists() {
        tokio::fs::write(&config_path, "invalid toml content {{{")
            .await
            .expect("写入无效配置失败");
    }

    // 再次检查健康状态
    let health_after_corruption = coordinator.health_check().await.expect("健康检查失败");
    println!("损坏后健康状态: {:?}", health_after_corruption);

    // 测试自动修复
    let repair_results = coordinator.auto_repair().await.expect("自动修复失败");
    println!("修复结果: {:?}", repair_results);

    // 验证修复后的健康状态
    let health_after_repair = coordinator.health_check().await.expect("健康检查失败");
    println!("修复后健康状态: {:?}", health_after_repair);
}

#[tokio::test]
async fn test_backup_and_restore() {
    let (coordinator, _temp_dir) = create_test_coordinator().await;

    // 创建一些测试数据
    let test_config = json!({
        "name": "Backup Test",
        "version": "1.0.0"
    });

    coordinator
        .update_config("app", test_config.clone())
        .await
        .expect("更新配置失败");

    // 创建备份
    let backup_path = coordinator
        .create_backup(StorageLayer::Config)
        .await
        .expect("创建备份失败");

    println!("备份路径: {:?}", backup_path);
    assert!(backup_path.exists());

    // 修改配置
    let modified_config = json!({
        "name": "Modified Config",
        "version": "2.0.0"
    });

    coordinator
        .update_config("app", modified_config)
        .await
        .expect("修改配置失败");

    // 从备份恢复
    coordinator
        .restore_from_backup(StorageLayer::Config)
        .await
        .expect("从备份恢复失败");

    // 验证恢复的配置
    let restored_config = coordinator
        .get_config("app")
        .await
        .expect("获取恢复配置失败");
    assert_eq!(restored_config.get("name").unwrap(), "Backup Test");
}

#[tokio::test]
async fn test_concurrent_operations() {
    let (coordinator, _temp_dir) = create_test_coordinator().await;
    let coordinator = std::sync::Arc::new(coordinator);

    // 并发配置操作
    let mut handles = Vec::new();

    for i in 0..10 {
        let coordinator = coordinator.clone();
        let handle = tokio::spawn(async move {
            let config = json!({
                "test_id": i,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            coordinator
                .update_config(&format!("test_{}", i), config)
                .await
                .expect("并发更新配置失败");

            coordinator
                .get_config(&format!("test_{}", i))
                .await
                .expect("并发获取配置失败")
        });
        handles.push(handle);
    }

    // 等待所有操作完成
    for handle in handles {
        let result = handle.await.expect("并发任务失败");
        println!("并发操作结果: {:?}", result);
    }

    // 验证缓存统计
    let cache_stats = coordinator.get_cache_stats().await;
    println!("并发操作后缓存统计: {:?}", cache_stats);
}

#[tokio::test]
async fn test_event_system() {
    let (coordinator, _temp_dir) = create_test_coordinator().await;

    // 订阅事件
    let mut event_receiver = coordinator.subscribe_events();

    // 在后台监听事件
    let event_handle = tokio::spawn(async move {
        let mut events = Vec::new();

        // 监听事件，最多等待5秒
        let timeout = tokio::time::timeout(Duration::from_secs(5), async {
            while let Ok(event) = event_receiver.recv().await {
                events.push(event);
                if events.len() >= 3 {
                    break;
                }
            }
        });

        let _ = timeout.await;
        events
    });

    // 执行一些操作来触发事件
    sleep(Duration::from_millis(100)).await;

    coordinator
        .update_config("app", json!({"test": "event"}))
        .await
        .expect("更新配置失败");

    let session_state = SessionState::default();
    coordinator
        .save_session_state(&session_state)
        .await
        .expect("保存会话状态失败");

    let save_options = SaveOptions::new().table("test_table");
    let _ = coordinator
        .save_data(&json!({"test": "data"}), &save_options)
        .await;

    // 等待事件处理完成
    let events = event_handle.await.expect("事件监听任务失败");
    println!("捕获的事件: {:?}", events);

    // 验证至少捕获了一些事件
    assert!(!events.is_empty());
}
