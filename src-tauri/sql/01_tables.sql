-- 数据库表结构定义
-- 创建所有基础表

-- AI模型配置表
CREATE TABLE IF NOT EXISTS ai_models (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    api_url TEXT,
    api_key_encrypted TEXT,
    model_name TEXT NOT NULL,
    display_name TEXT,
    model_type TEXT DEFAULT 'chat' CHECK (model_type IN ('chat', 'embedding')),
    config_json TEXT,

    -- OAuth 支持
    auth_type TEXT NOT NULL DEFAULT 'api_key' CHECK (auth_type IN ('api_key', 'oauth')),
    oauth_provider TEXT CHECK (oauth_provider IN ('openai_codex', 'claude_pro', 'gemini_advanced') OR oauth_provider IS NULL),
    oauth_refresh_token_encrypted TEXT,
    oauth_access_token_encrypted TEXT,
    oauth_token_expires_at INTEGER,
    oauth_metadata TEXT,  -- JSON: {"account_id": "...", "subscription_tier": "..."}

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

-- ===========================
-- Workspace 中心化架构
-- ===========================

CREATE TABLE IF NOT EXISTS workspaces (
    path TEXT PRIMARY KEY,
    display_name TEXT,
    active_thread_id INTEGER,
    selected_run_action_id TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_accessed_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS threads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_path TEXT NOT NULL REFERENCES workspaces(path) ON DELETE CASCADE,
    parent_thread_id INTEGER REFERENCES threads(id) ON DELETE CASCADE,
    spawned_by_tool_call_id TEXT,
    title TEXT NOT NULL DEFAULT '',
    display_name TEXT,
    thread_type TEXT NOT NULL DEFAULT 'agent' CHECK (thread_type IN ('agent', 'shell')),
    agent_type TEXT NOT NULL DEFAULT 'coder',
    model_id TEXT,
    provider_id TEXT,
    rollout_path TEXT NOT NULL,
    worktree_path TEXT,
    status TEXT NOT NULL DEFAULT 'idle' CHECK (status IN ('idle', 'running', 'completed', 'error', 'cancelled')),
    is_archived INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_event_at INTEGER,
    first_user_message TEXT
);

CREATE TABLE IF NOT EXISTS subagents (
    id TEXT PRIMARY KEY,
    parent_thread_id INTEGER NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    child_thread_id INTEGER NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    parent_message_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    profile TEXT NOT NULL,
    task_title TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'cancelled', 'error')),
    latest_activity TEXT,
    final_summary TEXT,
    error_message TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    finished_at INTEGER,
    UNIQUE(parent_thread_id, name)
);

CREATE TABLE IF NOT EXISTS thread_dynamic_tools (
    thread_id INTEGER NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    input_schema_json TEXT NOT NULL,
    PRIMARY KEY (thread_id, position)
);

CREATE TABLE IF NOT EXISTS thread_memories (
    thread_id INTEGER PRIMARY KEY REFERENCES threads(id) ON DELETE CASCADE,
    source_updated_at INTEGER NOT NULL,
    raw_memory TEXT NOT NULL,
    rollout_summary TEXT NOT NULL,
    generated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_jobs (
    id TEXT PRIMARY KEY,
    thread_id INTEGER REFERENCES threads(id) ON DELETE SET NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    result_json TEXT,
    last_error TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    started_at INTEGER,
    finished_at INTEGER
);

CREATE TABLE IF NOT EXISTS agent_job_items (
    job_id TEXT NOT NULL REFERENCES agent_jobs(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL,
    row_index INTEGER NOT NULL,
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL,
    assigned_thread_id INTEGER REFERENCES threads(id) ON DELETE SET NULL,
    result_json TEXT,
    last_error TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    finished_at INTEGER,
    PRIMARY KEY (job_id, item_id)
);

CREATE TABLE IF NOT EXISTS agent_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    thread_id INTEGER REFERENCES threads(id) ON DELETE CASCADE,
    ts INTEGER NOT NULL,
    level TEXT NOT NULL,
    target TEXT NOT NULL,
    message TEXT,
    fields_json TEXT
);

CREATE TABLE IF NOT EXISTS run_actions (
    id TEXT PRIMARY KEY,
    workspace_path TEXT NOT NULL REFERENCES workspaces(path) ON DELETE CASCADE,
    name TEXT NOT NULL,
    command TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
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
    workspace_path TEXT NOT NULL REFERENCES workspaces(path) ON DELETE CASCADE,
    thread_id INTEGER NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    message_id INTEGER NOT NULL,
    parent_id INTEGER REFERENCES checkpoints(id) ON DELETE SET NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS checkpoint_file_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    checkpoint_id INTEGER NOT NULL REFERENCES checkpoints(id) ON DELETE CASCADE,
    relative_path TEXT NOT NULL,
    blob_hash TEXT NOT NULL REFERENCES checkpoint_blobs(hash),
    change_type TEXT NOT NULL CHECK (change_type IN ('added', 'modified', 'deleted')),
    file_size INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE (checkpoint_id, relative_path)
);


-- Legacy triggers removed with legacy tables.
