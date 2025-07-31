//! 补全命令功能测试

use std::sync::{Arc, Mutex};
use terminal_lib::completion::{CompletionRequest, CompletionEngineState};

#[test]
fn test_completion_request_creation() {
    let request = CompletionRequest {
        input: "ls /home".to_string(),
        cursor_position: 8,
        working_directory: "/tmp".to_string(),
        max_results: Some(10),
    };

    assert_eq!(request.input, "ls /home");
    assert_eq!(request.cursor_position, 8);
    assert_eq!(request.working_directory, "/tmp");
    assert_eq!(request.max_results, Some(10));
}

#[test]
fn test_completion_request_without_max_results() {
    let request = CompletionRequest {
        input: "cd".to_string(),
        cursor_position: 2,
        working_directory: "/home/user".to_string(),
        max_results: None,
    };

    assert_eq!(request.input, "cd");
    assert_eq!(request.cursor_position, 2);
    assert_eq!(request.working_directory, "/home/user");
    assert!(request.max_results.is_none());
}

#[tokio::test]
async fn test_engine_state_management() {
    let state: CompletionEngineState = Arc::new(Mutex::new(None));

    // 初始状态应该为None
    {
        let guard = state.lock().unwrap();
        assert!(guard.is_none());
    }

    // 这里我们不能直接测试初始化，因为需要实际的Tauri状态
    // 但我们可以测试状态的基本操作
}

#[test]
fn test_completion_request_serialization() {
    let request = CompletionRequest {
        input: "git status".to_string(),
        cursor_position: 10,
        working_directory: "/project".to_string(),
        max_results: Some(5),
    };

    // 测试序列化
    let serialized = serde_json::to_string(&request).unwrap();
    assert!(serialized.contains("git status"));
    assert!(serialized.contains("10"));
    assert!(serialized.contains("/project"));
    assert!(serialized.contains("5"));

    // 测试反序列化
    let deserialized: CompletionRequest = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.input, request.input);
    assert_eq!(deserialized.cursor_position, request.cursor_position);
    assert_eq!(deserialized.working_directory, request.working_directory);
    assert_eq!(deserialized.max_results, request.max_results);
}

#[test]
fn test_completion_request_edge_cases() {
    // 空输入
    let empty_request = CompletionRequest {
        input: "".to_string(),
        cursor_position: 0,
        working_directory: "/".to_string(),
        max_results: None,
    };
    assert!(empty_request.input.is_empty());
    assert_eq!(empty_request.cursor_position, 0);

    // 长输入
    let long_input = "a".repeat(1000);
    let long_request = CompletionRequest {
        input: long_input.clone(),
        cursor_position: 1000,
        working_directory: "/very/long/path/to/test/directory".to_string(),
        max_results: Some(100),
    };
    assert_eq!(long_request.input.len(), 1000);
    assert_eq!(long_request.cursor_position, 1000);
}
