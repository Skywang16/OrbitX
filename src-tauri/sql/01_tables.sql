-- 数据库表结构定义
-- 创建所有基础表

-- AI模型配置表
CREATE TABLE IF NOT EXISTS ai_models (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    api_url TEXT,
    api_key_encrypted TEXT,
    model_name TEXT NOT NULL,
    model_type TEXT DEFAULT 'chat' CHECK (model_type IN ('chat', 'embedding')),
    config_json TEXT,
    use_custom_base_url INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- AI功能配置表
CREATE TABLE IF NOT EXISTS ai_features (
    feature_name TEXT PRIMARY KEY,
    enabled BOOLEAN DEFAULT TRUE,
    config_json TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 全局偏好设置表
CREATE TABLE IF NOT EXISTS app_preferences (
    key TEXT PRIMARY KEY,
    value TEXT,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 终端会话表
CREATE TABLE IF NOT EXISTS terminal_sessions (
    id TEXT PRIMARY KEY,
    name TEXT,
    working_directory TEXT,
    environment_vars TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_active_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE
);

-- 审计日志表
CREATE TABLE IF NOT EXISTS audit_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation TEXT NOT NULL,
    table_name TEXT NOT NULL,
    record_id TEXT,
    user_context TEXT,
    details TEXT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    success BOOLEAN DEFAULT TRUE,
    error_message TEXT
);

-- AI模型使用统计表
CREATE TABLE IF NOT EXISTS ai_model_usage_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model_id TEXT NOT NULL,
    request_count INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    total_cost REAL DEFAULT 0.0,
    last_used_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (model_id) REFERENCES ai_models(id) ON DELETE CASCADE
);


-- 迁移记录表
CREATE TABLE IF NOT EXISTS schema_migrations (
    version TEXT PRIMARY KEY,
    applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- ===========================
-- Workspace 中心化架构
-- ===========================

CREATE TABLE IF NOT EXISTS workspaces (
    path TEXT PRIMARY KEY,
    display_name TEXT,
    active_session_id INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_accessed_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_path TEXT NOT NULL,
    title TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (workspace_path) REFERENCES workspaces(path) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    status TEXT NOT NULL CHECK (status IN ('streaming', 'completed', 'cancelled', 'error')),
    blocks_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    finished_at INTEGER,
    duration_ms INTEGER,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cache_read_tokens INTEGER,
    cache_write_tokens INTEGER,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS session_summaries (
    session_id INTEGER PRIMARY KEY,
    summary_content TEXT NOT NULL DEFAULT '',
    summary_tokens INTEGER NOT NULL DEFAULT 0,
    messages_summarized INTEGER NOT NULL DEFAULT 0,
    tokens_saved INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS workspace_file_context (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_path TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    record_state TEXT NOT NULL CHECK (record_state IN ('active', 'stale')),
    record_source TEXT NOT NULL CHECK (
        record_source IN ('read_tool', 'user_edited', 'agent_edited', 'file_mentioned')
    ),
    agent_read_at INTEGER,
    agent_edit_at INTEGER,
    user_edit_at INTEGER,
    created_at INTEGER NOT NULL,
    UNIQUE (workspace_path, relative_path),
    FOREIGN KEY (workspace_path) REFERENCES workspaces(path) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS agent_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    execution_id TEXT NOT NULL UNIQUE,
    session_id INTEGER NOT NULL,
    user_request TEXT NOT NULL,
    system_prompt_used TEXT NOT NULL,
    execution_config TEXT,
    has_conversation_context INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'error', 'cancelled')),
    current_iteration INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,
    max_iterations INTEGER NOT NULL DEFAULT 50,
    total_input_tokens INTEGER NOT NULL DEFAULT 0,
    total_output_tokens INTEGER NOT NULL DEFAULT 0,
    total_cost REAL NOT NULL DEFAULT 0.0,
    context_tokens INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    started_at INTEGER,
    completed_at INTEGER,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS execution_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    execution_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('system', 'user', 'assistant', 'tool')),
    content TEXT NOT NULL,
    tokens INTEGER NOT NULL DEFAULT 0,
    is_summary INTEGER NOT NULL DEFAULT 0,
    iteration INTEGER NOT NULL DEFAULT 0,
    sequence INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (execution_id) REFERENCES agent_executions(execution_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS tool_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    execution_id TEXT NOT NULL,
    tool_call_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    tool_arguments TEXT NOT NULL,
    tool_result TEXT,
    error_message TEXT,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'error')),
    files_read TEXT NOT NULL DEFAULT '[]',
    files_written TEXT NOT NULL DEFAULT '[]',
    directories_accessed TEXT NOT NULL DEFAULT '[]',
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    duration_ms INTEGER,
    FOREIGN KEY (execution_id) REFERENCES agent_executions(execution_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS execution_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    execution_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    event_data TEXT NOT NULL,
    iteration INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (execution_id) REFERENCES agent_executions(execution_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS checkpoint_blobs (
    hash TEXT PRIMARY KEY,
    content BLOB NOT NULL,
    size INTEGER NOT NULL,
    ref_count INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS checkpoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_path TEXT NOT NULL,
    session_id INTEGER NOT NULL,
    message_id INTEGER NOT NULL,
    parent_id INTEGER,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (workspace_path) REFERENCES workspaces(path) ON DELETE CASCADE,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES checkpoints(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS checkpoint_file_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    checkpoint_id INTEGER NOT NULL,
    relative_path TEXT NOT NULL,
    blob_hash TEXT NOT NULL,
    change_type TEXT NOT NULL CHECK (change_type IN ('added', 'modified', 'deleted')),
    file_size INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE (checkpoint_id, relative_path),
    FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(id) ON DELETE CASCADE,
    FOREIGN KEY (blob_hash) REFERENCES checkpoint_blobs(hash)
);

CREATE INDEX IF NOT EXISTS idx_workspaces_last_accessed
    ON workspaces(last_accessed_at DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_workspace
    ON sessions(workspace_path, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_session
    ON messages(session_id, created_at ASC);
CREATE INDEX IF NOT EXISTS idx_session_summaries
    ON session_summaries(session_id);
CREATE INDEX IF NOT EXISTS idx_workspace_file_context_state
    ON workspace_file_context(workspace_path, record_state);
CREATE INDEX IF NOT EXISTS idx_agent_executions_session
    ON agent_executions(session_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_executions_status
    ON agent_executions(status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_execution_messages_iter
    ON execution_messages(execution_id, iteration, sequence);
CREATE INDEX IF NOT EXISTS idx_tool_executions_started
    ON tool_executions(execution_id, started_at);
CREATE INDEX IF NOT EXISTS idx_execution_events_iter
    ON execution_events(execution_id, iteration);
CREATE INDEX IF NOT EXISTS idx_checkpoints_workspace
    ON checkpoints(workspace_path, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_checkpoints_session
    ON checkpoints(session_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_checkpoints_message
    ON checkpoints(message_id);
CREATE INDEX IF NOT EXISTS idx_checkpoints_parent
    ON checkpoints(parent_id);
CREATE INDEX IF NOT EXISTS idx_checkpoint_files_checkpoint
    ON checkpoint_file_snapshots(checkpoint_id);
CREATE INDEX IF NOT EXISTS idx_checkpoint_files_blob
    ON checkpoint_file_snapshots(blob_hash);
CREATE INDEX IF NOT EXISTS idx_blob_refcount
    ON checkpoint_blobs(ref_count);

CREATE TRIGGER IF NOT EXISTS trg_session_summaries_updated_at
AFTER UPDATE ON session_summaries
FOR EACH ROW
BEGIN
    UPDATE session_summaries
    SET updated_at = strftime('%s','now')
    WHERE session_id = NEW.session_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_agent_executions_completed_at
AFTER UPDATE OF status ON agent_executions
FOR EACH ROW
WHEN NEW.status IN ('completed', 'error', 'cancelled')
BEGIN
    UPDATE agent_executions
    SET completed_at = COALESCE(NEW.completed_at, strftime('%s','now'))
    WHERE id = NEW.id;
END;
