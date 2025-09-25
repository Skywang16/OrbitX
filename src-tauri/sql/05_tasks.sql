-- OrbitX 任务系统重构级架构（Final）- 双轨制（Breaking）
-- 删除/弃用：原有 task_index、task_ui_events、task_api_messages、task_checkpoints 表

-- 删除旧表
DROP TABLE IF EXISTS task_index;
DROP TABLE IF EXISTS task_ui_events;
DROP TABLE IF EXISTS task_api_messages;
DROP TABLE IF EXISTS task_checkpoints;

-- UI 渲染轨（最小字段集）
CREATE TABLE IF NOT EXISTS ui_tasks (
  ui_id INTEGER PRIMARY KEY AUTOINCREMENT,
  conversation_id INTEGER NOT NULL,
  task_id TEXT NOT NULL,
  name TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('init','active','paused','completed','error')),
  parent_ui_id INTEGER,
  render_json TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (conversation_id) REFERENCES ai_conversations(id) ON DELETE CASCADE
);

-- 原始上下文轨（单表：状态/事件/快照，三合一）
CREATE TABLE IF NOT EXISTS eko_context (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id TEXT NOT NULL,
  conversation_id INTEGER NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('state','event','snapshot')),
  name TEXT,
  node_id TEXT,
  status TEXT CHECK (status IN ('init','running','paused','aborted','done','error')),
  payload_json TEXT NOT NULL,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (conversation_id) REFERENCES ai_conversations(id) ON DELETE CASCADE
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_ui_tasks_conv ON ui_tasks(conversation_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_ui_tasks_parent ON ui_tasks(parent_ui_id);
CREATE UNIQUE INDEX IF NOT EXISTS uniq_ui_tasks_conv_task ON ui_tasks(conversation_id, task_id);

CREATE INDEX IF NOT EXISTS idx_eko_ctx_conv_task ON eko_context(conversation_id, task_id, id);
CREATE INDEX IF NOT EXISTS idx_eko_ctx_task ON eko_context(task_id, id);
CREATE INDEX IF NOT EXISTS idx_eko_ctx_kind ON eko_context(kind);