// Tools interface and builtins for Agent module
// Real implementation after migration

pub mod builtin;
pub mod error;
pub mod logger;
pub mod registry;
pub mod r#trait;

// Re-exports for external use
pub use error::{ErrorSeverity, ToolExecutorError, ToolExecutorResult};
pub use logger::ToolExecutionLogger;
pub use r#trait::{RunnableTool, ToolPermission, ToolResult, ToolResultContent, ToolSchema};
pub use registry::{ToolExecutionStats, ToolRegistry};

// Builtin tool type re-exports
pub use builtin::{
    ApplyDiffTool, EditFileTool, InsertContentTool, ListCodeDefinitionNamesTool, ListFilesTool,
    OrbitSearchTool, ReadFileTool, ReadManyFilesTool, ShellTool, WebFetchTool, WriteToFileTool,
};

use std::sync::Arc;

/// 创建并初始化工具注册表（目前注册前端迁移中的工具骨架）
pub async fn create_tool_registry() -> Arc<ToolRegistry> {
    let registry = Arc::new(ToolRegistry::new());
    register_builtin_tools(&registry).await;
    registry
}

async fn register_builtin_tools(registry: &ToolRegistry) {
    use std::sync::Arc;
    use tracing::info;

    info!("注册 Agent 工具骨架（迁移中）");

    registry
        .register("apply_diff", Arc::new(ApplyDiffTool::new()))
        .await
        .ok();
    registry
        .register("insert_content", Arc::new(InsertContentTool::new()))
        .await
        .ok();
    registry
        .register("read_many_files", Arc::new(ReadManyFilesTool::new()))
        .await
        .ok();
    registry
        .register("web_fetch", Arc::new(WebFetchTool::new()))
        .await
        .ok();

    // 文件类工具骨架
    registry
        .register("read_file", Arc::new(ReadFileTool::new()))
        .await
        .ok();
    registry
        .register("write_to_file", Arc::new(WriteToFileTool::new()))
        .await
        .ok();
    registry
        .register("edit_file", Arc::new(EditFileTool::new()))
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

    // 其它工具骨架
    registry
        .register("shell", Arc::new(ShellTool::new()))
        .await
        .ok();
    registry
        .register("orbit_search", Arc::new(OrbitSearchTool::new()))
        .await
        .ok();
}
