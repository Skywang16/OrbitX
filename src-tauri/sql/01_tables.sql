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
-- Agent 会话上下文架构
-- ===========================
-- 注意：自动清库逻辑已移除，避免正常启动时清空会话数据。
-- 如需在迁移时清理旧表，请改用专用手动脚本。

-- 会话表：顶层会话容器
CREATE TABLE IF NOT EXISTS conversations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT,
    workspace_path TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 会话摘要表：智能压缩信息
CREATE TABLE IF NOT EXISTS conversation_summaries (
    conversation_id INTEGER PRIMARY KEY,
    summary_content TEXT NOT NULL DEFAULT '',
    summary_tokens INTEGER NOT NULL DEFAULT 0,
    messages_before_summary INTEGER NOT NULL DEFAULT 0,
    tokens_saved INTEGER NOT NULL DEFAULT 0,
    compression_cost REAL NOT NULL DEFAULT 0.0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);

-- 文件上下文表：跟踪文件状态
CREATE TABLE IF NOT EXISTS file_context_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    record_state TEXT NOT NULL CHECK (record_state IN ('active', 'stale')),
    record_source TEXT NOT NULL CHECK (
        record_source IN ('read_tool', 'user_edited', 'agent_edited', 'file_mentioned')
    ),
    agent_read_timestamp INTEGER,
    agent_edit_timestamp INTEGER,
    user_edit_timestamp INTEGER,
    created_at INTEGER NOT NULL,
    UNIQUE (conversation_id, file_path),
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);

-- 执行记录表：单次Agent执行元数据
CREATE TABLE IF NOT EXISTS agent_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    execution_id TEXT NOT NULL UNIQUE,
    conversation_id INTEGER NOT NULL,
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
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);

-- 执行消息表：ReAct循环消息
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

-- 工具执行表：工具调用持久化
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

-- 执行事件表：调试与审计
CREATE TABLE IF NOT EXISTS execution_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    execution_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    event_data TEXT NOT NULL,
    iteration INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (execution_id) REFERENCES agent_executions(execution_id) ON DELETE CASCADE
);

-- Agent UI 缓存表
DROP TABLE IF EXISTS agent_ui_events;
DROP TABLE IF EXISTS agent_ui_tasks;
DROP TABLE IF EXISTS agent_ui_context_snapshots;

CREATE TABLE IF NOT EXISTS agent_ui_conversations (
    id INTEGER PRIMARY KEY,
    title TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (id) REFERENCES conversations(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS agent_ui_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    content TEXT,
    steps_json TEXT,
    status TEXT CHECK (status IN ('streaming', 'complete', 'error')),
    duration_ms INTEGER,
    created_at INTEGER NOT NULL,
    images_json TEXT,
    FOREIGN KEY (conversation_id) REFERENCES agent_ui_conversations(id) ON DELETE CASCADE
);

-- Agent 相关索引
CREATE INDEX IF NOT EXISTS idx_conversations_updated ON conversations(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_file_context_conversation_state
    ON file_context_entries(conversation_id, record_state);
CREATE INDEX IF NOT EXISTS idx_file_context_path_state
    ON file_context_entries(file_path, record_state);
CREATE INDEX IF NOT EXISTS idx_executions_conversation_created
    ON agent_executions(conversation_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_executions_status_created
    ON agent_executions(status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_execution_iter_seq
    ON execution_messages(execution_id, iteration, sequence);
CREATE INDEX IF NOT EXISTS idx_messages_summary
    ON execution_messages(execution_id, is_summary);
CREATE INDEX IF NOT EXISTS idx_tools_execution_started
    ON tool_executions(execution_id, started_at);
CREATE INDEX IF NOT EXISTS idx_tools_name_status
    ON tool_executions(tool_name, status);
CREATE INDEX IF NOT EXISTS idx_events_execution_iter
    ON execution_events(execution_id, iteration);
CREATE INDEX IF NOT EXISTS idx_events_type_created
    ON execution_events(event_type, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_ui_messages_conversation
    ON agent_ui_messages(conversation_id, created_at ASC);

-- Agent 相关触发器
CREATE TRIGGER IF NOT EXISTS trg_conversation_summaries_updated_at
AFTER UPDATE ON conversation_summaries
FOR EACH ROW
BEGIN
    UPDATE conversation_summaries
    SET updated_at = strftime('%s','now')
    WHERE conversation_id = NEW.conversation_id;
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

-- 最近打开的工作区表
CREATE TABLE IF NOT EXISTS recent_workspaces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    last_accessed_at INTEGER NOT NULL
);

-- ===========================
-- Checkpoint 系统架构
-- ===========================
-- 用于追踪 Agent 对文件的修改历史，支持回滚到任意历史状态

-- Blob 存储表：内容寻址存储
CREATE TABLE IF NOT EXISTS checkpoint_blobs (
    hash TEXT PRIMARY KEY,
    content BLOB NOT NULL,
    size INTEGER NOT NULL,
    ref_count INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL
);

-- Checkpoint 主表：树状结构
CREATE TABLE IF NOT EXISTS checkpoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL,
    parent_id INTEGER,
    user_message TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES checkpoints(id) ON DELETE SET NULL
);

-- 文件快照表：记录每个 checkpoint 包含哪些文件
CREATE TABLE IF NOT EXISTS checkpoint_file_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    checkpoint_id INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    blob_hash TEXT NOT NULL,
    change_type TEXT NOT NULL CHECK (change_type IN ('added', 'modified', 'deleted')),
    file_size INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE (checkpoint_id, file_path),
    FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(id) ON DELETE CASCADE,
    FOREIGN KEY (blob_hash) REFERENCES checkpoint_blobs(hash)
);

-- Checkpoint 相关索引
CREATE INDEX IF NOT EXISTS idx_checkpoints_conversation
    ON checkpoints(conversation_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_checkpoints_parent
    ON checkpoints(parent_id);
CREATE INDEX IF NOT EXISTS idx_file_snapshots_checkpoint
    ON checkpoint_file_snapshots(checkpoint_id);
CREATE INDEX IF NOT EXISTS idx_file_snapshots_blob
    ON checkpoint_file_snapshots(blob_hash);
CREATE INDEX IF NOT EXISTS idx_blobs_ref_count
    ON checkpoint_blobs(ref_count);
