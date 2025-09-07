/*!
 * 测试固定数据和辅助函数
 * 
 * 提供标准化的测试数据，包括：
 * - 模拟代码向量
 * - 测试配置  
 * - 性能测试数据集
 */

use std::collections::HashMap;
use terminal_lib::vector_index::types::{SearchOptions, SearchResult, VectorIndexConfig, CodeVector};
use uuid::Uuid;

/// 测试用代码向量
#[derive(Debug, Clone)]
pub struct TestCodeVector {
    pub vector: CodeVector,
    pub expected_search_terms: Vec<String>,
}

/// 生成测试用的代码向量
pub fn generate_test_code_vectors(count: usize) -> Vec<TestCodeVector> {
    let mut vectors = Vec::new();

    for i in 0..count {
        let language = match i % 4 {
            0 => "typescript",
            1 => "rust", 
            2 => "python",
            _ => "javascript",
        };

        let chunk_type = match i % 3 {
            0 => "function",
            1 => "class",
            _ => "method",
        };

        let file_path = format!("test/file_{}.{}", i, get_extension_for_language(language));
        let content = generate_code_content(language, chunk_type, i);
        let expected_terms = generate_expected_search_terms(language, chunk_type, i);

        let mut metadata = HashMap::new();
        metadata.insert("complexity".to_string(), format!("{}", i % 5 + 1));
        metadata.insert("author".to_string(), "test_user".to_string());

        let vector = CodeVector {
            id: Uuid::new_v4().to_string(),
            file_path,
            content,
            start_line: (i * 10) as u32 + 1,
            end_line: (i * 10) as u32 + 15,
            language: language.to_string(),
            chunk_type: chunk_type.to_string(),
            vector: generate_dummy_vector(1536),
            metadata,
        };

        vectors.push(TestCodeVector {
            vector,
            expected_search_terms: expected_terms,
        });
    }

    vectors
}

/// 创建测试用的Qdrant配置
pub fn create_test_qdrant_config() -> VectorIndexConfig {
    VectorIndexConfig {
        qdrant_url: "http://localhost:6333".to_string(),
        qdrant_api_key: None,
        collection_name: format!("test_collection_{}", Uuid::new_v4().to_string()[..8].to_string()),
        vector_size: 1536,
        batch_size: 50,
        supported_extensions: vec![
            ".ts".to_string(), ".tsx".to_string(), ".js".to_string(), ".jsx".to_string(),
            ".rs".to_string(), ".py".to_string(),
        ],
        ignore_patterns: vec![
            "**/node_modules/**".to_string(),
            "**/target/**".to_string(),
            "**/.git/**".to_string(),
        ],
        max_concurrent_files: 4,
        chunk_size_range: [10, 2000],
    }
}

/// 创建测试用的搜索选项
pub fn create_test_search_options(query: &str) -> SearchOptions {
    SearchOptions {
        query: query.to_string(),
        max_results: Some(10),
        min_score: Some(0.3),
        directory_filter: None,
        language_filter: None,
        chunk_type_filter: None,
    }
}

/// 生成性能测试用的大量向量
pub fn generate_performance_test_vectors(count: usize) -> Vec<CodeVector> {
    let test_vectors = generate_test_code_vectors(count);
    test_vectors.into_iter().map(|tv| tv.vector).collect()
}

/// 创建基础测试搜索结果
pub fn create_test_search_result(file_path: &str, score: f32) -> SearchResult {
    SearchResult {
        file_path: file_path.to_string(),
        content: format!("function test() {{ return 'content for {}'; }}", file_path),
        start_line: 1,
        end_line: 5,
        language: "typescript".to_string(),
        chunk_type: "function".to_string(),
        score,
        metadata: HashMap::new(),
    }
}

/// 创建详细的测试搜索结果
pub fn create_test_search_result_detailed(
    file_path: &str,
    start_line: u32,
    end_line: u32,
    score: f32,
) -> SearchResult {
    SearchResult {
        file_path: file_path.to_string(),
        content: format!("content for {} lines {}-{}", file_path, start_line, end_line),
        start_line,
        end_line,
        language: "typescript".to_string(),
        chunk_type: "function".to_string(),
        score,
        metadata: HashMap::new(),
    }
}

/// 创建模拟搜索结果集合
pub fn create_mock_search_results(count: usize) -> Vec<SearchResult> {
    (0..count)
        .map(|i| SearchResult {
            file_path: format!("test/file_{}.ts", i),
            content: format!("function test{}() {{ return {}; }}", i, i),
            start_line: (i * 10) as u32 + 1,
            end_line: (i * 10) as u32 + 5,
            language: "typescript".to_string(),
            chunk_type: "function".to_string(),
            score: 1.0 - (i as f32 * 0.1),
            metadata: HashMap::new(),
        })
        .collect()
}

// === 私有辅助函数 ===

fn get_extension_for_language(language: &str) -> &str {
    match language {
        "typescript" => "ts",
        "javascript" => "js", 
        "rust" => "rs",
        "python" => "py",
        _ => "txt",
    }
}

fn generate_code_content(language: &str, chunk_type: &str, index: usize) -> String {
    match (language, chunk_type) {
        ("typescript", "function") => format!(
            "export function testFunction{}(param: string): string {{\n  return `Test ${{param}} - {}`;\n}}",
            index, index
        ),
        ("typescript", "class") => format!(
            "export class TestClass{} {{\n  private value: number = {};\n  \n  getValue(): number {{\n    return this.value;\n  }}\n}}",
            index, index
        ),
        ("rust", "function") => format!(
            "pub fn test_function_{}(param: &str) -> String {{\n    format!(\"Test {{}} - {{}}\", param, {})\n}}",
            index, index
        ),
        ("rust", "class") => format!(
            "pub struct TestStruct{} {{\n    pub value: i32,\n}}\n\nimpl TestStruct{} {{\n    pub fn new() -> Self {{\n        Self {{ value: {} }}\n    }}\n}}",
            index, index, index
        ),
        ("python", "function") => format!(
            "def test_function_{}(param):\n    return f'Test {{param}} - {}'",
            index, index
        ),
        ("python", "class") => format!(
            "class TestClass{}:\n    def __init__(self):\n        self.value = {}\n    \n    def get_value(self):\n        return self.value",
            index, index
        ),
        _ => format!("// 通用测试代码 {}\nfunction test() {{ return {}; }}", index, index),
    }
}

fn generate_expected_search_terms(language: &str, chunk_type: &str, index: usize) -> Vec<String> {
    let mut terms = vec![
        format!("test{}", index),
        language.to_string(),
        chunk_type.to_string(),
    ];

    match language {
        "typescript" | "javascript" => {
            terms.push("function".to_string());
            terms.push("export".to_string());
        }
        "rust" => {
            terms.push("pub".to_string());
            terms.push("struct".to_string()); 
        }
        "python" => {
            terms.push("def".to_string());
            terms.push("class".to_string());
        }
        _ => {}
    }

    terms
}

fn generate_dummy_vector(size: usize) -> Vec<f32> {
    (0..size).map(|i| (i as f32) / (size as f32)).collect()
}
