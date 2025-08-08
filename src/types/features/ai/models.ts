/**
 * AI模型配置相关类型定义
 */

// ===== AI提供商类型 =====

export type AIProvider = 'openAI' | 'claude' | 'custom'

// ===== AI模型配置类型 =====

export interface AIModelConfig {
  id: string
  name: string
  provider: AIProvider
  apiUrl: string
  apiKey: string
  model: string
  isDefault?: boolean
  options?: {
    maxTokens?: number
    temperature?: number
    timeout?: number
    customConfig?: string
  }
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

// ===== AI设置类型 =====

export interface AISettings {
  models: AIModelConfig[]
  defaultModelId: string | null
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

// ===== 错误类型 =====

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

// ===== 统计和监控类型 =====

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
