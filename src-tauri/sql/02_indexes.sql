-- 数据库索引定义
-- 创建所有表的索引以优化查询性能

-- 终端会话索引
CREATE INDEX IF NOT EXISTS idx_terminal_sessions_active ON terminal_sessions(is_active);

-- AI模型索引
CREATE INDEX IF NOT EXISTS idx_ai_models_provider ON ai_models(provider);
CREATE INDEX IF NOT EXISTS idx_ai_features_enabled ON ai_features(enabled);

-- 审计日志索引
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_logs_operation ON audit_logs(operation);
CREATE INDEX IF NOT EXISTS idx_audit_logs_table_name ON audit_logs(table_name);
CREATE INDEX IF NOT EXISTS idx_audit_logs_success ON audit_logs(success);

-- Completion learning model indexes
CREATE INDEX IF NOT EXISTS idx_completion_command_keys_last_used
    ON completion_command_keys(last_used_ts);
CREATE INDEX IF NOT EXISTS idx_completion_command_keys_root
    ON completion_command_keys(root);
CREATE INDEX IF NOT EXISTS idx_completion_transitions_prev_last_used
    ON completion_transitions(prev_id, last_used_ts DESC);
CREATE INDEX IF NOT EXISTS idx_completion_transitions_last_used
    ON completion_transitions(last_used_ts DESC);
CREATE INDEX IF NOT EXISTS idx_completion_entities_type_last_used
    ON completion_entity_stats(entity_type, last_used_ts DESC);

-- Agent system (new design) indexes
CREATE INDEX IF NOT EXISTS idx_sessions_workspace ON sessions(workspace_path);
CREATE INDEX IF NOT EXISTS idx_sessions_parent ON sessions(parent_id);
CREATE INDEX IF NOT EXISTS idx_sessions_agent ON sessions(agent_type);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(workspace_path, status);

CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_session_role ON messages(session_id, role);
CREATE INDEX IF NOT EXISTS idx_messages_parent ON messages(parent_message_id);

CREATE INDEX IF NOT EXISTS idx_tool_executions_message ON tool_executions(message_id);
CREATE INDEX IF NOT EXISTS idx_tool_executions_session ON tool_executions(session_id);
CREATE INDEX IF NOT EXISTS idx_tool_executions_tool ON tool_executions(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_executions_status ON tool_executions(status);
