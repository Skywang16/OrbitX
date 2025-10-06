use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use tempfile::TempDir;

use crate::agent::context::{FileContextTracker, FileOperationRecord};
use crate::agent::persistence::{AgentPersistence, FileRecordSource, FileRecordState};
use crate::agent::ui::AgentUiPersistence;
use crate::storage::database::PoolSize;
use crate::storage::{DatabaseManager, DatabaseOptions, RepositoryManager, StoragePathsBuilder};
use crate::utils::error::AppResult;
struct TestHarness {
    #[allow(dead_code)]
    temp_dir: TempDir,
    #[allow(dead_code)]
    database: Arc<DatabaseManager>,
    persistence: Arc<AgentPersistence>,
    ui_persistence: Arc<AgentUiPersistence>,
    repositories: Arc<RepositoryManager>,
}

impl TestHarness {
    async fn new() -> AppResult<Self> {
        let temp_dir = TempDir::new().expect("temp dir");
        let app_dir = temp_dir.path().to_path_buf();
        let paths = StoragePathsBuilder::new()
            .app_dir(app_dir.clone())
            .config_dir(app_dir.join("config"))
            .state_dir(app_dir.join("state"))
            .data_dir(app_dir.join("data"))
            .build()?;

        let options = DatabaseOptions {
            encryption: false,
            pool_size: PoolSize::Fixed(NonZeroU32::new(4).unwrap()),
            connection_timeout: Duration::from_secs(5),
            statement_timeout: Duration::from_secs(5),
            wal: false,
            sql_dir: None,
        };

        let database = Arc::new(DatabaseManager::new(paths, options).await?);
        database.initialize().await?;

        let persistence = Arc::new(AgentPersistence::new(Arc::clone(&database)));
        let ui_persistence = Arc::new(AgentUiPersistence::new(Arc::clone(&database)));
        let repositories = Arc::new(RepositoryManager::new(Arc::clone(&database)));

        Ok(Self {
            temp_dir,
            database,
            persistence,
            ui_persistence,
            repositories,
        })
    }
}

#[tokio::test]
async fn conversation_persistence_roundtrip() -> AppResult<()> {
    let harness = TestHarness::new().await?;
    let conversations = harness.persistence.conversations();

    let record = conversations
        .create(Some("Test Session"), Some("/tmp/workspace"))
        .await?;
    assert_eq!(record.title.as_deref(), Some("Test Session"));

    let summary_repo = harness.persistence.conversation_summaries();
    let summary = summary_repo
        .upsert(record.id, "Summary text", 128, 10, 64, 0.002)
        .await?;
    assert_eq!(summary.conversation_id, record.id);
    assert_eq!(summary.summary_tokens, 128);

    let fetched = summary_repo.get(record.id).await?.expect("summary");
    assert_eq!(fetched.tokens_saved, 64);

    Ok(())
}

#[tokio::test]
async fn file_context_tracker_transitions() -> AppResult<()> {
    let harness = TestHarness::new().await?;
    let conversations = harness.persistence.conversations();
    let convo = conversations.create(None, None).await?;

    let tracker = FileContextTracker::new(harness.persistence.clone(), convo.id);
    let path = std::path::Path::new("src/lib.rs");

    tracker
        .track_file_operation(FileOperationRecord::new(path, FileRecordSource::ReadTool))
        .await?;
    let active = tracker.get_active_files().await?;
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].record_state, FileRecordState::Active);

    tracker
        .track_file_operation(FileOperationRecord::new(path, FileRecordSource::UserEdited))
        .await?;
    let stale = tracker.get_stale_files().await?;
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].record_state, FileRecordState::Stale);

    tracker.mark_file_as_stale(path).await?;
    let stale_again = tracker.get_stale_files().await?;
    assert_eq!(stale_again.len(), 1);

    Ok(())
}
