import type { BaseConfig } from '../core'
import type { Message } from './aiMessage'
import type { AuthType, OAuthConfig } from '../oauth'

export type AIProvider = 'anthropic' | 'openai_compatible'

export type ModelType = 'chat' | 'embedding'

export interface AIModelConfig {
  id: string
  provider: AIProvider
  authType: AuthType
  apiUrl?: string
  apiKey?: string
  model: string
  modelType: ModelType
  options?: {
    maxContextTokens?: number
    temperature?: number
    timeout?: number
    dimension?: number // 向量模型的维度
    contextWindow?: number
    maxTokens?: number
  }
  oauthConfig?: OAuthConfig
  useCustomBaseUrl?: boolean
  createdAt?: Date
  updatedAt?: Date
}

export interface AIResponse {
  content: string
  responseType: 'text' | 'code' | 'command'
  suggestions?: string[]
  metadata?: {
    model?: string
    tokensUsed?: number
    responseTime?: number
  }
  error?: {
    message: string
    code?: string
    details?: Record<string, unknown>
    providerResponse?: Record<string, unknown>
  }
}

export interface AISettings {
  models: AIModelConfig[]
  features: {
    chat: {
      enabled: boolean
      model?: string
      explanation?: boolean
      maxHistoryLength: number
      autoSaveHistory: boolean
      contextWindowSize: number
    }
  }
  performance: {
    requestTimeout: number
    maxConcurrentRequests: number
    cacheEnabled: boolean
    cacheTtl: number
  }
}

export enum AIErrorType {
  CONFIGURATION_ERROR = 'CONFIGURATION_ERROR',
  AUTHENTICATION_ERROR = 'AUTHENTICATION_ERROR',
  NETWORK_ERROR = 'NETWORK_ERROR',
  RATE_LIMIT_ERROR = 'RATE_LIMIT_ERROR',
  MODEL_ERROR = 'MODEL_ERROR',
  TIMEOUT_ERROR = 'TIMEOUT_ERROR',
  VALIDATION_ERROR = 'VALIDATION_ERROR',
  UNKNOWN_ERROR = 'UNKNOWN_ERROR',
}

export class AIError extends Error {
  constructor(
    public type: AIErrorType,
    message: string,
    public modelId?: string,
    public details?: Record<string, unknown>
  ) {
    super(message)
    this.name = 'AIError'
  }
}

export interface Conversation {
  id: number
  title: string
  workspacePath?: string | null
  messageCount?: number
  createdAt: Date
  updatedAt: Date
}

export type ChatStatus = 'idle' | 'loading' | 'streaming' | 'error'
export type ChatMode = 'chat' | 'agent'

export interface ChatInputState {
  value: string
  isComposing: boolean
  placeholder: string
  disabled: boolean
}

export interface ConversationState {
  currentSessionId: number | null | -1
  sessions: Conversation[]
  messages: Message[]
  isLoading: boolean
  error: string | null
}

export interface SendMessageRequest {
  sessionId: number
  content: string
  modelId?: string
}

export interface AIConfig extends BaseConfig {
  maxContextTokens: number
  modelName: string
  enableSemanticCompression: boolean
}
