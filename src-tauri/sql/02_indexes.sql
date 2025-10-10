-- 数据库索引定义
-- 创建所有表的索引以优化查询性能

-- 命令历史索引
CREATE INDEX IF NOT EXISTS idx_command_history_executed_at ON command_history(executed_at);
CREATE INDEX IF NOT EXISTS idx_command_history_command ON command_history(command);
CREATE INDEX IF NOT EXISTS idx_command_history_session ON command_history(session_id);
CREATE INDEX IF NOT EXISTS idx_command_history_working_dir ON command_history(working_directory);
CREATE INDEX IF NOT EXISTS idx_command_history_exit_code ON command_history(exit_code);

-- 终端会话索引
CREATE INDEX IF NOT EXISTS idx_terminal_sessions_active ON terminal_sessions(is_active);



-- AI模型索引
CREATE INDEX IF NOT EXISTS idx_ai_models_enabled ON ai_models(enabled);
CREATE INDEX IF NOT EXISTS idx_ai_models_provider ON ai_models(provider);
CREATE INDEX IF NOT EXISTS idx_ai_features_enabled ON ai_features(enabled);

-- 命令使用统计索引
CREATE INDEX IF NOT EXISTS idx_command_usage_stats_hash_dir ON command_usage_stats(command_hash, working_directory);
CREATE INDEX IF NOT EXISTS idx_command_usage_stats_last_used ON command_usage_stats(last_used);
CREATE INDEX IF NOT EXISTS idx_command_usage_stats_usage_count ON command_usage_stats(usage_count);

-- 审计日志索引
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_logs_operation ON audit_logs(operation);
CREATE INDEX IF NOT EXISTS idx_audit_logs_table_name ON audit_logs(table_name);
CREATE INDEX IF NOT EXISTS idx_audit_logs_success ON audit_logs(success);

-- 最近工作区索引
CREATE INDEX IF NOT EXISTS idx_recent_workspaces_last_accessed ON recent_workspaces(last_accessed_at DESC);

