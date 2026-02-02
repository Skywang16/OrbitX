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
    
    -- OAuth 支持
    auth_type TEXT NOT NULL DEFAULT 'api_key' CHECK (auth_type IN ('api_key', 'oauth')),
    oauth_provider TEXT CHECK (oauth_provider IN ('openai_codex', 'claude_pro', 'gemini_advanced') OR oauth_provider IS NULL),
    oauth_refresh_token_encrypted TEXT,
    oauth_access_token_encrypted TEXT,
    oauth_token_expires_at INTEGER,
    oauth_metadata TEXT,  -- JSON: {"account_id": "...", "subscription_tier": "..."}
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(provider, model_name)
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
    active_session_id INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_accessed_at INTEGER NOT NULL
);

-- ===========================
-- Agent system
-- ===========================
-- This schema is intentionally NOT backward-compatible.

CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_path TEXT NOT NULL REFERENCES workspaces(path) ON DELETE CASCADE,

    parent_id INTEGER REFERENCES sessions(id) ON DELETE CASCADE,
    agent_type TEXT NOT NULL DEFAULT 'coder',
    spawned_by_tool_call TEXT,

    title TEXT,
    model_id TEXT,
    provider_id TEXT,

    status TEXT NOT NULL DEFAULT 'idle' CHECK (status IN ('idle', 'running', 'completed', 'error', 'cancelled')),
    is_archived INTEGER NOT NULL DEFAULT 0,

    total_messages INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    total_cost REAL NOT NULL DEFAULT 0,

    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_message_at INTEGER
);

CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,

    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    agent_type TEXT NOT NULL DEFAULT 'coder',
    parent_message_id INTEGER REFERENCES messages(id) ON DELETE SET NULL,

    blocks TEXT NOT NULL DEFAULT '[]',

    status TEXT NOT NULL DEFAULT 'completed' CHECK (status IN ('streaming', 'completed', 'error', 'cancelled')),
    is_summary INTEGER NOT NULL DEFAULT 0,
    is_internal INTEGER NOT NULL DEFAULT 0,

    model_id TEXT,
    provider_id TEXT,

    input_tokens INTEGER,
    output_tokens INTEGER,
    cache_read_tokens INTEGER,
    cache_write_tokens INTEGER,

    created_at INTEGER NOT NULL,
    finished_at INTEGER,
    duration_ms INTEGER
);

CREATE TABLE IF NOT EXISTS tool_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id INTEGER NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,

    call_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'completed', 'error', 'cancelled')),

    started_at INTEGER NOT NULL,
    finished_at INTEGER,
    duration_ms INTEGER
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
    session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    message_id INTEGER NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
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

-- ===========================
-- Completion learning model (offline, small footprint)
-- ===========================

CREATE TABLE IF NOT EXISTS completion_command_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT NOT NULL UNIQUE,
    root TEXT NOT NULL,
    sub TEXT,
    use_count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    fail_count INTEGER NOT NULL DEFAULT 0,
    last_used_ts INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS completion_transitions (
    prev_id INTEGER NOT NULL,
    next_id INTEGER NOT NULL,
    count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    fail_count INTEGER NOT NULL DEFAULT 0,
    last_used_ts INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (prev_id, next_id),
    FOREIGN KEY (prev_id) REFERENCES completion_command_keys(id) ON DELETE CASCADE,
    FOREIGN KEY (next_id) REFERENCES completion_command_keys(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS completion_entity_stats (
    entity_type TEXT NOT NULL,
    value TEXT NOT NULL,
    use_count INTEGER NOT NULL DEFAULT 0,
    last_used_ts INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (entity_type, value)
);

-- Legacy triggers removed with legacy tables.
