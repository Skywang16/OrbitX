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

class ConversationAPI {
  async createConversation(title?: string): Promise<number> {
    try {
      return await invoke('create_conversation', { title })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to create conversation'))
    }
  }

  async getConversations(limit?: number, offset?: number): Promise<Conversation[]> {
    try {
      const conversations = await invoke<RawConversation[]>('get_conversations', { limit, offset })
      return conversations.map(this.convertConversation)
    } catch (error) {
      throw new Error(handleError(error, 'Failed to get conversations'))
    }
  }

  async getConversation(conversationId: number): Promise<Conversation> {
    try {
      const conversation = await invoke<RawConversation>('get_conversation', { conversationId })
      return this.convertConversation(conversation)
    } catch (error) {
      throw new Error(handleError(error, 'Failed to get conversation'))
    }
  }

  async updateConversationTitle(conversationId: number, title: string): Promise<void> {
    try {
      await invoke('update_conversation_title', { conversationId, title })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to update conversation title'))
    }
  }

  async deleteConversation(conversationId: number): Promise<void> {
    try {
      await invoke('delete_conversation', { conversationId })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to delete conversation'))
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
      throw new Error(handleError(error, 'Failed to get conversation context'))
    }
  }

  async buildPromptWithContext(
    conversationId: number,
    currentMessage: string,
    upToMessageId?: number,
    paneId?: number,
    tagContext?: any
  ): Promise<string> {
    try {
      const prompt = await invoke<string>('build_prompt_with_context', {
        conversationId,
        currentMessage,
        upToMessageId,
        paneId,
        tagContext,
      })
      return prompt
    } catch (error) {
      throw new Error(handleError(error, 'Failed to build prompt'))
    }
  }

  async saveMessage(conversationId: number, role: string, content: string): Promise<number> {
    try {
      return await invoke('save_message', { conversationId, role, content })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to save message'))
    }
  }

  async updateMessageContent(messageId: number, content: string): Promise<void> {
    try {
      await invoke('update_message_content', { messageId, content })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to update message content'))
    }
  }

  async updateMessageSteps(messageId: number, steps: any[]): Promise<void> {
    try {
      const cleanedSteps = this.cleanStepsData(steps)

      const stepsJson = JSON.stringify(cleanedSteps)
      await invoke('update_message_steps', {
        messageId,
        stepsJson,
      })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to update message steps'))
    }
  }

  async updateMessageStatus(
    messageId: number,
    status?: 'pending' | 'streaming' | 'complete' | 'error',
    duration?: number
  ): Promise<void> {
    try {
      await invoke('update_message_status', {
        messageId,
        status,
        durationMs: duration,
      })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to update message status'))
    }
  }

  async truncateConversation(conversationId: number, truncateAfterMessageId: number): Promise<void> {
    try {
      await invoke('truncate_conversation', { conversationId, truncateAfterMessageId })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to truncate conversation'))
    }
  }

  private cleanStepsData(steps: any[]): any[] {
    return steps.map(step => {
      if (step && typeof step === 'object') {
        const cleanedStep = { ...step }

        if (cleanedStep.result && typeof cleanedStep.result === 'object') {
          if (typeof cleanedStep.result.text === 'string') {
            cleanedStep.result.text = this.cleanJsonEscapes(cleanedStep.result.text)
          }

          if (Array.isArray(cleanedStep.result.content)) {
            cleanedStep.result.content = cleanedStep.result.content.map((item: any) => {
              if (item && typeof item.text === 'string') {
                return { ...item, text: this.cleanJsonEscapes(item.text) }
              }
              return item
            })
          }
        }

        return cleanedStep
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
    let steps: any[] | undefined = undefined
    if (raw.stepsJson) {
      try {
        steps = JSON.parse(raw.stepsJson)
      } catch (error) {
        console.error('Failed to parse steps:', error)
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
}

export async function analyzeCode(params: AnalyzeCodeParams): Promise<AnalysisResult> {
  try {
    return await invoke<AnalysisResult>('analyze_code', params as unknown as Record<string, unknown>)
  } catch (error) {
    throw new Error(handleError(error, 'Code analysis failed'))
  }
}

export async function webFetchHeadless(request: WebFetchRequest): Promise<WebFetchResponse> {
  try {
    return await invoke<WebFetchResponse>('web_fetch_headless', { request })
  } catch (error) {
    throw new Error(handleError(error, 'Web request failed'))
  }
}

export class AiApi {
  private conversationAPI = new ConversationAPI()

  async getModels(): Promise<AIModelConfig[]> {
    try {
      return await invoke<AIModelConfig[]>('get_ai_models')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to get AI models'))
    }
  }

  async addModel(model: Omit<AIModelConfig, 'id'>): Promise<AIModelConfig> {
    try {
      // 创建完整的模型配置，包含ID和时间戳
      const fullModel: AIModelConfig = {
        id: crypto.randomUUID(),
        ...model,
        enabled: model.enabled ?? true,
        createdAt: new Date(),
        updatedAt: new Date(),
      }
      const result = await invoke<AIModelConfig>('add_ai_model', { config: fullModel })
      return result
    } catch (error) {
      throw new Error(handleError(error, 'Failed to add AI model'))
    }
  }

  async updateModel(model: AIModelConfig): Promise<void> {
    try {
      const { id: modelId, ...updates } = model
      await invoke('update_ai_model', { modelId, updates })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to update AI model'))
    }
  }

  async deleteModel(id: string): Promise<void> {
    try {
      await invoke('remove_ai_model', { modelId: id })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to delete AI model'))
    }
  }

  async testConnectionWithConfig(config: AIModelConfig): Promise<boolean> {
    try {
      return await invoke<boolean>('test_ai_connection_with_config', { config })
    } catch (error) {
      throw new Error(handleError(error, 'AI model connection test failed'))
    }
  }

  async getUserPrefixPrompt(): Promise<string | null> {
    try {
      return await invoke<string | null>('get_user_prefix_prompt')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to get user prefix prompt'))
    }
  }

  async setUserPrefixPrompt(prompt: string | null): Promise<void> {
    try {
      await invoke('set_user_prefix_prompt', { prompt })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to set user prefix prompt'))
    }
  }

  // embedding模型相关方法已移除，统一使用AI模型接口通过modelType区分

  async getSettings(): Promise<AISettings> {
    try {
      return await invoke<AISettings>('get_ai_settings')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to get AI settings'))
    }
  }

  async updateSettings(settings: Partial<AISettings>): Promise<void> {
    try {
      await invoke('update_ai_settings', { settings })
    } catch (error) {
      throw new Error(handleError(error, 'Failed to update AI settings'))
    }
  }

  async getStats(): Promise<AIStats> {
    try {
      return await invoke<AIStats>('get_ai_stats')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to get AI stats'))
    }
  }

  async getHealthStatus(): Promise<AIHealthStatus> {
    try {
      return await invoke<AIHealthStatus>('get_ai_health_status')
    } catch (error) {
      throw new Error(handleError(error, 'Failed to get AI health status'))
    }
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
    tagContext?: any
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

  async updateMessageSteps(messageId: number, steps: any[]) {
    return this.conversationAPI.updateMessageSteps(messageId, steps)
  }

  async updateMessageStatus(messageId: number, status?: string, duration?: number) {
    return this.conversationAPI.updateMessageStatus(messageId, status as any, duration)
  }

  async truncateConversation(conversationId: number, truncateAfterMessageId: number) {
    return this.conversationAPI.truncateConversation(conversationId, truncateAfterMessageId)
  }

  async analyzeCode(params: AnalyzeCodeParams): Promise<AnalysisResult> {
    return analyzeCode(params)
  }

  async webFetch(request: WebFetchRequest): Promise<WebFetchResponse> {
    return webFetchHeadless(request)
  }
}

export const aiApi = new AiApi()

export type * from './types'

export default aiApi
