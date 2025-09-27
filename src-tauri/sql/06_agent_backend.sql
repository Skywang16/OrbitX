-- Agent后端迁移 - 新的数据库表结构
-- 完全重构的Agent表结构，支持后端状态持久化和任务恢复

-- ===========================
-- Agent任务主表
-- ===========================
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

-- ===========================
-- Agent执行日志表
-- ===========================
CREATE TABLE IF NOT EXISTS agent_execution_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL,
    iteration INTEGER NOT NULL,
    step_type TEXT NOT NULL CHECK (step_type IN ('thinking', 'tool_call', 'tool_result', 'final_answer', 'error')),
    content_json TEXT NOT NULL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (task_id) REFERENCES agent_tasks(task_id) ON DELETE CASCADE
);

-- ===========================
-- Agent工具调用记录表
-- ===========================
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

-- ===========================
-- Agent上下文快照表
-- ===========================
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
-- 索引优化
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
-- 触发器 - 自动更新updated_at
-- ===========================
CREATE TRIGGER IF NOT EXISTS update_agent_tasks_updated_at
    AFTER UPDATE ON agent_tasks
    FOR EACH ROW
BEGIN
    UPDATE agent_tasks SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- ===========================
-- 清理策略 - 自动清理过期数据
-- ===========================

-- 清理30天前已完成的任务执行日志
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

-- 清理30天前的工具调用记录
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
