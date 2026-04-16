#[cfg(test)]
mod web_fetch_tests {
    use serde_json::json;
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;
    use terminal_lib::agent::config::AgentRunConfig;
    use terminal_lib::agent::core::context::{
        AgentRunContext, AgentRunContextDeps, AgentRunContextInit, SubAgentRequest,
        SubAgentResponse, SubAgentRunner,
    };
    use terminal_lib::agent::error::{AgentRunError, AgentRunResult};
    use terminal_lib::agent::persistence::AgentPersistence;
    use terminal_lib::agent::tools::builtin::WebFetchTool;
    use terminal_lib::agent::tools::{RunnableTool, ToolRegistry};
    use terminal_lib::agent::workspace_changes::WorkspaceChangeJournal;
    use terminal_lib::settings::SettingsManager;
    use terminal_lib::storage::{DatabaseManager, DatabaseOptions, StoragePathsBuilder};

    struct NoopSubAgentRunner;

    #[async_trait::async_trait]
    impl SubAgentRunner for NoopSubAgentRunner {
        async fn run_subagent(
            &self,
            _parent: &AgentRunContext,
            _request: SubAgentRequest,
        ) -> AgentRunResult<SubAgentResponse> {
            Err(AgentRunError::InternalError(
                "NoopSubAgentRunner does not execute child agents".to_string(),
            ))
        }

        async fn spawn_subagent(
            &self,
            _parent: &AgentRunContext,
            _request: SubAgentRequest,
            _collab_call_id: String,
            _collab_tool: String,
        ) -> AgentRunResult<i64> {
            Err(AgentRunError::InternalError(
                "NoopSubAgentRunner does not spawn child agents".to_string(),
            ))
        }

        async fn cancel_subagent(
            &self,
            _parent: &AgentRunContext,
            _thread_id: i64,
        ) -> AgentRunResult<()> {
            Err(AgentRunError::InternalError(
                "NoopSubAgentRunner does not cancel child agents".to_string(),
            ))
        }
    }

    async fn create_test_task_context(root: &Path) -> AgentRunContext {
        let storage_root = root.join("storage");
        std::fs::create_dir_all(&storage_root).expect("failed to create test storage root");

        let paths = StoragePathsBuilder::new()
            .app_dir(storage_root)
            .build()
            .expect("failed to build storage paths");
        paths
            .ensure_directories()
            .expect("failed to create storage directories");

        let options = DatabaseOptions {
            encryption: false,
            ..DatabaseOptions::default()
        };

        let database = Arc::new(
            DatabaseManager::new(paths, options)
                .await
                .expect("failed to create test database"),
        );
        database
            .initialize()
            .await
            .expect("failed to initialize test database");

        let cwd = std::env::current_dir()
            .expect("failed to resolve current dir")
            .to_string_lossy()
            .to_string();

        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO workspaces (path, display_name, active_thread_id, selected_run_action_id, created_at, updated_at, last_accessed_at)
             VALUES (?, ?, NULL, NULL, ?, ?, ?)",
        )
        .bind(&cwd)
        .bind("Web Fetch Test")
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(database.pool())
        .await
        .expect("failed to create test workspace");

        let persistence = Arc::new(AgentPersistence::new(Arc::clone(&database)));
        let thread = persistence
            .threads()
            .create(
                terminal_lib::agent::rollout::projection::CreateThreadParams {
                    workspace_path: &cwd,
                    title: "Web Fetch Test",
                    display_name: None,
                    agent_type: "chat",
                    parent_thread_id: None,
                    spawned_by_tool_call_id: None,
                    rollout_path: "",
                    worktree_path: None,
                    model_id: None,
                    provider_id: None,
                },
            )
            .await
            .expect("failed to create test thread");

        AgentRunContext::new(AgentRunContextInit {
            run_id: "web-fetch-test".to_string(),
            thread_id: thread.id,
            user_prompt: "test prompt".to_string(),
            agent_type: "chat".to_string(),
            config: AgentRunConfig::default(),
            workspace_path: cwd,
            emit_task_events: false,
            progress_channel: None,
            deps: AgentRunContextDeps {
                tool_registry: Arc::new(ToolRegistry::default()),
                repositories: Arc::clone(&database),
                agent_persistence: persistence,
                checkpoint_service: None,
                workspace_changes: Arc::new(WorkspaceChangeJournal::new()),
                subagent_runner: Arc::new(NoopSubAgentRunner),
                settings_manager: Arc::new(
                    SettingsManager::new().expect("failed to create test settings manager"),
                ),
            },
        })
        .await
        .expect("failed to create test run context")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_web_fetch_timeout() {
        let tool = WebFetchTool::new();
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let context = create_test_task_context(temp_dir.path()).await;

        println!("Starting WebFetch test...");
        let start = std::time::Instant::now();

        let result = tokio::time::timeout(
            Duration::from_secs(10),
            tool.run(&context, json!({"url": "http://127.0.0.1"})),
        )
        .await;

        println!("Test completed in {:?}", start.elapsed());

        match result {
            Ok(Ok(tool_result)) => {
                println!("Success: {:?}", tool_result.status);
            }
            Ok(Err(e)) => {
                println!("Tool error: {e:?}");
            }
            Err(_) => {
                panic!("WebFetch timed out after 10 seconds!");
            }
        }
    }
}
