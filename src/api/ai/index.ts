import type {
  AIHealthStatus,
  AIModelConfig,
  AISettings,
  AIStats,
  Conversation,
  Message,
  TagContextInfo,
  AIOutputStep,
} from '@/types'
import { invoke } from '@/utils/request'
import type { RawConversation, RawMessage, WebFetchRequest, WebFetchResponse, PersistedStep } from './types'

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

  private cleanStepsData(steps: PersistedStep[]): PersistedStep[] {
    const isToolStep = (s: PersistedStep): s is Extract<PersistedStep, { type: 'tool_use' | 'tool_result' }> =>
      s.type === 'tool_use' || s.type === 'tool_result'
    return steps.map(s => {
      const step: PersistedStep = { ...s }
      if (step.type === 'text' && typeof step.content === 'string') {
        step.content = this.cleanJsonEscapes(step.content)
      }
      if (isToolStep(step)) {
        const te = { ...step.toolExecution }
        const r = te.result
        if (typeof r === 'string') {
          te.result = this.cleanJsonEscapes(r)
        } else if (r && typeof r === 'object') {
          const obj = r as Record<string, unknown>
          if (typeof obj['text'] === 'string') {
            obj['text'] = this.cleanJsonEscapes(obj['text'] as string)
          }
          const content = obj['content']
          if (Array.isArray(content)) {
            obj['content'] = (content as Array<Record<string, unknown>>).map(item => {
              if (item && typeof item['text'] === 'string') {
                return { ...item, text: this.cleanJsonEscapes(item['text'] as string) }
              }
              return item
            })
          }
          te.result = obj
        }
        step.toolExecution = te
      }
      return step
    })
  }

  private cleanJsonEscapes(text: string): string {
    return text.replace(/\\"/g, '"').replace(/\\n/g, '\n').replace(/\\t/g, '\t').replace(/\\\\/g, '\\')
  }

  private convertConversation(raw: RawConversation): Conversation {
    return {
      id: raw.id,
      title: raw.title,
      messageCount: raw.messageCount,
      createdAt: new Date(raw.createdAt),
      updatedAt: new Date(raw.updatedAt),
    }
  }

  private convertMessage(raw: RawMessage): Message {
    let steps: AIOutputStep[] | undefined = undefined
    if (raw.stepsJson) {
      try {
        const parsed = JSON.parse(raw.stepsJson) as PersistedStep[]
        steps = this.convertPersistedSteps(parsed)
      } catch {
        steps = undefined
      }
    }

    return {
      id: raw.id,
      conversationId: raw.conversationId,
      role: raw.role,
      content: raw.content,
      steps,
      status: raw.status,
      duration: raw.durationMs || undefined,
      createdAt: new Date(raw.createdAt),
    }
  }

  private convertPersistedSteps(steps: PersistedStep[]): AIOutputStep[] {
    const isToolStep = (s: PersistedStep): s is Extract<PersistedStep, { type: 'tool_use' | 'tool_result' }> =>
      s.type === 'tool_use' || s.type === 'tool_result'
    return steps.map(s => {
      const ts = typeof s.timestamp === 'number' ? s.timestamp : Date.now()
      if (isToolStep(s)) {
        const exec = s.toolExecution
        let status: 'running' | 'completed' | 'error'
        if (exec.status === 'running') {
          status = 'running'
        } else if (exec.status === 'failed' || exec.status === 'error') {
          status = 'error'
        } else {
          status = 'completed'
        }
        return {
          type: 'tool_use',
          content: s.content ?? '',
          timestamp: ts,
          toolExecution: {
            name: exec.name,
            params: exec.params && typeof exec.params === 'object' ? (exec.params as Record<string, unknown>) : {},
            status,
            startTime: exec.startTime,
            endTime: exec.endTime,
            result: exec.result,
            error: exec.error,
            toolId: exec.toolId,
          },
        } as AIOutputStep
      }
      return {
        type: s.type as 'thinking' | 'task' | 'task_thought' | 'text' | 'error',
        content: s.content ?? '',
        timestamp: ts,
      } as AIOutputStep
    })
  }
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
