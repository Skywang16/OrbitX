-- 全文搜索表和触发器定义
-- 创建FTS5全文搜索表和相关触发器

-- 命令历史全文搜索表
CREATE VIRTUAL TABLE IF NOT EXISTS command_search USING fts5(
    command,
    output,
    content='command_history',
    content_rowid='id'
);

-- 插入命令历史时同步到FTS表
CREATE TRIGGER IF NOT EXISTS command_search_insert
AFTER INSERT ON command_history
BEGIN
    INSERT INTO command_search(rowid, command, output)
    VALUES (NEW.id, NEW.command, COALESCE(NEW.output, ''));
END;

-- 删除命令历史时同步删除FTS记录
CREATE TRIGGER IF NOT EXISTS command_search_delete
AFTER DELETE ON command_history
BEGIN
    DELETE FROM command_search WHERE rowid = OLD.id;
END;

-- 更新命令历史时同步更新FTS记录
CREATE TRIGGER IF NOT EXISTS command_search_update
AFTER UPDATE ON command_history
BEGIN
    UPDATE command_search
    SET command = NEW.command, output = COALESCE(NEW.output, '')
    WHERE rowid = NEW.id;
END;
