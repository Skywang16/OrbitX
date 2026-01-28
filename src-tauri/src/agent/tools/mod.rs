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
    RunnableTool, ToolAvailabilityContext, ToolDescriptionContext, ToolResult, ToolResultContent,
    ToolResultStatus, ToolSchema,
};
pub use registry::{ToolExecutionStats, ToolRegistry};
pub use registry::ToolConfirmationManager;

// Builtin tool type re-exports
pub use builtin::{
    GrepTool, ListFilesTool, ReadFileTool, ReadTerminalTool, SemanticSearchTool, ShellTool,
    SyntaxDiagnosticsTool, TaskTool, TodoWriteTool, UnifiedEditTool, WebFetchTool, WriteFileTool,
};

use std::sync::Arc;

pub async fn create_tool_registry(
    chat_mode: &str,
    permission_rules: crate::settings::types::PermissionRules,
    agent_tool_filter: Option<crate::agent::permissions::ToolFilter>,
    confirmations: Arc<ToolConfirmationManager>,
    extra_tools: Vec<Arc<dyn RunnableTool>>,
    vector_search_engine: Option<Arc<crate::vector_db::search::SemanticSearchEngine>>,
) -> Arc<ToolRegistry> {
    let checker = Arc::new(crate::agent::permissions::PermissionChecker::new(
        &permission_rules,
    ));
    let agent_filter = agent_tool_filter.map(Arc::new);
    let registry = Arc::new(ToolRegistry::new(Some(checker), agent_filter, confirmations));
    let is_chat = chat_mode == "chat";

    let availability_ctx = ToolAvailabilityContext {
        has_vector_index: vector_search_engine.is_some(),
    };

    register_builtin_tools(&registry, is_chat, &availability_ctx, vector_search_engine).await;

    for tool in extra_tools {
        let name = tool.name().to_string();
        registry
            .register(&name, tool, is_chat, &availability_ctx)
            .await
            .ok();
    }

    registry
}

async fn register_builtin_tools(
    registry: &ToolRegistry,
    is_chat_mode: bool,
    availability_ctx: &ToolAvailabilityContext,
    vector_search_engine: Option<Arc<crate::vector_db::search::SemanticSearchEngine>>,
) {
    use std::sync::Arc;

    registry
        .register(
            "task",
            Arc::new(TaskTool::new()),
            is_chat_mode,
            availability_ctx,
        )
        .await
        .ok();

    registry
        .register(
            "todowrite",
            Arc::new(TodoWriteTool::new()),
            is_chat_mode,
            availability_ctx,
        )
        .await
        .ok();

    registry
        .register(
            "web_fetch",
            Arc::new(WebFetchTool::new()),
            is_chat_mode,
            availability_ctx,
        )
        .await
        .ok();

    registry
        .register(
            "read_file",
            Arc::new(ReadFileTool::new()),
            is_chat_mode,
            availability_ctx,
        )
        .await
        .ok();
    registry
        .register(
            "write_file",
            Arc::new(WriteFileTool::new()),
            is_chat_mode,
            availability_ctx,
        )
        .await
        .ok();
    registry
        .register(
            "edit_file",
            Arc::new(UnifiedEditTool::new()),
            is_chat_mode,
            availability_ctx,
        )
        .await
        .ok();
    registry
        .register(
            "list_files",
            Arc::new(ListFilesTool::new()),
            is_chat_mode,
            availability_ctx,
        )
        .await
        .ok();

    registry
        .register(
            "shell",
            Arc::new(ShellTool::new()),
            is_chat_mode,
            availability_ctx,
        )
        .await
        .ok();

    // 搜索工具
    registry
        .register(
            "grep",
            Arc::new(GrepTool::new()),
            is_chat_mode,
            availability_ctx,
        )
        .await
        .ok();
    if let Some(engine) = vector_search_engine {
        registry
            .register(
                "semantic_search",
                Arc::new(SemanticSearchTool::new(engine)),
                is_chat_mode,
                availability_ctx,
            )
            .await
            .ok();
    }

    registry
        .register(
            "read_terminal",
            Arc::new(ReadTerminalTool::new()),
            is_chat_mode,
            availability_ctx,
        )
        .await
        .ok();

    registry
        .register(
            "syntax_diagnostics",
            Arc::new(SyntaxDiagnosticsTool::new()),
            is_chat_mode,
            availability_ctx,
        )
        .await
        .ok();
}
