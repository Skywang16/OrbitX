-- 数据库索引定义
-- 创建所有表的索引以优化查询性能

-- 终端会话索引
CREATE INDEX IF NOT EXISTS idx_terminal_sessions_active ON terminal_sessions(is_active);

-- AI模型索引
-- 唯一索引已在表定义中通过 UNIQUE(provider, model_name) 约束创建
CREATE INDEX IF NOT EXISTS idx_ai_features_enabled ON ai_features(enabled);

-- 审计日志索引
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_logs_operation ON audit_logs(operation);
CREATE INDEX IF NOT EXISTS idx_audit_logs_table_name ON audit_logs(table_name);
CREATE INDEX IF NOT EXISTS idx_audit_logs_success ON audit_logs(success);

-- Agent system (new design) indexes
CREATE INDEX IF NOT EXISTS idx_workspaces_last_accessed
    ON workspaces(last_accessed_at DESC);

CREATE INDEX IF NOT EXISTS idx_run_actions_workspace
    ON run_actions(workspace_path, sort_order);

CREATE INDEX IF NOT EXISTS idx_threads_workspace ON threads(workspace_path);
CREATE INDEX IF NOT EXISTS idx_threads_parent ON threads(parent_thread_id);
CREATE INDEX IF NOT EXISTS idx_threads_status ON threads(workspace_path, status);
CREATE INDEX IF NOT EXISTS idx_threads_updated ON threads(updated_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_threads_archived ON threads(is_archived);
CREATE INDEX IF NOT EXISTS idx_subagents_parent_thread ON subagents(parent_thread_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_subagents_parent_message ON subagents(parent_message_id);
CREATE INDEX IF NOT EXISTS idx_subagents_child_thread ON subagents(child_thread_id);
CREATE INDEX IF NOT EXISTS idx_agent_jobs_thread ON agent_jobs(thread_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_jobs_status ON agent_jobs(status, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_job_items_job_status ON agent_job_items(job_id, status, row_index ASC);
CREATE INDEX IF NOT EXISTS idx_agent_logs_thread_ts ON agent_logs(thread_id, ts DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_thread_memories_updated ON thread_memories(source_updated_at DESC, thread_id DESC);

CREATE INDEX IF NOT EXISTS idx_checkpoints_workspace ON checkpoints(workspace_path, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_checkpoints_thread ON checkpoints(thread_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_checkpoints_message ON checkpoints(message_id);
CREATE INDEX IF NOT EXISTS idx_checkpoints_parent ON checkpoints(parent_id);
CREATE INDEX IF NOT EXISTS idx_checkpoint_files_checkpoint ON checkpoint_file_snapshots(checkpoint_id);
CREATE INDEX IF NOT EXISTS idx_checkpoint_files_blob ON checkpoint_file_snapshots(blob_hash);
CREATE INDEX IF NOT EXISTS idx_blob_refcount ON checkpoint_blobs(ref_count);
