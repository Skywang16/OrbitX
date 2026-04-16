import type { BaseConfig } from '../core'
import type { AuthType, OAuthConfig } from '../oauth'
import type { Message } from './aiMessage'

/** AI Provider identifier (dynamic, from models.dev) */
export type AIProvider = string

// ============================================================================
// Provider registry types (backend hardcoded providers)
// ============================================================================

export interface ModelCapabilities {
  reasoning: boolean
  toolCall: boolean
  attachment: boolean
}

export interface PresetModel {
  id: string
  name: string
  maxTokens?: number
  contextWindow: number
  description?: string
  capabilities: ModelCapabilities
}

export interface ProviderMetadata {
  providerType: string
  displayName: string
  defaultApiUrl: string
  presetModels: PresetModel[]
}

export type ModelType = 'chat' | 'embedding'

export interface AIModelConfig {
  id: string
  provider: AIProvider
  authType: AuthType
  apiUrl?: string
  apiKey?: string
  model: string
  displayName?: string
  modelType: ModelType
  options?: {
    maxContextTokens?: number
    temperature?: number
    timeoutSeconds?: number
    dimension?: number // Dimension of the vector model
    contextWindow?: number
    maxTokens?: number
    enableDeepThinking?: boolean // Enable deep thinking (Anthropic Extended Thinking / OpenAI Reasoning)
    reasoningEffort?: string // OpenAI Responses API effort: minimal/low/medium/high/xhigh
  }
  oauthConfig?: OAuthConfig
  createdAt?: Date
  updatedAt?: Date
}

export interface AIModelFileEntry {
  id: string
  model: string
  displayName?: string
  modelType: ModelType
  authType: AuthType
  apiUrl?: string
  apiKey?: string
  options?: AIModelConfig['options']
  oauthConfig?: OAuthConfig
  createdAt?: Date
  updatedAt?: Date
}

export interface AIModelProviderConfig {
  models: AIModelFileEntry[]
}

export interface AIModelDefaults {
  chatModelId?: string
  embeddingModelId?: string
}

export interface AgentModelDefaults {
  subagent?: string
}

export interface AgentModelConfig {
  defaults?: AgentModelDefaults
  bindings?: Record<string, string>
}

export interface AIModelsConfig {
  version: number
  defaults?: AIModelDefaults
  agents?: AgentModelConfig
  providers: Record<string, AIModelProviderConfig>
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

export interface ChatInputState {
  value: string
  isComposing: boolean
  placeholder: string
  disabled: boolean
}

export interface ConversationState {
  currentThreadId: number | null | -1
  threads: Conversation[]
  messages: Message[]
  isLoading: boolean
  error: string | null
}

export interface SendMessageRequest {
  threadId: number
  content: string
  modelId?: string
}

export interface AIConfig extends BaseConfig {
  maxContextTokens: number
  modelName: string
  enableSemanticCompression: boolean
}
