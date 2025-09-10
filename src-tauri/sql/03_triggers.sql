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
        -- 如果是第一条用户消息且标题为空，则自动更新标题
        title = CASE
            WHEN NEW.role = 'user' AND (title IS NULL OR title = '') THEN
                CASE
                    WHEN LENGTH(NEW.content) > 20 THEN SUBSTR(NEW.content, 1, 20) || '...'
                    ELSE NEW.content
                END
            ELSE title
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

-- 向量索引工作区自动更新时间戳
CREATE TRIGGER IF NOT EXISTS trigger_vector_workspaces_updated_at
AFTER UPDATE ON vector_workspaces
FOR EACH ROW
BEGIN
    UPDATE vector_workspaces
    SET updated_at = CURRENT_TIMESTAMP
    WHERE id = NEW.id;
END;


