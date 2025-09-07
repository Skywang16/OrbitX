/*!
 * 测试辅助函数和宏
 * 
 * 提供常用的测试辅助函数：
 * - 测试断言宏
 * - 内存使用测试
 * - 性能测试辅助
 */

/// 测试操作成功的断言宏
#[macro_export]
macro_rules! assert_operation_success {
    ($result:expr, $message:expr) => {
        match $result {
            Ok(_) => {
                println!("✅ {}: 操作成功", $message);
            }
            Err(e) => {
                panic!("❌ {}: 操作失败 - {}", $message, e);
            }
        }
    };
}

/// 测试操作失败的断言宏
#[macro_export]
macro_rules! assert_operation_failure {
    ($result:expr, $message:expr) => {
        match $result {
            Ok(_) => {
                panic!("❌ {}: 预期失败但操作成功", $message);
            }
            Err(_) => {
                println!("✅ {}: 预期失败且操作确实失败", $message);
            }
        }
    };
}

/// Qdrant连接成功断言宏
#[macro_export]
macro_rules! assert_qdrant_connection_success {
    ($result:expr) => {
        match $result {
            Ok(message) => {
                println!("✅ Qdrant连接成功: {}", message);
            }
            Err(e) => {
                panic!("❌ Qdrant连接失败: {}", e);
            }
        }
    };
}

/// 向量构建成功断言宏
#[macro_export]
macro_rules! assert_vector_build_success {
    ($result:expr) => {
        match $result {
            Ok(stats) => {
                println!("✅ 向量索引构建成功:");
                println!("   - 处理文件数: {}", stats.total_files);
                println!("   - 向量总数: {}", stats.total_chunks);
                println!("   - 处理时间: {:?}", stats.processing_time);
            }
            Err(e) => {
                panic!("❌ 向量索引构建失败: {}", e);
            }
        }
    };
}

/// 内存使用测试宏（简化版）
#[macro_export]
macro_rules! memory_test {
    ($block:block) => {{
        let initial_memory = get_memory_usage();
        let result = $block;
        let final_memory = get_memory_usage();
        let memory_delta = final_memory.saturating_sub(initial_memory);
        (result, memory_delta)
    }};
}

/// 获取当前内存使用量（简化实现）
pub fn get_memory_usage() -> usize {
    // 简化实现，实际项目中可以使用更精确的内存监控
    // 这里返回一个模拟值
    std::thread::current().id().as_u64() as usize % 1024
}

/// 创建测试用的临时目录
pub fn create_temp_test_dir() -> std::io::Result<tempfile::TempDir> {
    tempfile::TempDir::new()
}

/// 创建测试用的文件内容
pub fn create_test_files(
    temp_dir: &tempfile::TempDir,
    files: &[(&str, &str)],
) -> std::io::Result<Vec<String>> {
    use std::fs;
    use std::path::Path;

    let mut file_paths = Vec::new();

    for (file_name, content) in files {
        let file_path = temp_dir.path().join(file_name);
        
        // 创建父目录（如果需要）
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 写入文件内容
        fs::write(&file_path, content)?;
        file_paths.push(file_path.to_string_lossy().to_string());
    }

    Ok(file_paths)
}

/// 验证测试结果的通用函数
pub fn validate_test_result<T, E>(
    result: Result<T, E>,
    expected_success: bool,
    message: &str,
) -> Result<T, E>
where
    E: std::fmt::Display,
{
    match (&result, expected_success) {
        (Ok(_), true) => {
            println!("✅ {}: 测试通过（预期成功）", message);
        }
        (Err(e), false) => {
            println!("✅ {}: 测试通过（预期失败）- {}", message, e);
        }
        (Ok(_), false) => {
            panic!("❌ {}: 测试失败（预期失败但成功了）", message);
        }
        (Err(e), true) => {
            panic!("❌ {}: 测试失败（预期成功但失败了）- {}", message, e);
        }
    }
    result
}

/// 异步等待指定时间（用于测试中的时序控制）
pub async fn wait_for_duration(duration_ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(duration_ms)).await;
}

/// 重试执行函数直到成功或达到最大重试次数
pub async fn retry_until_success<F, T, E>(
    mut operation: F,
    max_retries: usize,
    delay_ms: u64,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Display,
{
    for attempt in 0..max_retries {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt < max_retries - 1 {
                    println!("重试 {}/{}: {}", attempt + 1, max_retries, e);
                    wait_for_duration(delay_ms).await;
                } else {
                    return Err(e);
                }
            }
        }
    }
    unreachable!()
}

/// 比较两个向量的相似度（用于测试向量搜索结果）
pub fn calculate_cosine_similarity(vec1: &[f32], vec2: &[f32]) -> f32 {
    if vec1.len() != vec2.len() {
        return 0.0;
    }

    let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
    let magnitude1: f32 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude2: f32 = vec2.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude1 == 0.0 || magnitude2 == 0.0 {
        0.0
    } else {
        dot_product / (magnitude1 * magnitude2)
    }
}

/// 验证搜索结果的排序是否正确（按分数降序）
pub fn validate_search_results_order(results: &[impl SearchResultLike]) -> bool {
    for i in 1..results.len() {
        if results[i - 1].score() < results[i].score() {
            return false;
        }
    }
    true
}

/// 用于搜索结果验证的trait
pub trait SearchResultLike {
    fn score(&self) -> f32;
}

// 为SearchResult实现trait
impl SearchResultLike for terminal_lib::vector_index::types::SearchResult {
    fn score(&self) -> f32 {
        self.score
    }
}
