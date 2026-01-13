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
pub use metadata::{
    BackoffStrategy, ExecutionMode, RateLimitConfig, ToolCategory, ToolMetadata, ToolPriority,
};
pub use parallel::{execute_batch, ToolCall, ToolCallResult};
pub use r#trait::{
    RunnableTool, ToolDescriptionContext, ToolResult, ToolResultContent, ToolResultStatus,
    ToolSchema,
};
pub use registry::{ToolExecutionStats, ToolRegistry};

// Builtin tool type re-exports
pub use builtin::{
    ListFilesTool, OrbitSearchTool, ReadFileTool, ReadTerminalTool, ShellTool, TodoWriteTool,
    SyntaxDiagnosticsTool, UnifiedEditTool, WebFetchTool, WriteFileTool,
};

use std::sync::Arc;

pub async fn create_tool_registry(
    chat_mode: &str,
    permission_rules: crate::settings::types::PermissionRules,
    extra_tools: Vec<Arc<dyn RunnableTool>>,
) -> Arc<ToolRegistry> {
    let checker = Arc::new(crate::agent::permissions::PermissionChecker::new(&permission_rules));
    let registry = Arc::new(ToolRegistry::new(Some(checker)));
    let is_chat = chat_mode == "chat";
    register_builtin_tools(&registry, is_chat).await;

    for tool in extra_tools {
        let name = tool.name().to_string();
        registry.register(&name, tool, is_chat).await.ok();
    }

    registry
}

async fn register_builtin_tools(registry: &ToolRegistry, is_chat_mode: bool) {
    use std::sync::Arc;

    registry
        .register("todowrite", Arc::new(TodoWriteTool::new()), is_chat_mode)
        .await
        .ok();

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

    registry
        .register(
            "syntax_diagnostics",
            Arc::new(SyntaxDiagnosticsTool::new()),
            is_chat_mode,
        )
        .await
        .ok();
}
