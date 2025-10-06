// Tools interface and builtins for Agent module
// Real implementation after migration

pub mod builtin;
pub mod error;
pub mod logger;
pub mod metadata;
pub mod registry;
pub mod r#trait;

// Re-exports for external use
pub use error::{ErrorSeverity, ToolExecutorError, ToolExecutorResult};
pub use logger::ToolExecutionLogger;
pub use metadata::{BackoffStrategy, RateLimitConfig, ToolCategory, ToolMetadata, ToolPriority};
pub use r#trait::{RunnableTool, ToolPermission, ToolResult, ToolResultContent, ToolSchema};
pub use registry::{ToolExecutionStats, ToolRegistry};

// Builtin tool type re-exports
pub use builtin::{
    ListCodeDefinitionNamesTool, ListFilesTool, OrbitSearchTool, ReadFileTool, ReadManyFilesTool,
    ShellTool, UnifiedEditTool, WebFetchTool, WriteFileTool,
};

use std::sync::Arc;

pub async fn create_tool_registry() -> Arc<ToolRegistry> {
    let registry = Arc::new(ToolRegistry::new());
    register_builtin_tools(&registry).await;
    registry
}

async fn register_builtin_tools(registry: &ToolRegistry) {
    use std::sync::Arc;
    use tracing::info;

    info!("注册 Agent 工具集");

    registry
        .register("read_many_files", Arc::new(ReadManyFilesTool::new()))
        .await
        .ok();
    registry
        .register("web_fetch", Arc::new(WebFetchTool::new()))
        .await
        .ok();

    registry
        .register("read_file", Arc::new(ReadFileTool::new()))
        .await
        .ok();
    registry
        .register("write_file", Arc::new(WriteFileTool::new()))
        .await
        .ok();
    registry
        .register("edit_file", Arc::new(UnifiedEditTool::new()))
        .await
        .ok();
    registry
        .register("list_files", Arc::new(ListFilesTool::new()))
        .await
        .ok();
    registry
        .register(
            "list_code_definition_names",
            Arc::new(ListCodeDefinitionNamesTool::new()),
        )
        .await
        .ok();

    registry
        .register("shell", Arc::new(ShellTool::new()))
        .await
        .ok();
    registry
        .register("orbit_search", Arc::new(OrbitSearchTool::new()))
        .await
        .ok();
}
