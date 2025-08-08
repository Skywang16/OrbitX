-- 数据库触发器定义
-- 创建自动更新时间戳和维护数据一致性的触发器

-- 自动更新会话时间戳
CREATE TRIGGER IF NOT EXISTS update_conversations_timestamp
AFTER UPDATE ON ai_conversations
BEGIN
    UPDATE ai_conversations SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- 插入消息时更新会话统计
CREATE TRIGGER IF NOT EXISTS update_message_count_insert
AFTER INSERT ON ai_messages
BEGIN
    UPDATE ai_conversations
    SET message_count = message_count + 1,
        updated_at = CURRENT_TIMESTAMP,
        last_message_preview = CASE
            WHEN LENGTH(NEW.content) > 40 THEN SUBSTR(NEW.content, 1, 40) || '...'
            ELSE NEW.content
        END
    WHERE id = NEW.conversation_id;
END;

-- 删除消息时更新会话统计
CREATE TRIGGER IF NOT EXISTS update_message_count_delete
AFTER DELETE ON ai_messages
BEGIN
    UPDATE ai_conversations
    SET message_count = message_count - 1,
        updated_at = CURRENT_TIMESTAMP
    WHERE id = OLD.conversation_id;
END;

-- 更新消息时刷新会话预览
CREATE TRIGGER IF NOT EXISTS update_message_preview
AFTER UPDATE ON ai_messages
BEGIN
    UPDATE ai_conversations
    SET updated_at = CURRENT_TIMESTAMP,
        last_message_preview = CASE
            WHEN LENGTH(NEW.content) > 40 THEN SUBSTR(NEW.content, 1, 40) || '...'
            ELSE NEW.content
        END
    WHERE id = NEW.conversation_id;
END;
