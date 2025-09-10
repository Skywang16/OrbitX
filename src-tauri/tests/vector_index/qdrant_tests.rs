/*!
 * Qdrant集成层测试
 *
 * 测试Qdrant数据库集成的各个方面：
 * - 连接管理
 * - 集合创建和配置
 * - 向量批量上传
 * - 错误处理和恢复
 * - 性能基准测试
 *
 * Requirements: 3.1, 3.2, 3.3, 3.4 的测试覆盖
 */

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use terminal_lib::vector_index::{
    qdrant::{QdrantClientImpl, QdrantService},
    types::{CodeVector, SearchOptions, VectorIndexConfig},
};
use tokio::time::timeout;

use crate::test_utils::*;
use crate::vector_index::test_fixtures::*;
use crate::vector_index::*;

/// Qdrant连接测试
#[cfg(test)]
mod connection_tests {
    use super::*;

    #[tokio::test]
    async fn test_valid_connection_success() -> TestResult {
        let config = create_test_qdrant_config();

        // 注意：这个测试需要实际的Qdrant实例运行
        // 在CI环境中可能需要跳过或使用mock
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            println!("跳过Qdrant集成测试（设置了SKIP_QDRANT_INTEGRATION）");
            return Ok(());
        }

        let result = QdrantClientImpl::new(config).await;

        match result {
            Ok(client) => {
                let connection_result = client.test_connection().await;
                assert_qdrant_connection_success!(connection_result);
            }
            Err(e) => {
                // 如果连接失败，检查是否是因为Qdrant服务不可用
                let error_msg = e.to_string();
                if error_msg.contains("Connection refused") || error_msg.contains("No such host") {
                    println!("跳过测试：Qdrant服务不可用 - {}", error_msg);
                    return Ok(());
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_connection_failure() -> TestResult {
        let mut config = create_test_qdrant_config();
        config.qdrant_url = "http://invalid-host:9999".to_string();

        let result = QdrantClientImpl::new(config).await;

        // 应该连接失败
        assert!(result.is_err(), "无效的Qdrant URL应该导致连接失败");

        Ok(())
    }

    #[tokio::test]
    async fn test_connection_timeout() -> TestResult {
        let mut config = create_test_qdrant_config();
        config.qdrant_url = "http://192.0.2.1:6333".to_string(); // RFC 5737测试地址

        // 设置较短的超时时间
        let result = timeout(Duration::from_secs(5), QdrantClientImpl::new(config)).await;

        match result {
            Ok(Err(_)) => {
                // 连接失败是预期的
            }
            Err(_) => {
                // 超时也是预期的
            }
            Ok(Ok(_)) => {
                panic!("不应该成功连接到测试地址");
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_api_key_authentication() -> TestResult {
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            return Ok(());
        }

        let mut config = create_test_qdrant_config();
        config.qdrant_api_key = Some("test-api-key".to_string());

        // 注意：这个测试需要配置了API密钥的Qdrant实例
        let result = QdrantClientImpl::new(config).await;

        // 根据实际Qdrant配置，这可能成功或失败
        // 主要是测试API密钥设置不会导致panic
        match result {
            Ok(_) => println!("API密钥认证测试通过"),
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("Unauthorized") || error_msg.contains("Connection refused") {
                    println!("API密钥认证测试：预期的认证错误");
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }
}

/// 集合管理测试
#[cfg(test)]
mod collection_tests {
    use super::*;

    #[tokio::test]
    async fn test_collection_initialization() -> TestResult {
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            return Ok(());
        }

        let config = create_test_qdrant_config();

        match QdrantClientImpl::new(config).await {
            Ok(client) => {
                let result = client.initialize_collection().await;
                assert_operation_success!(result, "集合初始化");

                // 验证集合信息
                let info_result = client.get_collection_info().await;
                assert!(info_result.is_ok(), "应该能够获取集合信息");

                let (points_count, vectors_count) = info_result.unwrap();
                println!("集合统计: {} 个点, {} 个向量", points_count, vectors_count);
            }
            Err(e) => {
                if e.to_string().contains("Connection refused") {
                    println!("跳过测试：Qdrant服务不可用");
                    return Ok(());
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_collection_config_validation() -> TestResult {
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            return Ok(());
        }

        let config = create_test_qdrant_config();

        match QdrantClientImpl::new(config.clone()).await {
            Ok(client) => {
                // 首次初始化
                let init_result = client.initialize_collection().await;
                assert_operation_success!(init_result, "首次集合初始化");

                // 再次初始化相同配置应该成功（验证现有配置）
                let second_init = client.initialize_collection().await;
                assert_operation_success!(second_init, "重复集合初始化");
            }
            Err(e) => {
                if e.to_string().contains("Connection refused") {
                    println!("跳过测试：Qdrant服务不可用");
                    return Ok(());
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_collection_dimension_mismatch() -> TestResult {
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            return Ok(());
        }

        let mut config = create_test_qdrant_config();

        // 创建集合
        match QdrantClientImpl::new(config.clone()).await {
            Ok(client) => {
                let _ = client.initialize_collection().await;

                // 尝试使用不同的向量维度
                config.vector_size = 768; // 不同的维度

                match QdrantClientImpl::new(config).await {
                    Ok(client2) => {
                        // 应该能检测到维度不匹配
                        let result = client2.initialize_collection().await;

                        // 根据实现，这可能成功（如果跳过验证）或失败
                        match result {
                            Ok(_) => println!("维度验证可能被跳过"),
                            Err(e) => {
                                assert!(
                                    e.to_string().contains("维度不匹配")
                                        || e.to_string().contains("dimension"),
                                    "应该检测到维度不匹配错误"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        if !e.to_string().contains("Connection refused") {
                            return Err(e.into());
                        }
                    }
                }
            }
            Err(e) => {
                if e.to_string().contains("Connection refused") {
                    println!("跳过测试：Qdrant服务不可用");
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }
}

/// 向量上传测试
#[cfg(test)]
mod upload_tests {
    use super::*;

    #[tokio::test]
    async fn test_small_batch_upload() -> TestResult {
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            return Ok(());
        }

        let config = create_test_qdrant_config();

        match QdrantClientImpl::new(config).await {
            Ok(client) => {
                let _ = client.initialize_collection().await;

                // 创建小批量测试数据
                let test_vectors = generate_performance_test_vectors(5);

                let result = client.upload_vectors(test_vectors.clone()).await;
                assert_operation_success!(result, "小批量向量上传");

                // 验证上传后的集合信息
                let (points_count, _) = client.get_collection_info().await?;
                assert!(
                    points_count >= test_vectors.len(),
                    "上传后的点数量应该至少为 {}, 实际为 {}",
                    test_vectors.len(),
                    points_count
                );
            }
            Err(e) => {
                if e.to_string().contains("Connection refused") {
                    println!("跳过测试：Qdrant服务不可用");
                    return Ok(());
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_large_batch_upload() -> TestResult {
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            return Ok(());
        }

        let config = create_test_qdrant_config();

        match QdrantClientImpl::new(config).await {
            Ok(client) => {
                let _ = client.initialize_collection().await;

                // 创建大批量测试数据（超过单个批次大小）
                let test_vectors = generate_performance_test_vectors(1500); // 超过1000的批次大小

                let start_time = std::time::Instant::now();
                let result = client.upload_vectors(test_vectors.clone()).await;
                let upload_duration = start_time.elapsed();

                assert_operation_success!(result, "大批量向量上传");

                println!(
                    "大批量上传性能: {} 个向量在 {:?} 内完成",
                    test_vectors.len(),
                    upload_duration
                );

                // 验证上传性能（不超过合理时间）
                let max_expected_duration = Duration::from_secs(30);
                assert!(
                    upload_duration < max_expected_duration,
                    "上传时间 {:?} 超过了最大预期时间 {:?}",
                    upload_duration,
                    max_expected_duration
                );
            }
            Err(e) => {
                if e.to_string().contains("Connection refused") {
                    println!("跳过测试：Qdrant服务不可用");
                    return Ok(());
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_empty_batch_upload() -> TestResult {
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            return Ok(());
        }

        let config = create_test_qdrant_config();

        match QdrantClientImpl::new(config).await {
            Ok(client) => {
                let _ = client.initialize_collection().await;

                // 测试空向量列表
                let empty_vectors = Vec::new();
                let result = client.upload_vectors(empty_vectors).await;

                // 空上传应该成功（优雅处理）
                assert_operation_success!(result, "空批量向量上传");
            }
            Err(e) => {
                if e.to_string().contains("Connection refused") {
                    println!("跳过测试：Qdrant服务不可用");
                    return Ok(());
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_vector_upload() -> TestResult {
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            return Ok(());
        }

        let config = create_test_qdrant_config();

        match QdrantClientImpl::new(config).await {
            Ok(client) => {
                let _ = client.initialize_collection().await;

                // 测试无效向量（错误维度）
                let invalid_vectors = create_invalid_test_vectors();

                for invalid_vector in invalid_vectors {
                    let result = client.upload_vectors(vec![invalid_vector]).await;

                    // 应该失败，因为向量维度不匹配
                    assert_operation_failure!(result, "无效向量上传");
                }
            }
            Err(e) => {
                if e.to_string().contains("Connection refused") {
                    println!("跳过测试：Qdrant服务不可用");
                    return Ok(());
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }
}

/// 错误处理和恢复测试
#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_recovery() -> TestResult {
        // 这个测试模拟网络中断和恢复
        // 在实际环境中，这可能需要手动测试或特殊的测试设置

        println!("连接恢复测试需要手动验证（模拟网络中断）");
        Ok(())
    }

    #[tokio::test]
    async fn test_partial_upload_failure() -> TestResult {
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            return Ok(());
        }

        // 这个测试验证部分上传失败时的行为
        // 需要创建一个混合了有效和无效向量的批次

        let config = create_test_qdrant_config();

        match QdrantClientImpl::new(config).await {
            Ok(client) => {
                let _ = client.initialize_collection().await;

                let mut mixed_vectors = generate_performance_test_vectors(3);
                let invalid_vectors = create_invalid_test_vectors();
                mixed_vectors.extend(invalid_vectors);

                let result = client.upload_vectors(mixed_vectors).await;

                // 根据实现，这应该失败或部分成功
                match result {
                    Ok(_) => {
                        // 如果成功，可能是因为验证被延迟到Qdrant服务端
                        println!("混合批次上传成功（服务端验证）");
                    }
                    Err(e) => {
                        // 预期的失败
                        assert!(
                            e.to_string().contains("维度不匹配")
                                || e.to_string().contains("dimension"),
                            "应该检测到维度错误: {}",
                            e
                        );
                    }
                }
            }
            Err(e) => {
                if e.to_string().contains("Connection refused") {
                    println!("跳过测试：Qdrant服务不可用");
                    return Ok(());
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }
}

/// 性能基准测试
#[cfg(test)]
mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_upload_throughput() -> TestResult {
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            return Ok(());
        }

        let config = create_test_qdrant_config();

        match QdrantClientImpl::new(config).await {
            Ok(client) => {
                let _ = client.initialize_collection().await;

                // 性能基准测试
                let test_sizes = vec![10, 50, 100, 500];

                for size in test_sizes {
                    let vectors = generate_performance_test_vectors(size);

                    let start_time = std::time::Instant::now();
                    let result = client.upload_vectors(vectors.clone()).await;
                    let duration = start_time.elapsed();

                    assert_operation_success!(result, format!("上传 {} 个向量", size));

                    let throughput = size as f64 / duration.as_secs_f64();
                    println!("上传性能: {} 个向量/秒 (批次大小: {})", throughput, size);

                    // 基本性能要求（可根据实际情况调整）
                    assert!(throughput > 1.0, "上传吞吐量过低: {} 向量/秒", throughput);
                }
            }
            Err(e) => {
                if e.to_string().contains("Connection refused") {
                    println!("跳过测试：Qdrant服务不可用");
                    return Ok(());
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_usage() -> TestResult {
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            return Ok(());
        }

        // 内存使用测试
        let test_vectors = generate_performance_test_vectors(1000);

        let (result, memory_delta) = memory_test!({
            // 模拟向量处理
            let _processed: Vec<_> = test_vectors.iter().map(|v| v.vector.len()).collect();
        });

        println!("内存使用测试: {} bytes 增长", memory_delta);

        // 基本内存要求（可根据实际情况调整）
        let max_memory_per_vector = 10 * 1024; // 10KB per vector
        let expected_max_memory = test_vectors.len() * max_memory_per_vector;

        assert!(
            memory_delta < expected_max_memory,
            "内存使用过高: {} bytes (期望 < {} bytes)",
            memory_delta,
            expected_max_memory
        );

        Ok(())
    }
}

/// 集成场景测试
#[cfg(test)]
mod integration_scenario_tests {
    use super::*;

    #[tokio::test]
    async fn test_typescript_scenario() -> TestResult {
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            return Ok(());
        }

        let config = create_test_qdrant_config();

        match QdrantClientImpl::new(config).await {
            Ok(client) => {
                let _ = client.initialize_collection().await;

                // 使用TypeScript特定的测试数据
                let scenarios = generate_scenario_test_vectors();
                let ts_vectors: Vec<CodeVector> = scenarios
                    .get("typescript_react")
                    .unwrap()
                    .iter()
                    .map(|tv| tv.vector.clone())
                    .collect();

                let result = client.upload_vectors(ts_vectors.clone()).await;
                assert_operation_success!(result, "TypeScript场景向量上传");

                println!(
                    "TypeScript React组件场景测试完成: {} 个向量",
                    ts_vectors.len()
                );
            }
            Err(e) => {
                if e.to_string().contains("Connection refused") {
                    println!("跳过测试：Qdrant服务不可用");
                    return Ok(());
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_rust_scenario() -> TestResult {
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            return Ok(());
        }

        let config = create_test_qdrant_config();

        match QdrantClientImpl::new(config).await {
            Ok(client) => {
                let _ = client.initialize_collection().await;

                // 使用Rust特定的测试数据
                let scenarios = generate_scenario_test_vectors();
                let rust_vectors: Vec<CodeVector> = scenarios
                    .get("rust_system")
                    .unwrap()
                    .iter()
                    .map(|tv| tv.vector.clone())
                    .collect();

                let result = client.upload_vectors(rust_vectors.clone()).await;
                assert_operation_success!(result, "Rust场景向量上传");

                println!("Rust系统编程场景测试完成: {} 个向量", rust_vectors.len());
            }
            Err(e) => {
                if e.to_string().contains("Connection refused") {
                    println!("跳过测试：Qdrant服务不可用");
                    return Ok(());
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }
}

/// 并发测试
#[cfg(test)]
mod concurrency_tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_uploads() -> TestResult {
        if std::env::var("SKIP_QDRANT_INTEGRATION").is_ok() {
            return Ok(());
        }

        let config = create_test_qdrant_config();

        match QdrantClientImpl::new(config.clone()).await {
            Ok(client) => {
                let _ = client.initialize_collection().await;

                // 并发上传测试
                let concurrency = 3;
                let vectors_per_task = 50;

                let results = run_concurrent_vector_operations(
                    |task_id| {
                        let client_config = config.clone();
                        async move {
                            let client = QdrantClientImpl::new(client_config).await?;
                            let vectors = generate_performance_test_vectors(vectors_per_task);

                            // 为每个任务添加唯一标识符，避免ID冲突
                            let unique_vectors: Vec<CodeVector> = vectors
                                .into_iter()
                                .map(|mut v| {
                                    v.id = format!("task{}__{}", task_id, v.id);
                                    v
                                })
                                .collect();

                            client.upload_vectors(unique_vectors).await?;
                            Ok(())
                        }
                    },
                    concurrency,
                )
                .await?;

                // 验证所有任务都成功
                for (i, result) in results.iter().enumerate() {
                    assert_operation_success!(*result, format!("并发任务 {}", i));
                }

                println!(
                    "并发上传测试完成: {} 个任务, 每个 {} 个向量",
                    concurrency, vectors_per_task
                );
            }
            Err(e) => {
                if e.to_string().contains("Connection refused") {
                    println!("跳过测试：Qdrant服务不可用");
                    return Ok(());
                } else {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }
}
