-- Agent后端迁移脚本
-- 删除旧的任务表结构，应用新的Agent后端表结构
-- 执行顺序：先删除旧表，再创建新表

-- ===========================
-- 步骤1：删除旧的任务相关表
-- ===========================

-- 删除旧的Agent/Task相关表（如果存在）
DROP TABLE IF EXISTS task_index;
DROP TABLE IF EXISTS task_ui_events;
DROP TABLE IF EXISTS task_api_messages;  
DROP TABLE IF EXISTS task_checkpoints;

-- 删除当前的任务系统表（05_tasks.sql中定义）
DROP TABLE IF EXISTS ui_tasks;
DROP TABLE IF EXISTS eko_context;

-- 删除相关的索引（如果存在）
DROP INDEX IF EXISTS idx_ui_tasks_conv;
DROP INDEX IF EXISTS idx_ui_tasks_parent;
DROP INDEX IF EXISTS uniq_ui_tasks_conv_task;
DROP INDEX IF EXISTS idx_eko_ctx_conv_task;
DROP INDEX IF EXISTS idx_eko_ctx_task;
DROP INDEX IF EXISTS idx_eko_ctx_kind;

-- 删除相关的触发器（如果存在）
DROP TRIGGER IF EXISTS update_ui_tasks_updated_at;
DROP TRIGGER IF EXISTS update_eko_context_updated_at;

-- ===========================
-- 步骤2：应用新的Agent表结构
-- ===========================

-- agent_tasks - 任务主表
CREATE TABLE IF NOT EXISTS agent_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL UNIQUE,
    conversation_id INTEGER NOT NULL,
    user_prompt TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('created', 'running', 'paused', 'completed', 'error', 'cancelled')),
    current_iteration INTEGER DEFAULT 0,
    max_iterations INTEGER DEFAULT 100,
    error_count INTEGER DEFAULT 0,
    config_json TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    FOREIGN KEY (conversation_id) REFERENCES ai_conversations(id) ON DELETE CASCADE
);

-- agent_execution_log - 执行日志表
CREATE TABLE IF NOT EXISTS agent_execution_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL,
    iteration INTEGER NOT NULL,
    step_type TEXT NOT NULL CHECK (step_type IN ('thinking', 'tool_call', 'tool_result', 'final_answer', 'error')),
    content_json TEXT NOT NULL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (task_id) REFERENCES agent_tasks(task_id) ON DELETE CASCADE
);

-- agent_tool_calls - 工具调用记录表
CREATE TABLE IF NOT EXISTS agent_tool_calls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL,
    call_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL,
    result_json TEXT,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'error')),
    error_message TEXT,
    started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    FOREIGN KEY (task_id) REFERENCES agent_tasks(task_id) ON DELETE CASCADE
);

-- agent_context_snapshots - 上下文快照表
CREATE TABLE IF NOT EXISTS agent_context_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL,
    iteration INTEGER NOT NULL,
    context_type TEXT NOT NULL CHECK (context_type IN ('full', 'incremental')),
    messages_json TEXT NOT NULL,
    additional_state_json TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (task_id) REFERENCES agent_tasks(task_id) ON DELETE CASCADE
);

-- ===========================
-- 步骤3：创建优化索引
-- ===========================

-- agent_tasks索引
CREATE INDEX IF NOT EXISTS idx_agent_tasks_conversation ON agent_tasks(conversation_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_tasks_status ON agent_tasks(status, updated_at DESC);
CREATE UNIQUE INDEX IF NOT EXISTS uniq_agent_tasks_task_id ON agent_tasks(task_id);

-- agent_execution_log索引
CREATE INDEX IF NOT EXISTS idx_agent_execution_task_iter ON agent_execution_log(task_id, iteration ASC);
CREATE INDEX IF NOT EXISTS idx_agent_execution_step_type ON agent_execution_log(step_type, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_agent_execution_timestamp ON agent_execution_log(timestamp DESC);

-- agent_tool_calls索引
CREATE INDEX IF NOT EXISTS idx_agent_tool_calls_task ON agent_tool_calls(task_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_tool_calls_status ON agent_tool_calls(status, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_tool_calls_call_id ON agent_tool_calls(call_id);

-- agent_context_snapshots索引
CREATE INDEX IF NOT EXISTS idx_agent_context_task_iter ON agent_context_snapshots(task_id, iteration DESC);
CREATE INDEX IF NOT EXISTS idx_agent_context_type ON agent_context_snapshots(context_type, created_at DESC);

-- ===========================
-- 步骤4：创建触发器
-- ===========================

-- 自动更新updated_at字段
CREATE TRIGGER IF NOT EXISTS update_agent_tasks_updated_at
    AFTER UPDATE ON agent_tasks
    FOR EACH ROW
BEGIN
    UPDATE agent_tasks SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- 自动清理过期的执行日志
CREATE TRIGGER IF NOT EXISTS cleanup_old_execution_logs
    AFTER INSERT ON agent_execution_log
    FOR EACH ROW
    WHEN (SELECT COUNT(*) FROM agent_execution_log) > 10000
BEGIN
    DELETE FROM agent_execution_log 
    WHERE timestamp < datetime('now', '-30 days')
    AND task_id IN (
        SELECT task_id FROM agent_tasks 
        WHERE status IN ('completed', 'error', 'cancelled')
        AND completed_at < datetime('now', '-30 days')
    );
END;

-- 自动清理过期的工具调用记录
CREATE TRIGGER IF NOT EXISTS cleanup_old_tool_calls
    AFTER INSERT ON agent_tool_calls
    FOR EACH ROW
    WHEN (SELECT COUNT(*) FROM agent_tool_calls) > 5000
BEGIN
    DELETE FROM agent_tool_calls 
    WHERE started_at < datetime('now', '-30 days')
    AND task_id IN (
        SELECT task_id FROM agent_tasks 
        WHERE status IN ('completed', 'error', 'cancelled')
        AND completed_at < datetime('now', '-30 days')
    );
END;

-- 保留最新5个上下文快照，删除旧的
CREATE TRIGGER IF NOT EXISTS cleanup_old_context_snapshots
    AFTER INSERT ON agent_context_snapshots
    FOR EACH ROW
BEGIN
    DELETE FROM agent_context_snapshots 
    WHERE task_id = NEW.task_id 
    AND id NOT IN (
        SELECT id FROM agent_context_snapshots 
        WHERE task_id = NEW.task_id 
        ORDER BY iteration DESC, id DESC 
        LIMIT 5
    );
END;

-- ===========================
-- 步骤5：记录迁移版本
-- ===========================
INSERT OR REPLACE INTO schema_migrations (version) VALUES ('agent_backend_migration_v1.0.0');
