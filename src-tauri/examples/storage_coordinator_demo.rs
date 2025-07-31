/*!
 * 存储协调器演示程序
 *
 * 展示如何使用存储协调器进行配置管理、会话状态保存和数据查询
 */

use serde_json::json;
use std::time::Duration;
use tempfile::TempDir;

use terminal_lib::storage::types::{DataQuery, SaveOptions, SessionState};
use terminal_lib::storage::{
    CacheConfig, MessagePackOptions, SqliteOptions, StorageCoordinator, StorageCoordinatorOptions,
    StoragePaths, TomlConfigOptions,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    println!("🚀 存储协调器演示程序");
    println!("========================");

    // 创建临时目录用于演示
    let temp_dir = TempDir::new()?;
    let paths = StoragePaths::new(temp_dir.path().to_path_buf())?;

    println!("📁 存储路径: {:?}", temp_dir.path());

    // 配置存储协调器选项
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

    // 创建存储协调器
    println!("🔧 初始化存储协调器...");
    let coordinator = StorageCoordinator::new(paths, options).await?;
    println!("✅ 存储协调器初始化完成");

    // 演示1: 系统健康检查
    println!("\n🏥 系统健康检查");
    println!("================");
    let health = coordinator.health_check().await?;
    println!(
        "整体健康状态: {}",
        if health.overall_healthy {
            "✅ 健康"
        } else {
            "❌ 不健康"
        }
    );
    println!("检查项目数量: {}", health.checks.len());
    for check in &health.checks {
        let status = if check.healthy { "✅" } else { "❌" };
        println!("  {} {}: {}", status, check.name, check.message);
    }

    // 演示2: 配置管理
    println!("\n⚙️  配置管理演示");
    println!("================");

    // 获取默认配置
    let app_config = coordinator.get_config("app").await?;
    println!(
        "默认应用配置: {}",
        serde_json::to_string_pretty(&app_config)?
    );

    // 更新配置
    let new_config = json!({
        "name": "TermX Demo",
        "version": "1.0.0",
        "debug": true,
        "features": {
            "ai_integration": true,
            "auto_completion": true,
            "theme_support": true
        }
    });

    coordinator.update_config("app", new_config.clone()).await?;
    println!("✅ 配置已更新");

    // 验证配置更新
    let updated_config = coordinator.get_config("app").await?;
    println!(
        "更新后配置: {}",
        serde_json::to_string_pretty(&updated_config)?
    );

    // 演示3: 会话状态管理
    println!("\n💾 会话状态管理演示");
    println!("==================");

    // 创建示例会话状态
    let mut session_state = SessionState::default();
    session_state.window_state.size = (1920, 1080);
    session_state.window_state.position = (100, 100);
    session_state.ui_state.current_theme = "dark".to_string();
    session_state.ui_state.font_size = 14.0;

    // 添加一些标签页
    for i in 0..3 {
        session_state
            .tabs
            .push(terminal_lib::storage::types::TabState {
                id: format!("tab_{}", i),
                title: format!("Terminal {}", i + 1),
                is_active: i == 0,
                working_directory: format!("/home/user/project{}", i + 1),
                terminal_session_id: Some(format!("session_{}", i)),
                custom_data: std::collections::HashMap::new(),
            });
    }

    // 保存会话状态
    coordinator.save_session_state(&session_state).await?;
    println!("✅ 会话状态已保存");

    // 加载会话状态
    let loaded_state = coordinator.load_session_state().await?;
    if let Some(state) = loaded_state {
        println!("✅ 会话状态已加载");
        println!("窗口大小: {:?}", state.window_state.size);
        println!("当前主题: {}", state.ui_state.current_theme);
        println!("标签页数量: {}", state.tabs.len());
    }

    // 演示4: 数据操作
    println!("\n🗄️  数据操作演示");
    println!("===============");

    // 保存一些示例命令历史
    let commands = [
        ("ls -la", 0, 150),
        ("cd /home/user", 0, 50),
        ("git status", 0, 200),
        ("npm install", 0, 5000),
        ("cargo build", 0, 3000),
    ];

    for (i, (command, exit_code, duration)) in commands.iter().enumerate() {
        let data = json!({
            "id": format!("cmd_{}", i),
            "command": command,
            "exit_code": exit_code,
            "duration_ms": duration,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "working_directory": "/home/user"
        });

        let save_options = SaveOptions::new()
            .table("command_history")
            .validate(true)
            .backup(false);

        coordinator.save_data(&data, &save_options).await?;
    }
    println!("✅ 已保存 {} 条命令历史", commands.len());

    // 查询数据
    let query =
        DataQuery::new("SELECT * FROM command_history ORDER BY timestamp DESC").with_limit(3);

    let results = coordinator.query_data(&query).await?;
    println!("📊 最近的 {} 条命令:", results.len());
    for (i, result) in results.iter().enumerate() {
        if let Some(command) = result.get("command") {
            println!("  {}. {}", i + 1, command.as_str().unwrap_or("N/A"));
        }
    }

    // 演示5: 缓存性能
    println!("\n⚡ 缓存性能演示");
    println!("===============");

    // 预加载缓存
    coordinator.preload_cache().await?;
    println!("✅ 缓存预加载完成");

    // 测试缓存性能
    let start_time = std::time::Instant::now();
    for _ in 0..100 {
        let _ = coordinator.get_config("app").await?;
    }
    let cached_time = start_time.elapsed();

    // 清理缓存后测试
    coordinator.clear_cache().await?;
    let start_time = std::time::Instant::now();
    for _ in 0..10 {
        let _ = coordinator.get_config("app").await?;
    }
    let no_cache_time = start_time.elapsed();

    println!("缓存访问时间 (100次): {:?}", cached_time);
    println!("无缓存访问时间 (10次): {:?}", no_cache_time);

    // 获取缓存统计
    let cache_stats = coordinator.get_cache_stats().await;
    println!("缓存统计:");
    println!("  总命中率: {:.2}%", cache_stats.total_hit_rate * 100.0);
    println!("  内存使用: {} bytes", cache_stats.total_memory_usage);
    println!("  条目数量: {}", cache_stats.total_entries);

    // 演示6: 备份和恢复
    println!("\n💿 备份和恢复演示");
    println!("================");

    // 创建配置备份
    let backup_path = coordinator
        .create_backup(terminal_lib::storage::StorageLayer::Config)
        .await?;
    println!("✅ 配置备份已创建: {:?}", backup_path);

    // 修改配置
    let modified_config = json!({
        "name": "Modified Config",
        "version": "2.0.0"
    });
    coordinator.update_config("app", modified_config).await?;
    println!("✅ 配置已修改");

    // 从备份恢复
    coordinator
        .restore_from_backup(terminal_lib::storage::StorageLayer::Config)
        .await?;
    println!("✅ 已从备份恢复配置");

    // 验证恢复
    let restored_config = coordinator.get_config("app").await?;
    if let Some(name) = restored_config.get("name") {
        println!("恢复后的应用名称: {}", name.as_str().unwrap_or("N/A"));
    }

    // 演示7: 存储统计
    println!("\n📈 存储统计");
    println!("===========");
    let storage_stats = coordinator.get_storage_stats().await?;
    println!(
        "总存储大小: {}",
        terminal_lib::storage::types::StorageStats::format_size(storage_stats.total_size)
    );
    println!(
        "配置大小: {}",
        terminal_lib::storage::types::StorageStats::format_size(storage_stats.config_size)
    );
    println!(
        "状态大小: {}",
        terminal_lib::storage::types::StorageStats::format_size(storage_stats.state_size)
    );
    println!(
        "数据大小: {}",
        terminal_lib::storage::types::StorageStats::format_size(storage_stats.data_size)
    );
    println!(
        "缓存大小: {}",
        terminal_lib::storage::types::StorageStats::format_size(storage_stats.cache_size)
    );

    println!("\n🎉 演示完成！");
    println!("存储协调器成功演示了所有核心功能:");
    println!("  ✅ 系统健康检查");
    println!("  ✅ 配置管理");
    println!("  ✅ 会话状态管理");
    println!("  ✅ 数据操作");
    println!("  ✅ 缓存性能优化");
    println!("  ✅ 备份和恢复");
    println!("  ✅ 存储统计");

    Ok(())
}
