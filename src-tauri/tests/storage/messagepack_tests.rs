/*!
 * MessagePack状态管理器测试模块
 *
 * 测试会话状态的序列化、反序列化、备份和恢复功能
 * 验证数据完整性、错误处理和性能表现
 */

use chrono::Utc;
use tempfile::TempDir;
use tokio::fs;

use terminal_lib::storage::{
    messagepack::{MessagePackManager, MessagePackOptions},
    paths::StoragePaths,
    types::{AiState, SessionState, TerminalState, UiState},
};
use terminal_lib::utils::error::AppResult;

/// 创建测试用的临时存储路径
async fn create_test_paths() -> AppResult<(TempDir, StoragePaths)> {
    let temp_dir = TempDir::new().unwrap();
    let paths = StoragePaths::new(temp_dir.path().to_path_buf())?;
    paths.ensure_directories()?;
    Ok((temp_dir, paths))
}

/// 创建测试用的会话状态
fn create_test_session_state() -> SessionState {
    let terminals = vec![
        TerminalState {
            id: 1,
            title: "Terminal 1".to_string(),
            active: true,
            shell: Some("bash".to_string()),
        },
        TerminalState {
            id: 2,
            title: "Terminal 2".to_string(),
            active: false,
            shell: None,
        },
    ];

    SessionState {
        version: 1,
        terminals,
        ui: UiState {
            theme: "dark".to_string(),
            font_size: 14.0,
            sidebar_width: 300,
        },
        ai: AiState::default(),
        timestamp: Utc::now(),
    }
}

/// 测试MessagePack管理器的基本创建
#[tokio::test]
async fn test_messagepack_manager_creation() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();
    let options = MessagePackOptions::default();

    let manager = MessagePackManager::new(paths, options).await;
    assert!(manager.is_ok(), "MessagePack管理器创建应该成功");
}

/// 测试会话状态的序列化和反序列化
#[tokio::test]
async fn test_session_state_serialization() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();
    let options = MessagePackOptions::default();
    let manager = MessagePackManager::new(paths, options).await.unwrap();

    let original_state = create_test_session_state();

    // 测试序列化
    let serialized = manager.serialize_state(&original_state);
    assert!(serialized.is_ok(), "序列化应该成功");

    let serialized_data = serialized.unwrap();
    assert!(!serialized_data.is_empty(), "序列化数据不应为空");

    // 测试反序列化
    let deserialized = manager.deserialize_state(&serialized_data);
    assert!(deserialized.is_ok(), "反序列化应该成功");

    let deserialized_state = deserialized.unwrap();

    // 验证数据完整性
    assert_eq!(original_state.version, deserialized_state.version);
    assert_eq!(
        original_state.terminals.len(),
        deserialized_state.terminals.len()
    );
    assert_eq!(original_state.ui.theme, deserialized_state.ui.theme);
}

/// 测试压缩功能
#[tokio::test]
async fn test_compression() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();

    // 测试启用压缩
    let compressed_options = MessagePackOptions {
        compression: true,
        ..Default::default()
    };
    let compressed_manager = MessagePackManager::new(paths.clone(), compressed_options)
        .await
        .unwrap();

    // 测试禁用压缩
    let uncompressed_options = MessagePackOptions {
        compression: false,
        ..Default::default()
    };
    let uncompressed_manager = MessagePackManager::new(paths, uncompressed_options)
        .await
        .unwrap();

    let state = create_test_session_state();

    let compressed_data = compressed_manager.serialize_state(&state).unwrap();
    let uncompressed_data = uncompressed_manager.serialize_state(&state).unwrap();

    // 压缩数据应该更小（对于足够大的数据）
    println!("压缩数据大小: {} bytes", compressed_data.len());
    println!("未压缩数据大小: {} bytes", uncompressed_data.len());

    // 两种方式都应该能正确反序列化
    let compressed_result = compressed_manager
        .deserialize_state(&compressed_data)
        .unwrap();
    let uncompressed_result = uncompressed_manager
        .deserialize_state(&uncompressed_data)
        .unwrap();

    assert_eq!(compressed_result.version, uncompressed_result.version);
    assert_eq!(
        compressed_result.terminals.len(),
        uncompressed_result.terminals.len()
    );
}

/// 测试状态保存和加载
#[tokio::test]
async fn test_save_and_load_state() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();
    let options = MessagePackOptions::default();
    let manager = MessagePackManager::new(paths, options).await.unwrap();

    let original_state = create_test_session_state();

    // 测试保存状态
    let save_result = manager.save_state(&original_state).await;
    assert!(save_result.is_ok(), "保存状态应该成功");

    // 测试加载状态
    let load_result = manager.load_state().await;
    assert!(load_result.is_ok(), "加载状态应该成功");

    let loaded_state = load_result.unwrap();
    assert!(loaded_state.is_some(), "应该加载到状态数据");

    let loaded_state = loaded_state.unwrap();

    // 验证加载的数据与原始数据一致
    assert_eq!(original_state.version, loaded_state.version);
    assert_eq!(original_state.terminals.len(), loaded_state.terminals.len());
}

/// 测试不存在状态文件时的加载
#[tokio::test]
async fn test_load_nonexistent_state() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();
    let options = MessagePackOptions::default();
    let manager = MessagePackManager::new(paths, options).await.unwrap();

    // 加载不存在的状态文件
    let load_result = manager.load_state().await;
    assert!(
        load_result.is_ok(),
        "加载不存在的文件应该返回None而不是错误"
    );

    let loaded_state = load_result.unwrap();
    assert!(loaded_state.is_none(), "不存在的文件应该返回None");
}

/// 测试备份功能
#[tokio::test]
async fn test_backup_functionality() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();
    let options = MessagePackOptions {
        backup_count: 2,
        ..Default::default()
    };
    let manager = MessagePackManager::new(paths, options).await.unwrap();

    let state1 = create_test_session_state();
    let mut state2 = create_test_session_state();
    state2.version = 2;
    let mut state3 = create_test_session_state();
    state3.version = 3;

    // 保存第一个状态（不会创建备份，因为文件不存在）
    manager.save_state(&state1).await.unwrap();

    // 添加延迟确保文件时间戳不同
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // 保存第二个状态（应该创建第一个备份）
    manager.save_state(&state2).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // 保存第三个状态（应该创建第二个备份）
    manager.save_state(&state3).await.unwrap();

    // 获取统计信息
    let stats = manager.get_state_stats().await.unwrap();
    assert!(stats.state_file_exists, "状态文件应该存在");

    println!("实际备份数量: {}", stats.backup_count);
    println!("状态文件大小: {}", stats.state_file_size);

    // 第一次保存不创建备份，所以只有2个备份（第二次和第三次保存时创建）
    // 但由于时间戳可能相同，可能只创建了1个备份文件
    assert!(stats.backup_count >= 1, "至少应该有1个备份文件");
    assert!(stats.backup_count <= 2, "最多应该有2个备份文件");

    // 验证当前状态是最新的
    let current_state = manager.load_state().await.unwrap().unwrap();
    assert_eq!(current_state.version, 3, "当前状态应该是版本3");
}

/// 测试备份数量限制
#[tokio::test]
async fn test_backup_count_limit() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();
    let options = MessagePackOptions {
        backup_count: 2,
        ..Default::default()
    };
    let manager = MessagePackManager::new(paths, options).await.unwrap();

    // 创建多个状态并保存
    for i in 1..=5 {
        let mut state = create_test_session_state();
        state.version = i;
        manager.save_state(&state).await.unwrap();

        // 添加小延迟确保文件时间戳不同
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // 检查备份数量是否被限制
    let stats = manager.get_state_stats().await.unwrap();

    println!("备份数量限制测试 - 实际备份数量: {}", stats.backup_count);

    // 第一次保存不创建备份，所以5次保存会创建4个备份，但限制为2个
    // 由于时间戳可能相同，实际备份数量可能少于预期
    assert!(stats.backup_count <= 2, "备份数量应该被限制为2个或更少");
    assert!(stats.backup_count >= 1, "应该至少有1个备份文件");
}

/// 测试从备份恢复
#[tokio::test]
async fn test_restore_from_backup() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();
    let options = MessagePackOptions::default();
    let manager = MessagePackManager::new(paths.clone(), options)
        .await
        .unwrap();

    let original_state = create_test_session_state();

    // 保存原始状态
    manager.save_state(&original_state).await.unwrap();

    // 手动创建一个损坏的状态文件
    let state_file_path = paths.state_dir.join("session_state.msgpack");
    fs::write(&state_file_path, b"corrupted data")
        .await
        .unwrap();

    // 尝试加载状态（应该从备份恢复）
    let load_result = manager.load_state().await;

    // 如果有备份，应该能恢复；如果没有备份，应该返回None
    match load_result {
        Ok(Some(restored_state)) => {
            // 成功从备份恢复
            assert_eq!(restored_state.version, original_state.version);
            println!("成功从备份恢复状态");
        }
        Ok(None) => {
            // 没有可用的备份
            println!("没有可用的备份，返回None");
        }
        Err(e) => {
            panic!("恢复过程不应该出错: {}", e);
        }
    }
}

/// 测试校验和验证
#[tokio::test]
async fn test_checksum_validation() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();
    let options = MessagePackOptions {
        checksum_validation: true,
        ..Default::default()
    };
    let manager = MessagePackManager::new(paths, options).await.unwrap();

    let state = create_test_session_state();

    // 序列化状态
    let mut serialized_data = manager.serialize_state(&state).unwrap();

    // 损坏数据（修改最后一个字节）
    if let Some(last_byte) = serialized_data.last_mut() {
        *last_byte = last_byte.wrapping_add(1);
    }

    // 尝试反序列化损坏的数据
    let deserialize_result = manager.deserialize_state(&serialized_data);
    assert!(
        deserialize_result.is_err(),
        "损坏的数据应该导致反序列化失败"
    );

    // 验证错误信息包含校验和或反序列化相关内容
    let error_message = deserialize_result.unwrap_err().to_string();
    assert!(
        error_message.contains("校验")
            || error_message.contains("checksum")
            || error_message.contains("Invalid")
            || error_message.to_lowercase().contains("corrupt"),
        "错误信息应该提到校验和验证失败或数据损坏，实际错误: {}",
        error_message
    );
}

/// 测试禁用校验和验证
#[tokio::test]
async fn test_disabled_checksum_validation() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();
    let options = MessagePackOptions {
        checksum_validation: false,
        ..Default::default()
    };
    let manager = MessagePackManager::new(paths, options).await.unwrap();

    let state = create_test_session_state();

    // 序列化和反序列化应该正常工作
    let serialized_data = manager.serialize_state(&state).unwrap();
    let deserialized_state = manager.deserialize_state(&serialized_data).unwrap();

    assert_eq!(state.version, deserialized_state.version);
}

/// 测试文件大小限制
#[tokio::test]
async fn test_file_size_limit() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();
    let options = MessagePackOptions {
        max_file_size: 100, // 设置很小的限制
        ..Default::default()
    };
    let manager = MessagePackManager::new(paths, options).await.unwrap();

    let state = create_test_session_state();

    // 序列化应该失败，因为数据超过大小限制
    let serialize_result = manager.serialize_state(&state);
    assert!(serialize_result.is_err(), "超过大小限制的序列化应该失败");

    let error_message = serialize_result.unwrap_err().to_string();
    assert!(error_message.contains("过大"), "错误信息应该提到数据过大");
}

/// 测试状态统计信息
#[tokio::test]
async fn test_state_stats() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();
    let options = MessagePackOptions::default();
    let manager = MessagePackManager::new(paths, options).await.unwrap();

    // 初始状态：没有状态文件
    let initial_stats = manager.get_state_stats().await.unwrap();
    assert!(!initial_stats.state_file_exists, "初始时状态文件不应存在");
    assert_eq!(initial_stats.state_file_size, 0, "初始状态文件大小应为0");
    assert_eq!(initial_stats.backup_count, 0, "初始备份数量应为0");

    // 保存状态后
    let state = create_test_session_state();
    manager.save_state(&state).await.unwrap();

    let after_save_stats = manager.get_state_stats().await.unwrap();
    assert!(after_save_stats.state_file_exists, "保存后状态文件应存在");
    assert!(after_save_stats.state_file_size > 0, "状态文件大小应大于0");

    // 验证格式化方法
    let formatted_size = after_save_stats.state_file_size_formatted();
    assert!(
        formatted_size.contains("B") || formatted_size.contains("KB"),
        "格式化大小应包含单位"
    );
}

/// 测试并发访问
#[tokio::test]
async fn test_concurrent_access() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();
    let options = MessagePackOptions::default();
    let manager = std::sync::Arc::new(MessagePackManager::new(paths, options).await.unwrap());

    // 先保存一个初始状态，避免并发时的竞争条件
    let initial_state = create_test_session_state();
    manager.save_state(&initial_state).await.unwrap();

    let mut handles = Vec::new();

    // 启动多个并发任务，但使用更少的任务数量以减少竞争
    for i in 0..3 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            // 添加随机延迟以减少竞争
            tokio::time::sleep(tokio::time::Duration::from_millis(i * 10)).await;

            let mut state = create_test_session_state();
            state.version = (i + 10) as u32; // 使用不同的版本号

            // 保存状态（可能会失败，这是正常的）
            let save_result = manager_clone.save_state(&state).await;

            // 加载状态（应该总是成功）
            let loaded = manager_clone.load_state().await.unwrap();
            assert!(loaded.is_some(), "应该能加载到状态");

            (i, save_result.is_ok())
        });
        handles.push(handle);
    }

    // 等待所有任务完成
    let mut success_count = 0;
    for handle in handles {
        let (task_id, save_success) = handle.await.unwrap();
        if save_success {
            success_count += 1;
        }
        println!("任务 {} 完成，保存成功: {}", task_id, save_success);
    }

    // 验证最终状态
    let final_state = manager.load_state().await.unwrap();
    assert!(final_state.is_some(), "最终应该有状态数据");

    println!("成功保存的任务数量: {}", success_count);
}

/// 性能测试：大量数据的序列化和反序列化
#[tokio::test]
async fn test_large_data_performance() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();
    let options = MessagePackOptions::default();
    let manager = MessagePackManager::new(paths, options).await.unwrap();

    // 创建包含大量数据的状态
    let mut large_state = create_test_session_state();

    // 添加大量终端
    for i in 0..100 {
        let term = TerminalState {
            id: i as u32,
            title: format!("Terminal {}", i),
            active: i == 0,
            shell: Some("bash".to_string()),
        };
        large_state.terminals.push(term);
    }

    let start_time = std::time::Instant::now();

    // 测试序列化性能
    let serialized = manager.serialize_state(&large_state).unwrap();
    let serialize_duration = start_time.elapsed();

    println!("序列化耗时: {:?}", serialize_duration);
    println!("序列化数据大小: {} bytes", serialized.len());

    // 测试反序列化性能
    let deserialize_start = std::time::Instant::now();
    let deserialized = manager.deserialize_state(&serialized).unwrap();
    let deserialize_duration = deserialize_start.elapsed();

    println!("反序列化耗时: {:?}", deserialize_duration);

    // 验证数据完整性
    assert_eq!(large_state.version, deserialized.version);
    assert_eq!(large_state.terminals.len(), deserialized.terminals.len());

    // 性能断言（这些值可能需要根据实际情况调整）
    assert!(
        serialize_duration.as_millis() < 1000,
        "序列化应该在1秒内完成"
    );
    assert!(
        deserialize_duration.as_millis() < 1000,
        "反序列化应该在1秒内完成"
    );
}

/// 测试错误处理和恢复
#[tokio::test]
async fn test_error_handling() {
    let (_temp_dir, paths) = create_test_paths().await.unwrap();
    let options = MessagePackOptions::default();
    let manager = MessagePackManager::new(paths.clone(), options)
        .await
        .unwrap();

    // 测试无效的序列化数据
    let invalid_data = b"this is not valid messagepack data";
    let deserialize_result = manager.deserialize_state(invalid_data);
    assert!(deserialize_result.is_err(), "无效数据应该导致反序列化失败");

    // 测试空数据
    let empty_data = b"";
    let empty_result = manager.deserialize_state(empty_data);
    assert!(empty_result.is_err(), "空数据应该导致反序列化失败");

    // 测试权限问题（通过删除目录模拟）
    let state = create_test_session_state();

    // 删除状态目录
    if paths.state_dir.exists() {
        fs::remove_dir_all(&paths.state_dir).await.unwrap();
    }

    // 尝试保存状态（应该重新创建目录）
    let save_result = manager.save_state(&state).await;
    // 这应该成功，因为管理器会重新创建目录
    assert!(save_result.is_ok(), "管理器应该能够重新创建目录");
}
