-- Task Tree persistence schema (pure SQL)

-- Task index (basic searchable info + metadata JSON)
CREATE TABLE IF NOT EXISTS task_index (
    task_id TEXT PRIMARY KEY,
    name TEXT,
    status TEXT CHECK (status IN ('init','running','done','error')),
    parent_task_id TEXT,
    root_task_id TEXT,
    metadata_json TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_task_index_parent ON task_index(parent_task_id);
CREATE INDEX IF NOT EXISTS idx_task_index_root ON task_index(root_task_id);
CREATE INDEX IF NOT EXISTS idx_task_index_status ON task_index(status);
CREATE INDEX IF NOT EXISTS idx_task_index_updated ON task_index(updated_at);

-- UI events stream (append-only logical stream, ordered by id)
CREATE TABLE IF NOT EXISTS task_ui_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL,
    event_json TEXT NOT NULL,
    ts_ms INTEGER DEFAULT (strftime('%s','now')*1000),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_task_ui_events_task ON task_ui_events(task_id, id);

-- API messages (consolidated per task)
CREATE TABLE IF NOT EXISTS task_api_messages (
    task_id TEXT PRIMARY KEY,
    messages_json TEXT,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Checkpoints (named snapshots per task)
CREATE TABLE IF NOT EXISTS task_checkpoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL,
    name TEXT NOT NULL,
    checkpoint_json TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(task_id, name)
);

CREATE INDEX IF NOT EXISTS idx_task_checkpoints_task ON task_checkpoints(task_id, created_at);
