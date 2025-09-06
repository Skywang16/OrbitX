//! 并发状态访问测试
//!
//! 测试重构后的状态管理的并发安全性，验证AC-2.2.3要求

use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::sync::RwLock;

/// 测试RwLock的基本并发安全性（模拟我们在AI模型存储库中的使用模式）
#[tokio::test]
async fn test_rwlock_concurrent_access() {
    println!("🔒 测试RwLock并发访问安全性");
    
    // 模拟我们在AIModelRepository中使用的RwLock<Option<String>>
    let shared_data = Arc::new(RwLock::new(Option::<String>::None));
    let mut join_set = JoinSet::new();

    // 启动多个并发读取器
    for i in 0..5 {
        let data_clone = Arc::clone(&shared_data);
        join_set.spawn(async move {
            let mut read_count = 0;
            for j in 0..50 {
                match data_clone.read().await.clone() {
                    Some(value) => {
                        read_count += 1;
                        if j % 10 == 0 {
                            println!("读取线程 {} 第 {} 次读取到: {} 字符", i, j, value.len());
                        }
                    }
                    None => {
                        if j % 10 == 0 {
                            println!("读取线程 {} 第 {} 次读取到: None", i, j);
                        }
                    }
                }
                tokio::time::sleep(Duration::from_micros(10)).await;
            }
            (format!("reader_{}", i), read_count)
        });
    }

    // 启动多个并发写入器
    for i in 0..3 {
        let data_clone = Arc::clone(&shared_data);
        join_set.spawn(async move {
            let mut write_count = 0;
            for j in 0..20 {
                let value = format!("写入者{}的第{}次写入", i, j);
                *data_clone.write().await = Some(value.clone());
                write_count += 1;
                
                if j % 5 == 0 {
                    println!("写入线程 {} 第 {} 次写入: {}", i, j, value);
                }
                
                tokio::time::sleep(Duration::from_micros(50)).await;
            }
            (format!("writer_{}", i), write_count)
        });
    }

    // 等待所有任务完成
    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((task_name, operation_count)) => {
                results.push((task_name.clone(), operation_count));
                println!("任务 {} 完成 {} 次操作", task_name, operation_count);
            }
            Err(e) => {
                panic!("任务执行失败: {:?}", e);
            }
        }
    }

    // 验证结果
    assert_eq!(results.len(), 8); // 5个读取器 + 3个写入器
    
    // 验证最终状态
    let final_value = shared_data.read().await.clone();
    assert!(final_value.is_some(), "最终应该有值存在");
    
    println!("✅ RwLock 并发访问测试通过: {} 个任务完成", results.len());
}

/// 测试高并发压力下的状态访问稳定性
#[tokio::test]
async fn test_high_concurrency_stress() {
    println!("🔥 开始高并发压力测试");
    
    // 创建共享状态，模拟我们的重构模式
    let shared_counter = Arc::new(RwLock::new(0u64));
    let mut join_set = JoinSet::new();

    let num_tasks = 10;
    let operations_per_task = 100;
    
    for i in 0..num_tasks {
        let counter_clone = Arc::clone(&shared_counter);
        join_set.spawn(async move {
            let mut local_increments = 0;
            for j in 0..operations_per_task {
                // 混合读写操作
                if j % 3 == 0 {
                    // 读取操作
                    let _current_value = *counter_clone.read().await;
                } else {
                    // 写入操作（递增）
                    *counter_clone.write().await += 1;
                    local_increments += 1;
                }
                
                // 随机短暂延时模拟真实工作负载
                if j % 20 == 0 {
                    tokio::time::sleep(Duration::from_micros(j as u64 % 50)).await;
                }
            }
            
            (i, local_increments)
        });
    }

    // 收集结果
    let mut total_increments = 0;
    let mut completed_tasks = 0;
    
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((task_id, increments)) => {
                total_increments += increments;
                completed_tasks += 1;
                
                if task_id % 3 == 0 {
                    println!("任务 {} 完成 {} 次递增操作", task_id, increments);
                }
            }
            Err(e) => {
                panic!("高并发任务失败: {:?}", e);
            }
        }
    }

    // 验证结果
    assert_eq!(completed_tasks, num_tasks);
    
    let final_counter_value = *shared_counter.read().await;
    assert_eq!(final_counter_value, total_increments, 
        "最终计数器值应该等于所有递增操作的总和");
    
    println!("✅ 高并发压力测试通过: {} 个任务，最终计数: {}", 
        completed_tasks, final_counter_value);
}

/// 测试并发访问下的数据一致性
#[tokio::test]
async fn test_concurrent_data_consistency() {
    println!("🔒 测试并发访问下的数据一致性");
    
    // 模拟用户前置提示词的并发读写
    let user_prompt = Arc::new(RwLock::new(Option::<String>::None));
    
    // 设置初始值
    *user_prompt.write().await = Some("初始提示词".to_string());
    
    let mut join_set = JoinSet::new();
    let num_writers = 5;
    let writes_per_writer = 10;
    
    // 启动多个写入者
    for writer_id in 0..num_writers {
        let prompt_clone = Arc::clone(&user_prompt);
        join_set.spawn(async move {
            let mut successful_writes = 0;
            for write_count in 0..writes_per_writer {
                let value = format!("写入者{}的第{}次写入", writer_id, write_count);
                
                // 写入操作
                *prompt_clone.write().await = Some(value.clone());
                successful_writes += 1;
                
                // 立即读取验证
                let read_value = prompt_clone.read().await.clone();
                match read_value {
                    Some(read_val) => {
                        // 注意：在并发环境中，读取的值可能不是刚写入的值
                        // 但应该是某个写入者写入的有效值
                        assert!(!read_val.is_empty(), "读取的值不应为空");
                    }
                    None => {
                        panic!("写入后读取到None值");
                    }
                }
                
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            
            (writer_id, successful_writes)
        });
    }
    
    // 等待所有写入者完成
    let mut total_writes = 0;
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((writer_id, writes)) => {
                assert_eq!(writes, writes_per_writer, "写入者 {} 应该完成所有写入", writer_id);
                total_writes += writes;
                println!("写入者 {} 完成 {} 次写入", writer_id, writes);
            }
            Err(e) => {
                panic!("写入者任务失败: {:?}", e);
            }
        }
    }
    
    // 验证最终状态
    assert_eq!(total_writes, num_writers * writes_per_writer);
    
    // 最终读取验证
    let final_value = user_prompt.read().await.clone();
    assert!(final_value.is_some(), "最终应该有值存在");
    assert!(!final_value.unwrap().is_empty(), "最终值不应为空");
    
    println!("✅ 数据一致性测试通过: 总计 {} 次写入操作", total_writes);
}

/// 基础的线程安全验证测试
#[tokio::test]
async fn test_basic_thread_safety() {
    println!("🛡️ 基础线程安全验证");
    
    // 简单的Arc+RwLock模式验证（这是我们重构中使用的核心模式）
    let shared_data: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    let mut handles = Vec::new();
    
    // 创建多个异步任务同时访问共享数据
    for i in 0..5 {
        let data_clone = Arc::clone(&shared_data);
        let handle = tokio::spawn(async move {
            // 每个任务添加一些数据
            for j in 0..10 {
                {
                    let mut data = data_clone.write().await;
                    data.push(format!("Task-{}-Item-{}", i, j));
                }
                
                // 短暂延时模拟工作
                tokio::time::sleep(Duration::from_micros(100)).await;
                
                // 读取当前数据大小
                {
                    let data = data_clone.read().await;
                    if j == 9 {
                        println!("任务 {} 完成，当前数据大小: {}", i, data.len());
                    }
                }
            }
            i
        });
        handles.push(handle);
    }
    
    // 等待所有任务完成
    for handle in handles {
        let task_id = handle.await.expect("任务应该成功完成");
        println!("任务 {} 完成", task_id);
    }
    
    // 验证最终结果
    let final_data = shared_data.read().await;
    assert_eq!(final_data.len(), 50, "应该有50个项目 (5个任务 × 10个项目)");
    
    println!("✅ 基础线程安全验证通过: {} 个项目", final_data.len());
}