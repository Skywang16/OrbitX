/**
 * AI 模块 API
 *
 * 提供 AI 相关功能的统一接口，包括：
 * - 模型管理
 * - 会话管理
 * - 工具调用
 * - 设置管理
 */

import type { AIHealthStatus, AIModelConfig, AISettings, AIStats, Conversation, Message } from '@/types'
import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'
import type {
  RawConversation,
  RawMessage,
  AnalyzeCodeParams,
  AnalysisResult,
  WebFetchRequest,
  WebFetchResponse,
} from './types'

/**
 * AI 会话管理 API 类
 */
class ConversationAPI {
  // ===== 会话管理 =====

  async createConversation(title?: string): Promise<number> {
    try {
      return await invoke('create_conversation', { title })
    } catch (error) {
      throw new Error(handleError(error, '创建会话失败'))
    }
  }

  async getConversations(limit?: number, offset?: number): Promise<Conversation[]> {
    try {
      const conversations = await invoke<RawConversation[]>('get_conversations', { limit, offset })
      return conversations.map(this.convertConversation)
    } catch (error) {
      throw new Error(handleError(error, '获取会话列表失败'))
    }
  }

  async getConversation(conversationId: number): Promise<Conversation> {
    try {
      const conversation = await invoke<RawConversation>('get_conversation', { conversationId })
      return this.convertConversation(conversation)
    } catch (error) {
      throw new Error(handleError(error, '获取会话失败'))
    }
  }

  async updateConversationTitle(conversationId: number, title: string): Promise<void> {
    try {
      await invoke('update_conversation_title', { conversationId, title })
    } catch (error) {
      throw new Error(handleError(error, '更新会话标题失败'))
    }
  }

  async deleteConversation(conversationId: number): Promise<void> {
    try {
      await invoke('delete_conversation', { conversationId })
    } catch (error) {
      throw new Error(handleError(error, '删除会话失败'))
    }
  }

  async getCompressedContext(conversationId: number, upToMessageId?: number): Promise<Message[]> {
    try {
      const messages = await invoke<RawMessage[]>('get_compressed_context', {
        conversationId,
        upToMessageId,
      })
      return messages.map(this.convertMessage)
    } catch (error) {
      throw new Error(handleError(error, '获取会话上下文失败'))
    }
  }

  async saveMessage(conversationId: number, role: string, content: string): Promise<number> {
    try {
      return await invoke('save_message', { conversationId, role, content })
    } catch (error) {
      throw new Error(handleError(error, '保存消息失败'))
    }
  }

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
        status,
        durationMs: duration,
      })
    } catch (error) {
      throw new Error(handleError(error, '更新消息元数据失败'))
    }
  }

  async truncateConversation(conversationId: number, truncateAfterMessageId: number): Promise<void> {
    try {
      await invoke('truncate_conversation', { conversationId, truncateAfterMessageId })
    } catch (error) {
      throw new Error(handleError(error, '截断会话失败'))
    }
  }

  // ===== 数据转换方法 =====

  private convertConversation(raw: RawConversation): Conversation {
    return {
      id: raw.id,
      title: raw.title,
      messageCount: raw.messageCount,
      lastMessagePreview: raw.lastMessagePreview,
      createdAt: new Date(raw.createdAt),
      updatedAt: new Date(raw.updatedAt),
    }
  }

  private convertMessage(raw: RawMessage): Message {
    return {
      id: raw.id,
      conversationId: raw.conversationId,
      role: raw.role,
      content: raw.content,
      steps: raw.stepsJson ? JSON.parse(raw.stepsJson) : undefined,
      status: raw.status,
      duration: raw.durationMs || undefined,
      createdAt: new Date(raw.createdAt),
    }
  }
}

/**
 * 工具调用 API
 */

// ===== AST代码分析 =====

export async function analyzeCode(params: AnalyzeCodeParams): Promise<AnalysisResult> {
  try {
    return await invoke<AnalysisResult>('analyze_code', params as unknown as Record<string, unknown>)
  } catch (error) {
    throw new Error(handleError(error, '代码分析失败'))
  }
}

// ===== 网络请求 =====

export async function webFetchHeadless(request: WebFetchRequest): Promise<WebFetchResponse> {
  try {
    return await invoke<WebFetchResponse>('web_fetch_headless', { request })
  } catch (error) {
    throw new Error(handleError(error, '网络请求失败'))
  }
}

/**
 * AI API 接口类
 */
export class AiApi {
  private conversationAPI = new ConversationAPI()

  // ===== 模型管理 =====

  async getModels(): Promise<AIModelConfig[]> {
    try {
      return await invoke<AIModelConfig[]>('get_ai_models')
    } catch (error) {
      throw new Error(handleError(error, '获取AI模型失败'))
    }
  }

  async addModel(model: Omit<AIModelConfig, 'id'>): Promise<AIModelConfig> {
    try {
      return await invoke<AIModelConfig>('add_ai_model', { model })
    } catch (error) {
      throw new Error(handleError(error, '添加AI模型失败'))
    }
  }

  async updateModel(model: AIModelConfig): Promise<void> {
    try {
      await invoke('update_ai_model', { model })
    } catch (error) {
      throw new Error(handleError(error, '更新AI模型失败'))
    }
  }

  async deleteModel(id: string): Promise<void> {
    try {
      await invoke('remove_ai_model', { modelId: id })
    } catch (error) {
      throw new Error(handleError(error, '删除AI模型失败'))
    }
  }

  async setDefaultModel(id: string): Promise<void> {
    try {
      await invoke('set_default_ai_model', { modelId: id })
    } catch (error) {
      throw new Error(handleError(error, '设置默认AI模型失败'))
    }
  }

  async testConnectionWithConfig(config: AIModelConfig): Promise<boolean> {
    try {
      return await invoke<boolean>('test_ai_connection_with_config', { config })
    } catch (error) {
      throw new Error(handleError(error, 'AI模型连接测试失败'))
    }
  }

  async testConnection(modelId: string): Promise<boolean> {
    try {
      return await invoke<boolean>('test_ai_connection', { modelId: modelId })
    } catch (error) {
      throw new Error(handleError(error, 'AI模型连接测试失败'))
    }
  }

  async getUserPrefixPrompt(): Promise<string | null> {
    try {
      return await invoke<string | null>('get_user_prefix_prompt')
    } catch (error) {
      throw new Error(handleError(error, '获取用户前置提示词失败'))
    }
  }

  async setUserPrefixPrompt(prompt: string | null): Promise<void> {
    try {
      await invoke('set_user_prefix_prompt', { prompt })
    } catch (error) {
      throw new Error(handleError(error, '设置用户前置提示词失败'))
    }
  }

  // ===== 设置管理 =====

  async getSettings(): Promise<AISettings> {
    try {
      return await invoke<AISettings>('get_ai_settings')
    } catch (error) {
      throw new Error(handleError(error, '获取AI设置失败'))
    }
  }

  async updateSettings(settings: Partial<AISettings>): Promise<void> {
    try {
      await invoke('update_ai_settings', { settings })
    } catch (error) {
      throw new Error(handleError(error, '更新AI设置失败'))
    }
  }

  // ===== 统计信息 =====

  async getStats(): Promise<AIStats> {
    try {
      return await invoke<AIStats>('get_ai_stats')
    } catch (error) {
      throw new Error(handleError(error, '获取AI统计信息失败'))
    }
  }

  async getHealthStatus(): Promise<AIHealthStatus> {
    try {
      return await invoke<AIHealthStatus>('get_ai_health_status')
    } catch (error) {
      throw new Error(handleError(error, '获取AI健康状态失败'))
    }
  }

  // ===== 会话管理（代理到 ConversationAPI） =====

  async createConversation(title?: string) {
    return this.conversationAPI.createConversation(title)
  }

  async getConversations(limit?: number, offset?: number) {
    return this.conversationAPI.getConversations(limit, offset)
  }

  async getConversation(conversationId: number) {
    return this.conversationAPI.getConversation(conversationId)
  }

  async updateConversationTitle(conversationId: number, title: string) {
    return this.conversationAPI.updateConversationTitle(conversationId, title)
  }

  async deleteConversation(conversationId: number) {
    return this.conversationAPI.deleteConversation(conversationId)
  }

  async getCompressedContext(conversationId: number, upToMessageId?: number) {
    return this.conversationAPI.getCompressedContext(conversationId, upToMessageId)
  }

  async saveMessage(conversationId: number, role: string, content: string) {
    return this.conversationAPI.saveMessage(conversationId, role, content)
  }

  async updateMessageMeta(messageId: number, steps?: any[] | null, status?: string | null, duration?: number | null) {
    return this.conversationAPI.updateMessageMeta(messageId, steps, status as any, duration)
  }

  async truncateConversation(conversationId: number, truncateAfterMessageId: number) {
    return this.conversationAPI.truncateConversation(conversationId, truncateAfterMessageId)
  }

  // ===== 工具调用 =====

  async analyzeCode(params: AnalyzeCodeParams): Promise<AnalysisResult> {
    return analyzeCode(params)
  }

  async webFetch(request: WebFetchRequest): Promise<WebFetchResponse> {
    return webFetchHeadless(request)
  }
}

// 导出单例实例
export const aiApi = new AiApi()

// 导出类型
export type * from './types'

// 默认导出
export default aiApi
