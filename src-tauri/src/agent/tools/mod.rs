// Tools interface and builtins for Agent module
// Real implementation after migration

pub mod builtin;
pub mod logger;
pub mod metadata;
pub mod parallel;
pub mod registry;
pub mod r#trait;
// Re-exports for external use
pub use logger::ToolExecutionLogger;
pub use metadata::{BackoffStrategy, ExecutionMode, RateLimitConfig, ToolCategory, ToolMetadata, ToolPriority};
pub use parallel::{execute_batch, ToolCall, ToolCallResult};
pub use r#trait::{
    RunnableTool, ToolDescriptionContext, ToolPermission, ToolResult, ToolResultContent, ToolSchema,
};
pub use registry::{get_permissions_for_mode, ToolExecutionStats, ToolRegistry};

// Builtin tool type re-exports
pub use builtin::{
    ListFilesTool, OrbitSearchTool, ReadFileTool, ReadTerminalTool, ShellTool, UnifiedEditTool,
    WebFetchTool, WriteFileTool,
};

use std::sync::Arc;

pub async fn create_tool_registry(chat_mode: &str) -> Arc<ToolRegistry> {
    let permissions = get_permissions_for_mode(chat_mode);
    let registry = Arc::new(ToolRegistry::new(permissions));
    let is_chat = chat_mode == "chat";
    register_builtin_tools(&registry, is_chat).await;
    registry
}

async fn register_builtin_tools(registry: &ToolRegistry, is_chat_mode: bool) {
    use std::sync::Arc;

    registry
        .register("web_fetch", Arc::new(WebFetchTool::new()), is_chat_mode)
        .await
        .ok();

    registry
        .register("read_file", Arc::new(ReadFileTool::new()), is_chat_mode)
        .await
        .ok();
    registry
        .register("write_file", Arc::new(WriteFileTool::new()), is_chat_mode)
        .await
        .ok();
    registry
        .register("edit_file", Arc::new(UnifiedEditTool::new()), is_chat_mode)
        .await
        .ok();
    registry
        .register("list_files", Arc::new(ListFilesTool::new()), is_chat_mode)
        .await
        .ok();

    registry
        .register("shell", Arc::new(ShellTool::new()), is_chat_mode)
        .await
        .ok();
    registry
        .register(
            "orbit_search",
            Arc::new(OrbitSearchTool::new()),
            is_chat_mode,
        )
        .await
        .ok();
    registry
        .register(
            "read_terminal",
            Arc::new(ReadTerminalTool::new()),
            is_chat_mode,
        )
        .await
        .ok();
}
