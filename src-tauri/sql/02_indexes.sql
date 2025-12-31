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

