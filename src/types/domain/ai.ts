import type { BaseConfig } from '../core'
import type { UiStep } from '../../api/agent/types'

export interface ToolExecution {
  name: string
  params: Record<string, unknown>
  status: 'running' | 'completed' | 'error'
  startTime: number
  endTime?: number
  result?: unknown
  error?: string
  toolId?: string
}

export type AIProvider = 'openai' | 'anthropic' | 'gemini' | 'qwen' | 'custom'

export type ModelType = 'chat' | 'embedding'

export interface AIModelConfig {
  id: string
  name: string
  provider: AIProvider
  apiUrl: string
  apiKey: string
  model: string
  modelType: ModelType
  enabled?: boolean
  options?: {
    maxTokens?: number
    temperature?: number
    timeout?: number
    customConfig?: string
  }
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

export interface AIStats {
  totalRequests: number
  successfulRequests: number
  failedRequests: number
  averageResponseTime: number
  tokensUsed: number
  cacheHitRate?: number
  modelUsage: Record<string, number>
}

export interface AIHealthStatus {
  modelId: string
  status: 'healthy' | 'degraded' | 'unhealthy'
  lastChecked: Date
  responseTime?: number
  error?: string
}

export interface StreamChunk {
  content: string
  isComplete: boolean
  metadata?: Record<string, unknown>
}

export type StreamCallback = (chunk: StreamChunk) => void

export interface ChannelStreamOptions {
  modelId?: string
  timeout?: number
  maxRetries?: number
}

export interface CancellableStream {
  cancel: () => void
}

export interface Conversation {
  id: number
  title: string
  messageCount: number
  createdAt: Date
  updatedAt: Date
}

export interface BaseStep {
  content: string
  timestamp: number
  metadata?: {
    thinkingDuration?: number
    errorType?: string
    errorDetails?: string
    streamId?: string // 流式ID，用于识别同一轮流式更新
  }
}

export interface ToolStep extends BaseStep {
  type: 'tool_use' | 'tool_result'
  toolExecution: ToolExecution
}

export interface NonToolStep extends BaseStep {
  type: 'thinking' | 'task' | 'task_thought' | 'text' | 'error'
}

export type AIOutputStep = ToolStep | NonToolStep

export interface Message {
  id: number
  conversationId: number
  role: 'user' | 'assistant' | 'system'
  createdAt: Date
  steps?: UiStep[]
  status?: 'streaming' | 'complete' | 'error'
  duration?: number
  // 双轨架构：user消息直接显示content，assistant消息只通过steps渲染
  content?: string // 仅用于user消息
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
  currentConversationId: number | null
  conversations: Conversation[]
  messages: Message[]
  isLoading: boolean
  error: string | null
}

export interface SendMessageRequest {
  conversationId: number
  content: string
  modelId?: string
}

export interface AIConfig extends BaseConfig {
  maxContextTokens: number
  modelName: string
  enableSemanticCompression: boolean
}

export interface ContextStats {
  conversationId: number
  totalMessages: number
  summaryGenerated: boolean
  lastSummaryAt?: Date
}
