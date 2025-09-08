/*!
 * 向量索引系统测试数据生成器
 *
 * 提供标准化的测试数据，包括：
 * - 模拟代码向量
 * - 测试配置
 * - Mock Qdrant服务
 * - 性能测试数据集
 */

use std::collections::HashMap;
use terminal_lib::vector_index::types::{
    CodeVector, SearchOptions, SearchResult, VectorIndexConfig,
};
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
        metadata.insert("created_at".to_string(), "2024-01-01T00:00:00Z".to_string());

        let vector = CodeVector {
            id: Uuid::new_v4().to_string(),
            file_path,
            content,
            start_line: (i * 10) as u32 + 1,
            end_line: (i * 10) as u32 + 15,
            language: language.to_string(),
            chunk_type: chunk_type.to_string(),
            vector: generate_dummy_vector(1536), // text-embedding-3-small dimensions
            metadata,
        };

        vectors.push(TestCodeVector {
            vector,
            expected_search_terms: expected_terms,
        });
    }

    vectors
}

/// 生成特定场景的测试向量
pub fn generate_scenario_test_vectors() -> HashMap<String, Vec<TestCodeVector>> {
    let mut scenarios = HashMap::new();

    // 场景1: TypeScript React组件
    scenarios.insert(
        "typescript_react".to_string(),
        vec![
            create_react_component_vector("Button", "export const Button"),
            create_react_component_vector("Modal", "export const Modal"),
            create_react_component_vector("Input", "export const Input"),
        ],
    );

    // 场景2: Rust系统编程
    scenarios.insert(
        "rust_system".to_string(),
        vec![
            create_rust_function_vector("parse_config", "pub fn parse_config"),
            create_rust_function_vector("handle_error", "pub fn handle_error"),
            create_rust_struct_vector("Configuration", "pub struct Configuration"),
        ],
    );

    // 场景3: Python数据处理
    scenarios.insert(
        "python_data".to_string(),
        vec![
            create_python_function_vector("process_data", "def process_data"),
            create_python_class_vector("DataProcessor", "class DataProcessor"),
            create_python_function_vector("analyze_results", "def analyze_results"),
        ],
    );

    scenarios
}

/// 创建测试用的Qdrant配置
pub fn create_test_qdrant_config() -> VectorIndexConfig {
    VectorIndexConfig {
        qdrant_url: "http://localhost:6334".to_string(),
        qdrant_api_key: None,
        collection_name: format!(
            "test_collection_{}",
            Uuid::new_v4().to_string()[..8].to_string()
        ),
        vector_size: 1536,
        batch_size: 50,
        supported_extensions: vec![
            ".ts".to_string(),
            ".tsx".to_string(),
            ".js".to_string(),
            ".jsx".to_string(),
            ".rs".to_string(),
            ".py".to_string(),
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

/// 创建高级搜索选项（带过滤）
pub fn create_filtered_search_options(
    query: &str,
    language: Option<&str>,
    chunk_type: Option<&str>,
) -> SearchOptions {
    SearchOptions {
        query: query.to_string(),
        max_results: Some(20),
        min_score: Some(0.2),
        directory_filter: None,
        language_filter: language.map(|s| s.to_string()),
        chunk_type_filter: chunk_type.map(|s| s.to_string()),
    }
}

/// 生成性能测试用的大量向量
pub fn generate_performance_test_vectors(count: usize) -> Vec<CodeVector> {
    let test_vectors = generate_test_code_vectors(count);
    test_vectors.into_iter().map(|tv| tv.vector).collect()
}

/// 创建错误测试用的无效向量
pub fn create_invalid_test_vectors() -> Vec<CodeVector> {
    vec![
        // 向量维度不匹配
        CodeVector {
            id: Uuid::new_v4().to_string(),
            file_path: "invalid/wrong_dimensions.ts".to_string(),
            content: "// 错误的向量维度".to_string(),
            start_line: 1,
            end_line: 5,
            language: "typescript".to_string(),
            chunk_type: "function".to_string(),
            vector: vec![0.1, 0.2], // 错误的维度（应该是1536）
            metadata: HashMap::new(),
        },
        // 空内容
        CodeVector {
            id: Uuid::new_v4().to_string(),
            file_path: "invalid/empty_content.rs".to_string(),
            content: "".to_string(), // 空内容
            start_line: 1,
            end_line: 1,
            language: "rust".to_string(),
            chunk_type: "function".to_string(),
            vector: generate_dummy_vector(1536),
            metadata: HashMap::new(),
        },
    ]
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

// === 特定场景的向量创建函数 ===

fn create_react_component_vector(name: &str, content_prefix: &str) -> TestCodeVector {
    let content = format!(
        "{} = (props: {}Props) => {{\n  return <div>{{props.children}}</div>;\n}};",
        content_prefix, name
    );

    TestCodeVector {
        vector: CodeVector {
            id: Uuid::new_v4().to_string(),
            file_path: format!("components/{}.tsx", name),
            content,
            start_line: 1,
            end_line: 5,
            language: "typescript".to_string(),
            chunk_type: "function".to_string(),
            vector: generate_dummy_vector(1536),
            metadata: {
                let mut map = HashMap::new();
                map.insert("component_type".to_string(), "react".to_string());
                map.insert("ui_component".to_string(), "true".to_string());
                map
            },
        },
        expected_search_terms: vec![
            name.to_lowercase(),
            "react".to_string(),
            "component".to_string(),
            "typescript".to_string(),
        ],
    }
}

fn create_rust_function_vector(name: &str, content_prefix: &str) -> TestCodeVector {
    let content = format!(
        "{}(input: &str) -> Result<String, Error> {{\n    Ok(input.to_string())\n}}",
        content_prefix
    );

    TestCodeVector {
        vector: CodeVector {
            id: Uuid::new_v4().to_string(),
            file_path: format!("src/{}.rs", name),
            content,
            start_line: 1,
            end_line: 5,
            language: "rust".to_string(),
            chunk_type: "function".to_string(),
            vector: generate_dummy_vector(1536),
            metadata: {
                let mut map = HashMap::new();
                map.insert("visibility".to_string(), "public".to_string());
                map.insert("error_handling".to_string(), "result".to_string());
                map
            },
        },
        expected_search_terms: vec![
            name.to_string(),
            "rust".to_string(),
            "function".to_string(),
            "result".to_string(),
        ],
    }
}

fn create_rust_struct_vector(name: &str, content_prefix: &str) -> TestCodeVector {
    let content = format!(
        "{} {{\n    field: String,\n}}\n\nimpl {} {{\n    pub fn new() -> Self {{\n        Self {{ field: String::new() }}\n    }}\n}}",
        content_prefix, name
    );

    TestCodeVector {
        vector: CodeVector {
            id: Uuid::new_v4().to_string(),
            file_path: format!("src/{}.rs", name.to_lowercase()),
            content,
            start_line: 1,
            end_line: 10,
            language: "rust".to_string(),
            chunk_type: "struct".to_string(),
            vector: generate_dummy_vector(1536),
            metadata: {
                let mut map = HashMap::new();
                map.insert("type".to_string(), "struct".to_string());
                map.insert("has_impl".to_string(), "true".to_string());
                map
            },
        },
        expected_search_terms: vec![
            name.to_lowercase(),
            "rust".to_string(),
            "struct".to_string(),
            "impl".to_string(),
        ],
    }
}

fn create_python_function_vector(name: &str, content_prefix: &str) -> TestCodeVector {
    let content = format!(
        "{}(data):\n    \"\"\"处理数据并返回结果\"\"\"\n    return data.process()",
        content_prefix
    );

    TestCodeVector {
        vector: CodeVector {
            id: Uuid::new_v4().to_string(),
            file_path: format!("src/{}.py", name),
            content,
            start_line: 1,
            end_line: 5,
            language: "python".to_string(),
            chunk_type: "function".to_string(),
            vector: generate_dummy_vector(1536),
            metadata: {
                let mut map = HashMap::new();
                map.insert("docstring".to_string(), "true".to_string());
                map.insert("data_processing".to_string(), "true".to_string());
                map
            },
        },
        expected_search_terms: vec![
            name.to_string(),
            "python".to_string(),
            "function".to_string(),
            "data".to_string(),
        ],
    }
}

fn create_python_class_vector(name: &str, content_prefix: &str) -> TestCodeVector {
    let content = format!(
        "{}:\n    def __init__(self):\n        self.data = []\n    \n    def process(self):\n        return len(self.data)",
        content_prefix
    );

    TestCodeVector {
        vector: CodeVector {
            id: Uuid::new_v4().to_string(),
            file_path: format!("src/{}.py", name.to_lowercase()),
            content,
            start_line: 1,
            end_line: 8,
            language: "python".to_string(),
            chunk_type: "class".to_string(),
            vector: generate_dummy_vector(1536),
            metadata: {
                let mut map = HashMap::new();
                map.insert("has_init".to_string(), "true".to_string());
                map.insert("has_methods".to_string(), "true".to_string());
                map
            },
        },
        expected_search_terms: vec![
            name.to_lowercase(),
            "python".to_string(),
            "class".to_string(),
            "init".to_string(),
        ],
    }
}

/// Mock Qdrant 服务配置
pub struct MockQdrantConfig {
    pub simulate_latency: bool,
    pub simulate_errors: bool,
    pub error_rate: f32,
    pub latency_ms: u64,
}

impl Default for MockQdrantConfig {
    fn default() -> Self {
        Self {
            simulate_latency: false,
            simulate_errors: false,
            error_rate: 0.0,
            latency_ms: 0,
        }
    }
}

/// 创建Mock Qdrant配置用于错误测试
pub fn create_error_test_mock_config() -> MockQdrantConfig {
    MockQdrantConfig {
        simulate_latency: true,
        simulate_errors: true,
        error_rate: 0.1, // 10% 错误率
        latency_ms: 100,
    }
}

/// 创建性能测试Mock配置
pub fn create_performance_test_mock_config() -> MockQdrantConfig {
    MockQdrantConfig {
        simulate_latency: true,
        simulate_errors: false,
        error_rate: 0.0,
        latency_ms: 50,
    }
}

// === 统一的搜索结果创建函数 ===

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
        content: format!(
            "content for {} lines {}-{}",
            file_path, start_line, end_line
        ),
        start_line,
        end_line,
        language: "typescript".to_string(),
        chunk_type: "function".to_string(),
        score,
        metadata: HashMap::new(),
    }
}

/// 创建带指定内容的测试搜索结果
pub fn create_test_search_result_with_content(
    file_path: &str,
    content: &str,
    score: f32,
) -> SearchResult {
    SearchResult {
        file_path: file_path.to_string(),
        content: content.to_string(),
        start_line: 1,
        end_line: 10,
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
