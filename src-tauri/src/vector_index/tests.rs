/*!
 * 向量索引系统单元测试
 *
 * 测试优化后的关键组件功能
 */

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Write;
    use tempfile::NamedTempFile;

    mod smart_chunker_tests {
        use crate::vector_index::parser::smart_chunker::SmartChunker;
        use crate::vector_index::types::ChunkType;

        #[test]
        fn test_small_content_no_chunking() {
            let chunker = SmartChunker::new();
            let content = "fn small() { println!(\"hello\"); }";

            let result = chunker
                .chunk_large_content(content, "test.rs", "hash123", ChunkType::Function, 1)
                .unwrap();

            assert_eq!(result.len(), 1);
            assert_eq!(result[0].content, content);
            assert_eq!(result[0].start_line, 1);
            assert_eq!(result[0].chunk_type, ChunkType::Function);
        }

        #[test]
        fn test_oversized_line_segmentation() {
            let chunker = SmartChunker::new();
            let large_line = "a".repeat(3000); // 超过 effective_max_size

            let result = chunker
                .chunk_large_content(&large_line, "test.rs", "hash123", ChunkType::Other, 1)
                .unwrap();

            assert!(result.len() > 1);
            assert!(result.iter().all(|chunk| chunk.content.len() <= 2000));
        }

        #[test]
        fn test_content_hash_generation() {
            let chunker = SmartChunker::new();
            let content = "test content";

            let hash1 = chunker.generate_content_hash(content, "file1.rs", 1, 5);
            let hash2 = chunker.generate_content_hash(content, "file1.rs", 1, 5);
            let hash3 = chunker.generate_content_hash(content, "file2.rs", 1, 5);

            assert_eq!(hash1, hash2); // 相同内容应产生相同哈希
            assert_ne!(hash1, hash3); // 不同文件应产生不同哈希
        }
    }

    mod enhanced_config_tests {
        use crate::vector_index::enhanced_config::EnhancedConfigManager;

        #[test]
        fn test_parse_localhost_url() {
            let manager = EnhancedConfigManager::new();

            // 基础localhost
            assert_eq!(
                manager.parse_and_validate_url("localhost").unwrap(),
                "http://localhost:6334"
            );

            // 带端口的localhost
            assert_eq!(
                manager.parse_and_validate_url("localhost:6334").unwrap(),
                "http://localhost:6334"
            );

            // IP地址
            assert_eq!(
                manager.parse_and_validate_url("127.0.0.1").unwrap(),
                "http://127.0.0.1:6334"
            );
        }

        #[test]
        fn test_parse_cloud_url() {
            let manager = EnhancedConfigManager::new();

            // Qdrant Cloud URL
            let cloud_url = "my-cluster.us-east-1.aws.cloud.qdrant.io";
            assert_eq!(
                manager.parse_and_validate_url(cloud_url).unwrap(),
                format!("https://{}:6334", cloud_url)
            );
        }

        #[test]
        fn test_invalid_port_detection() {
            let manager = EnhancedConfigManager::new();

            // 错误的REST端口
            let result = manager.parse_and_validate_url("http://localhost:6333");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("6333"));
        }

        #[test]
        fn test_empty_url_default() {
            let manager = EnhancedConfigManager::new();

            assert_eq!(
                manager.parse_and_validate_url("").unwrap(),
                "http://localhost:6334"
            );

            assert_eq!(
                manager.parse_and_validate_url("   ").unwrap(),
                "http://localhost:6334"
            );
        }

        #[test]
        fn test_url_with_path_prefix() {
            let manager = EnhancedConfigManager::new();

            let result = manager
                .parse_and_validate_url("http://example.com:6334/qdrant/")
                .unwrap();
            assert_eq!(result, "http://example.com:6334/qdrant");
        }
    }

    mod error_handler_tests {
        use crate::vector_index::error_handler::EnhancedErrorHandler;
        use crate::vector_index::types::VectorIndexFullConfig;

        #[test]
        fn test_error_classification() {
            let config = VectorIndexFullConfig::default();
            let handler = EnhancedErrorHandler::new(config);

            let dimension_error = anyhow::anyhow!("dimension mismatch: expected 1536, got 768");
            let (category, suggestion) = handler.classify_error(&dimension_error);

            assert_eq!(category, "维度不匹配");
            assert!(suggestion.contains("检查模型配置"));
        }

        #[test]
        fn test_operation_tracking() {
            let config = VectorIndexFullConfig::default();
            let mut handler = EnhancedErrorHandler::new(config);

            handler.start_operation("测试操作");
            handler.add_step("步骤1");
            handler.add_step("步骤2");

            handler.complete_step("步骤1");
            handler.fail_step("步骤2", "测试错误");

            let (completed, failed, total) = handler.get_operation_summary();
            assert_eq!(completed, 1);
            assert_eq!(failed, 1);
            assert_eq!(total, 2);
        }
    }

    mod performance_monitor_tests {
        use crate::vector_index::performance_monitor::{MetricType, PerformanceMonitor};
        use std::collections::HashMap;
        use std::time::Duration;

        #[test]
        fn test_metric_recording() {
            let monitor = PerformanceMonitor::new();

            monitor.record_metric(MetricType::SearchResponse, 150.0, HashMap::new());
            monitor.record_metric(MetricType::SearchResponse, 200.0, HashMap::new());

            let stats = monitor.get_performance_stats();
            assert_eq!(stats.avg_response_time, 175.0);
        }

        #[test]
        fn test_operation_tracking() {
            let monitor = PerformanceMonitor::new();

            monitor.start_operation("test_op".to_string());
            std::thread::sleep(Duration::from_millis(10));
            let duration = monitor.end_operation("test_op".to_string(), MetricType::FileParsing);

            assert!(duration.is_some());
            assert!(duration.unwrap().as_millis() >= 10);
        }

        #[test]
        fn test_batch_performance_recording() {
            let monitor = PerformanceMonitor::new();

            monitor.record_batch_performance(100, Duration::from_millis(500), 95);

            let stats = monitor.get_performance_stats();
            assert!(stats.error_rate < 0.1); // 95/100 = 5% error rate
        }

        #[test]
        fn test_health_check() {
            let monitor = PerformanceMonitor::new();

            // 记录一些好的指标
            monitor.record_metric(MetricType::SearchResponse, 100.0, HashMap::new());
            monitor.record_metric(MetricType::ErrorRate, 0.01, HashMap::new());

            let health = monitor.check_health();
            assert!(matches!(
                health.level,
                crate::vector_index::performance_monitor::HealthLevel::Good
                    | crate::vector_index::performance_monitor::HealthLevel::Excellent
            ));
        }

        #[test]
        fn test_optimization_suggestions() {
            let monitor = PerformanceMonitor::new();

            // 记录一些性能问题
            monitor.record_metric(MetricType::SearchResponse, 2000.0, HashMap::new()); // 慢响应
            monitor.record_metric(MetricType::ErrorRate, 0.15, HashMap::new()); // 高错误率

            let suggestions = monitor.generate_optimization_suggestions();
            assert!(!suggestions.is_empty());
            assert!(suggestions.iter().any(|s| s.contains("响应时间")));
        }
    }

    mod vector_index_integration_tests {
        use crate::vector_index::types::{SearchOptions, VectorIndexConfig};

        #[test]
        fn test_default_config_creation() {
            let config = VectorIndexConfig::default();

            assert_eq!(config.qdrant_url, "http://localhost:6334");
            assert_eq!(config.collection_name, "orbitx-code-vectors");
            assert_eq!(config.max_concurrent_files, 4);
        }

        #[test]
        fn test_search_options_creation() {
            let options = SearchOptions {
                query: "test function".to_string(),
                max_results: Some(10),
                min_score: Some(0.5),
                directory_filter: Some("src/".to_string()),
                language_filter: Some("rust".to_string()),
                chunk_type_filter: Some("function".to_string()),
                min_content_length: Some(50),
                max_content_length: Some(2000),
            };

            assert_eq!(options.query, "test function");
            assert_eq!(options.max_results, Some(10));
            assert_eq!(options.directory_filter, Some("src/".to_string()));
        }
    }

    mod path_segment_tests {
        use std::path::Path;

        #[test]
        fn test_path_segment_parsing() {
            let file_path = "src/components/ui/Button.tsx";
            let normalized_path = file_path.replace('\\', "/");
            let segments: Vec<&str> = normalized_path
                .split('/')
                .filter(|s| !s.is_empty())
                .collect();

            assert_eq!(segments, vec!["src", "components", "ui", "Button.tsx"]);
        }

        #[test]
        fn test_windows_path_normalization() {
            let windows_path = "src\\components\\ui\\Button.tsx";
            let normalized = windows_path.replace('\\', "/");

            assert_eq!(normalized, "src/components/ui/Button.tsx");
        }

        #[test]
        fn test_path_depth_calculation() {
            let paths = vec![
                "file.ts",
                "src/file.ts",
                "src/components/file.ts",
                "src/components/ui/file.ts",
            ];

            for (i, path) in paths.iter().enumerate() {
                let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
                assert_eq!(segments.len(), i + 1);
            }
        }
    }
}

// 集成测试模块（需要实际组件）
#[cfg(test)]
mod integration_tests {
    use super::*;

    // 注意：这些测试需要实际的组件初始化，在CI环境中可能需要mock

    #[tokio::test]
    #[ignore] // 需要实际的LLM服务和Qdrant连接
    async fn test_full_indexing_pipeline() {
        // 完整的环境设置测试，在CI/CD中需要完整的依赖服务
        // 测试步骤：
        // 1. 创建临时测试文件
        // 2. 初始化向量索引服务
        // 3. 执行索引构建
        // 4. 验证结果
        // 5. 执行搜索测试
        // 6. 验证搜索结果
    }

    #[tokio::test]
    #[ignore]
    async fn test_incremental_update_workflow() {
        // 测试增量更新的完整工作流
        // 1. 构建初始索引
        // 2. 修改文件
        // 3. 执行增量更新
        // 4. 验证更新结果
    }
}

// 基准测试（用于性能回归检测）
#[cfg(test)]
mod benchmark_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn bench_smart_chunker_performance() {
        let chunker = crate::vector_index::parser::smart_chunker::SmartChunker::new();
        let large_content = "fn test() { println!(\"hello\"); }".repeat(100);

        let start = Instant::now();

        for _ in 0..100 {
            let _ = chunker.chunk_large_content(
                &large_content,
                "test.rs",
                "hash123",
                crate::vector_index::types::ChunkType::Function,
                1,
            );
        }

        let duration = start.elapsed();

        // 性能基准：100次操作应在100ms内完成
        assert!(
            duration.as_millis() < 100,
            "Smart chunker performance regression detected"
        );
    }

    #[test]
    fn bench_url_parsing_performance() {
        let manager = crate::vector_index::enhanced_config::EnhancedConfigManager::new();
        let urls = vec![
            "localhost",
            "http://localhost:6334",
            "https://my-cluster.us-east-1.aws.cloud.qdrant.io:6334",
            "http://192.168.1.100:6334",
        ];

        let start = Instant::now();

        for _ in 0..1000 {
            for url in &urls {
                let _ = manager.parse_and_validate_url(url);
            }
        }

        let duration = start.elapsed();

        // 性能基准：4000次URL解析应在50ms内完成
        assert!(
            duration.as_millis() < 50,
            "URL parsing performance regression detected"
        );
    }
}
