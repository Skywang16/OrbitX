/**
 * AI会话上下文管理API - 全新重构版本
 *
 * 基于新的双表架构和上下文管理系统的API接口
 */

import type { Conversation, Message } from '@/types/features/ai/chat'
import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'

// 内部类型定义，用于后端数据转换
interface RawConversation {
  id: number
  title: string
  messageCount: number
  lastMessagePreview?: string
  createdAt: string
  updatedAt: string
}

interface RawMessage {
  id: number
  conversationId: number
  role: 'user' | 'assistant' | 'system'
  content: string
  stepsJson?: string | null
  status?: 'pending' | 'streaming' | 'complete' | 'error'
  durationMs?: number | null
  createdAt: string
}

/**
 * AI会话管理API类
 */
export class ConversationAPI {
  // ===== 会话管理 =====

  /**
   * 创建新会话
   * @param title 会话标题，可选
   * @returns 新创建的会话ID
   */
  async createConversation(title?: string): Promise<number> {
    try {
      return await invoke('create_conversation', { title })
    } catch (error) {
      throw new Error(handleError(error, '创建会话失败'))
    }
  }

  /**
   * 获取会话列表
   * @param limit 限制返回数量
   * @param offset 偏移量
   * @returns 会话列表
   */
  async getConversations(limit?: number, offset?: number): Promise<Conversation[]> {
    try {
      const conversations = await invoke<RawConversation[]>('get_conversations', { limit, offset })
      // 转换日期字符串为Date对象
      return conversations.map((conv: RawConversation) => ({
        ...conv,
        createdAt: new Date(conv.createdAt),
        updatedAt: new Date(conv.updatedAt),
      }))
    } catch (error) {
      throw new Error(handleError(error, '获取会话列表失败'))
    }
  }

  /**
   * 获取会话详情
   * @param conversationId 会话ID
   * @returns 会话详情
   */
  async getConversation(conversationId: number): Promise<Conversation> {
    try {
      const conversation = await invoke<RawConversation>('get_conversation', { conversationId })
      return {
        ...conversation,
        createdAt: new Date(conversation.createdAt),
        updatedAt: new Date(conversation.updatedAt),
      }
    } catch (error) {
      throw new Error(handleError(error, '获取会话详情失败'))
    }
  }

  /**
   * 更新会话标题
   * @param conversationId 会话ID
   * @param title 新标题
   */
  async updateConversationTitle(conversationId: number, title: string): Promise<void> {
    try {
      await invoke('update_conversation_title', { conversationId, title })
    } catch (error) {
      throw new Error(handleError(error, '更新会话标题失败'))
    }
  }

  /**
   * 删除会话
   * @param conversationId 会话ID
   */
  async deleteConversation(conversationId: number): Promise<void> {
    try {
      await invoke('delete_conversation', { conversationId })
    } catch (error) {
      throw new Error(handleError(error, '删除会话失败'))
    }
  }

  // ===== 消息管理 =====

  /**
   * 获取压缩上下文（供eko使用）
   * @param conversationId 会话ID
   * @param upToMessageId 截止到某条消息ID
   * @returns 压缩后的消息列表
   */
  async getCompressedContext(conversationId: number, upToMessageId?: number): Promise<Message[]> {
    try {
      const messages = await invoke<RawMessage[]>('get_compressed_context', {
        conversationId,
        upToMessageId,
      })
      // 转换日期字符串为Date对象，并解析steps/status/duration
      return messages.map((msg: RawMessage) => ({
        id: msg.id,
        conversationId: msg.conversationId,
        role: msg.role,
        content: msg.content,
        createdAt: new Date(msg.createdAt),
        steps: msg.stepsJson ? JSON.parse(msg.stepsJson) : undefined,
        status: msg.status,
        duration: msg.durationMs ?? undefined,
      }))
    } catch (error) {
      throw new Error(handleError(error, '获取压缩上下文失败'))
    }
  }

  /**
   * 保存单条消息（供eko使用）
   * @param conversationId 会话ID
   * @param role 消息角色 ("user" | "assistant" | "system")
   * @param content 消息内容
   * @returns 保存的消息ID
   */
  async saveMessage(conversationId: number, role: string, content: string): Promise<number> {
    try {
      return await invoke('save_message', {
        conversationId,
        role,
        content,
      })
    } catch (error) {
      throw new Error(handleError(error, '保存消息失败'))
    }
  }

  /**
   * 更新消息扩展（steps/status/duration）
   */
  async updateMessageMeta(
    messageId: number,
    steps?: any[] | null,
    status?: 'pending' | 'streaming' | 'complete' | 'error' | null,
    duration?: number | null
  ): Promise<void> {
    try {
      await invoke('update_message_meta', {
        messageId,
        stepsJson: steps ? JSON.stringify(steps) : null,
        status: status ?? null,
        durationMs: duration ?? null,
      })
    } catch (error) {
      throw new Error(handleError(error, '更新消息扩展失败'))
    }
  }

  /**
   * 截断会话（供eko使用）
   * @param conversationId 会话ID
   * @param truncateAfterMessageId 截断点消息ID
   */
  async truncateConversation(conversationId: number, truncateAfterMessageId: number): Promise<void> {
    try {
      await invoke('truncate_conversation', {
        conversationId,
        truncateAfterMessageId,
      })
    } catch (error) {
      throw new Error(handleError(error, '截断会话失败'))
    }
  }
}

// 创建API实例
export const conversationAPI = new ConversationAPI()

/**
 * 便捷的会话操作函数集合
 * 提供简化的会话功能调用接口（适配新eko架构）
 */
export const conversations = {
  // 会话管理
  create: (title?: string) => conversationAPI.createConversation(title),
  getList: (limit?: number, offset?: number) => conversationAPI.getConversations(limit, offset),
  get: (id: number) => conversationAPI.getConversation(id),
  updateTitle: (id: number, title: string) => conversationAPI.updateConversationTitle(id, title),
  delete: (id: number) => conversationAPI.deleteConversation(id),

  // 新的eko架构专用接口
  getCompressedContext: (conversationId: number, upToMessageId?: number) =>
    conversationAPI.getCompressedContext(conversationId, upToMessageId),
  saveMessage: (conversationId: number, role: string, content: string) =>
    conversationAPI.saveMessage(conversationId, role, content),
  updateMessageMeta: (
    messageId: number,
    steps?: any[] | null,
    status?: 'pending' | 'streaming' | 'complete' | 'error' | null,
    duration?: number | null
  ) => conversationAPI.updateMessageMeta(messageId, steps, status, duration),
  truncateConversation: (conversationId: number, truncateAfterMessageId: number) =>
    conversationAPI.truncateConversation(conversationId, truncateAfterMessageId),
}

// 默认导出
export default conversationAPI
