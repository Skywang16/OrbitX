-- 初始数据库架构
-- 创建AI模型配置表
CREATE TABLE ai_models (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,  -- 'openai', 'anthropic', 'custom'
    api_url TEXT NOT NULL,
    api_key_encrypted BLOB,  -- 加密存储的API密钥
    model_name TEXT NOT NULL, -- 'gpt-4', 'claude-3-sonnet' 等
    is_default BOOLEAN DEFAULT FALSE,
    enabled BOOLEAN DEFAULT TRUE,
    config_json TEXT,        -- 额外配置参数 (JSON格式)
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    -- 确保只有一个默认模型
    CHECK (is_default IN (0, 1))
);

-- 触发器：确保只有一个默认模型
CREATE TRIGGER ensure_single_default_model
AFTER UPDATE OF is_default ON ai_models
WHEN NEW.is_default = 1
BEGIN
    UPDATE ai_models SET is_default = 0 WHERE id != NEW.id;
END;

-- 命令历史表
CREATE TABLE command_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    command TEXT NOT NULL,
    working_directory TEXT NOT NULL,
    exit_code INTEGER,
    output TEXT,
    duration_ms INTEGER,     -- 命令执行时长（毫秒）
    executed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    session_id TEXT,
    tags TEXT               -- JSON数组，用于命令分类
);

-- 命令历史索引
CREATE INDEX idx_command_history_executed_at ON command_history(executed_at);
CREATE INDEX idx_command_history_command ON command_history(command);
CREATE INDEX idx_command_history_session ON command_history(session_id);
CREATE INDEX idx_command_history_working_dir ON command_history(working_directory);

-- 命令全文搜索表
CREATE VIRTUAL TABLE command_search USING fts5(
    command, output, working_directory, tags,
    content='command_history',
    content_rowid='id'
);

-- 触发器：同步命令历史到全文搜索表
CREATE TRIGGER command_history_ai AFTER INSERT ON command_history BEGIN
    INSERT INTO command_search(rowid, command, output, working_directory, tags)
    VALUES (new.id, new.command, new.output, new.working_directory, new.tags);
END;

CREATE TRIGGER command_history_ad AFTER DELETE ON command_history BEGIN
    INSERT INTO command_search(command_search, rowid, command, output, working_directory, tags)
    VALUES('delete', old.id, old.command, old.output, old.working_directory, old.tags);
END;

CREATE TRIGGER command_history_au AFTER UPDATE ON command_history BEGIN
    INSERT INTO command_search(command_search, rowid, command, output, working_directory, tags)
    VALUES('delete', old.id, old.command, old.output, old.working_directory, old.tags);
    INSERT INTO command_search(rowid, command, output, working_directory, tags)
    VALUES (new.id, new.command, new.output, new.working_directory, new.tags);
END;

-- AI聊天历史表
CREATE TABLE ai_chat_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    token_count INTEGER,     -- token使用统计
    metadata_json TEXT,      -- 额外元数据 (JSON格式)
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (model_id) REFERENCES ai_models(id)
);

-- AI聊天历史索引
CREATE INDEX idx_chat_history_session ON ai_chat_history(session_id);
CREATE INDEX idx_chat_history_created_at ON ai_chat_history(created_at);
CREATE INDEX idx_chat_history_model ON ai_chat_history(model_id);

-- 命令使用统计表
CREATE TABLE command_usage_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    command_hash TEXT NOT NULL,  -- 命令的hash值，用于去重
    command TEXT NOT NULL,
    working_directory TEXT,
    usage_count INTEGER DEFAULT 1,
    last_used DATETIME DEFAULT CURRENT_TIMESTAMP,
    avg_duration_ms INTEGER,     -- 平均执行时长
    UNIQUE(command_hash, working_directory)
);

-- 命令使用统计索引
CREATE INDEX idx_command_usage_stats_hash ON command_usage_stats(command_hash);
CREATE INDEX idx_command_usage_stats_last_used ON command_usage_stats(last_used);
CREATE INDEX idx_command_usage_stats_usage_count ON command_usage_stats(usage_count);

-- AI模型使用统计表
CREATE TABLE ai_model_usage_stats (
    model_id TEXT NOT NULL,
    date DATE NOT NULL,
    request_count INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    avg_response_time_ms INTEGER DEFAULT 0,
    error_count INTEGER DEFAULT 0,
    PRIMARY KEY (model_id, date),
    FOREIGN KEY (model_id) REFERENCES ai_models(id)
);

-- 终端会话表
CREATE TABLE terminal_sessions (
    id TEXT PRIMARY KEY,
    title TEXT,
    working_directory TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_active DATETIME DEFAULT CURRENT_TIMESTAMP,
    command_count INTEGER DEFAULT 0
);

-- 终端会话索引
CREATE INDEX idx_terminal_sessions_last_active ON terminal_sessions(last_active);

-- AI功能配置表
CREATE TABLE ai_features (
    feature_name TEXT PRIMARY KEY,
    enabled BOOLEAN DEFAULT TRUE,
    config_json TEXT,        -- 功能特定配置 (JSON格式)
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 更新时间戳触发器
CREATE TRIGGER update_ai_models_timestamp
AFTER UPDATE ON ai_models
BEGIN
    UPDATE ai_models SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER update_ai_features_timestamp
AFTER UPDATE ON ai_features
BEGIN
    UPDATE ai_features SET updated_at = CURRENT_TIMESTAMP WHERE feature_name = NEW.feature_name;
END;

CREATE TRIGGER update_terminal_sessions_timestamp
AFTER UPDATE ON terminal_sessions
BEGIN
    UPDATE terminal_sessions SET last_active = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- 审计日志表
CREATE TABLE audit_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation TEXT NOT NULL,           -- 'CREATE', 'READ', 'UPDATE', 'DELETE'
    table_name TEXT NOT NULL,          -- 操作的表名
    record_id TEXT,                    -- 操作的记录ID
    user_context TEXT,                 -- 用户上下文信息
    details TEXT,                      -- 操作详情 (JSON格式)
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    success BOOLEAN DEFAULT TRUE,      -- 操作是否成功
    error_message TEXT                 -- 错误信息（如果有）
);

-- 审计日志索引
CREATE INDEX idx_audit_logs_timestamp ON audit_logs(timestamp);
CREATE INDEX idx_audit_logs_table ON audit_logs(table_name);
CREATE INDEX idx_audit_logs_operation ON audit_logs(operation);
CREATE INDEX idx_audit_logs_record_id ON audit_logs(record_id);