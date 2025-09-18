import type { AIHealthStatus, AIModelConfig, AISettings, AIStats, Conversation, Message, TagContextInfo } from '@/types'
import { invoke } from '@/utils/request'
import type { RawConversation, RawMessage, WebFetchRequest, WebFetchResponse, PersistedStep } from './types'

// 优雅的JSON解析工具函数
const safeJsonParse = <T>(json: string | null | undefined): T | undefined => {
  if (!json) return undefined
  try {
    return JSON.parse(json) as T
  } catch {
    return undefined
  }
}

class ConversationAPI {
  async createConversation(title?: string): Promise<number> {
    return await invoke<number>('ai_conversation_create', { title })
  }

  async getConversations(limit?: number, offset?: number): Promise<Conversation[]> {
    const conversations = await invoke<RawConversation[]>('ai_conversation_get_all', { limit, offset })
    return conversations.map(this.convertConversation)
  }

  async getConversation(conversationId: number): Promise<Conversation> {
    const conversation = await invoke<RawConversation>('ai_conversation_get', { conversationId })
    return this.convertConversation(conversation)
  }

  async updateConversationTitle(conversationId: number, title: string): Promise<void> {
    await invoke<void>('ai_conversation_update_title', { conversationId, title })
  }

  async deleteConversation(conversationId: number): Promise<void> {
    await invoke<void>('ai_conversation_delete', { conversationId })
  }

  async getCompressedContext(conversationId: number, upToMessageId?: number): Promise<Message[]> {
    const messages = await invoke<RawMessage[]>('ai_conversation_get_compressed_context', {
      conversationId,
      upToMessageId,
    })
    return messages.map(this.convertMessage)
  }

  async buildPromptWithContext(
    conversationId: number,
    currentMessage: string,
    upToMessageId?: number,
    paneId?: number,
    tagContext?: TagContextInfo
  ): Promise<string> {
    const prompt = await invoke<string>('ai_conversation_build_prompt_with_context', {
      conversationId,
      currentMessage,
      upToMessageId,
      paneId,
      tagContext,
    })
    return prompt
  }

  async saveMessage(conversationId: number, role: string, content: string): Promise<number> {
    return await invoke<number>('ai_conversation_save_message', { conversationId, role, content })
  }

  async updateMessageContent(messageId: number, content: string): Promise<void> {
    await invoke<void>('ai_conversation_update_message_content', { messageId, content })
  }

  async updateMessageSteps(messageId: number, steps: PersistedStep[]): Promise<void> {
    const cleanedSteps = this.cleanStepsData(steps)

    const stepsJson = JSON.stringify(cleanedSteps)
    await invoke<void>('ai_conversation_update_message_steps', {
      messageId,
      stepsJson,
    })
  }

  async updateMessageStatus(
    messageId: number,
    status?: 'pending' | 'streaming' | 'complete' | 'error',
    duration?: number
  ): Promise<void> {
    await invoke<void>('ai_conversation_update_message_status', {
      messageId,
      status,
      durationMs: duration,
    })
  }

  async truncateConversation(conversationId: number, truncateAfterMessageId: number): Promise<void> {
    await invoke<void>('ai_conversation_truncate', { conversationId, truncateAfterMessageId })
  }

  private cleanStepsData = (steps: PersistedStep[]): PersistedStep[] => steps.map(this.cleanSingleStep)

  private cleanSingleStep = (step: PersistedStep): PersistedStep => {
    if (step.type === 'text' && step.content) {
      return { ...step, content: this.cleanJsonEscapes(step.content) }
    }

    if (this.isToolStep(step)) {
      return { ...step, toolExecution: this.cleanToolExecution(step.toolExecution) }
    }

    return step
  }

  private isToolStep = (step: PersistedStep): step is Extract<PersistedStep, { type: 'tool_use' | 'tool_result' }> =>
    step.type === 'tool_use' || step.type === 'tool_result'

  private cleanToolExecution = (execution: any) => ({
    ...execution,
    result: this.cleanExecutionResult(execution.result),
  })

  private cleanExecutionResult = (result: any): any => {
    if (!result) return result

    // 字符串直接清理
    if (result.constructor === String) {
      return this.cleanJsonEscapes(String(result))
    }

    // 对象递归清理
    if (result.constructor === Object) {
      return this.cleanObjectResult(result)
    }

    return result
  }

  private cleanObjectResult = (obj: any): any => ({
    ...obj,
    ...(obj.text && { text: this.cleanJsonEscapes(obj.text) }),
    ...(obj.content &&
      Array.isArray(obj.content) && {
        content: obj.content.map(this.cleanContentItem),
      }),
  })

  private cleanContentItem = (item: any) => (item?.text ? { ...item, text: this.cleanJsonEscapes(item.text) } : item)

  private cleanJsonEscapes(text: string): string {
    return text.replace(/\\"/g, '"').replace(/\\n/g, '\n').replace(/\\t/g, '\t').replace(/\\\\/g, '\\')
  }

  private convertConversation = (raw: RawConversation): Conversation => ({
    id: raw.id,
    title: raw.title,
    messageCount: raw.messageCount,
    createdAt: new Date(raw.createdAt),
    updatedAt: new Date(raw.updatedAt),
  })

  private convertMessage = (raw: RawMessage): Message => ({
    id: raw.id,
    conversationId: raw.conversationId,
    role: raw.role,
    content: raw.content,
    steps: safeJsonParse<PersistedStep[]>(raw.stepsJson),
    status: raw.status,
    duration: raw.durationMs || undefined,
    createdAt: new Date(raw.createdAt),
  })
}

export async function webFetchHeadless(request: WebFetchRequest): Promise<WebFetchResponse> {
  return await invoke<WebFetchResponse>('network_web_fetch_headless', { request })
}

export class AiApi {
  private conversationAPI = new ConversationAPI()

  async getModels(): Promise<AIModelConfig[]> {
    return await invoke<AIModelConfig[]>('ai_models_get')
  }

  async addModel(model: Omit<AIModelConfig, 'id'>): Promise<AIModelConfig> {
    // 创建完整的模型配置，包含ID和时间戳
    const fullModel: AIModelConfig = {
      id: crypto.randomUUID(),
      ...model,
      enabled: model.enabled ?? true,
      createdAt: new Date(),
      updatedAt: new Date(),
    }
    const result = await invoke<AIModelConfig>('ai_models_add', { config: fullModel })
    return result
  }

  async updateModel(model: AIModelConfig): Promise<void> {
    const { id: modelId, ...updates } = model
    await invoke<void>('ai_models_update', { modelId, updates })
  }

  async deleteModel(id: string): Promise<void> {
    await invoke<void>('ai_models_remove', { modelId: id })
  }

  async testConnectionWithConfig(config: AIModelConfig): Promise<string> {
    return await invoke<string>('ai_models_test_connection', { config })
  }

  async getUserPrefixPrompt(): Promise<string | null> {
    return await invoke<string | null>('ai_conversation_get_user_prefix_prompt')
  }

  async setUserPrefixPrompt(prompt: string | null): Promise<void> {
    await invoke<void>('ai_conversation_set_user_prefix_prompt', { prompt })
  }

  // embedding模型相关方法已移除，统一使用AI模型接口通过modelType区分

  async getSettings(): Promise<AISettings> {
    return await invoke<AISettings>('get_ai_settings')
  }

  async updateSettings(settings: Partial<AISettings>): Promise<void> {
    await invoke<void>('update_ai_settings', { settings })
  }

  async getStats(): Promise<AIStats> {
    return await invoke<AIStats>('get_ai_stats')
  }

  async getHealthStatus(): Promise<AIHealthStatus> {
    return await invoke<AIHealthStatus>('get_ai_health_status')
  }

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

  async buildPromptWithContext(
    conversationId: number,
    currentMessage: string,
    upToMessageId?: number,
    paneId?: number,
    tagContext?: TagContextInfo
  ) {
    return this.conversationAPI.buildPromptWithContext(
      conversationId,
      currentMessage,
      upToMessageId,
      paneId,
      tagContext
    )
  }

  async saveMessage(conversationId: number, role: string, content: string) {
    return this.conversationAPI.saveMessage(conversationId, role, content)
  }

  async updateMessageContent(messageId: number, content: string) {
    return this.conversationAPI.updateMessageContent(messageId, content)
  }

  async updateMessageSteps(messageId: number, steps: PersistedStep[]) {
    return this.conversationAPI.updateMessageSteps(messageId, steps)
  }

  async updateMessageStatus(
    messageId: number,
    status?: 'pending' | 'streaming' | 'complete' | 'error',
    duration?: number
  ) {
    return this.conversationAPI.updateMessageStatus(messageId, status, duration)
  }

  async truncateConversation(conversationId: number, truncateAfterMessageId: number) {
    return this.conversationAPI.truncateConversation(conversationId, truncateAfterMessageId)
  }

  async webFetch(request: WebFetchRequest): Promise<WebFetchResponse> {
    return webFetchHeadless(request)
  }
}

export const aiApi = new AiApi()

export type * from './types'

export default aiApi
