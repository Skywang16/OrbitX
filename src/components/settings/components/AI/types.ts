/**
 * AI设置组件相关的类型定义
 */

// 重新导出通用AI类型
export type {
  AIHealthStatus,
  AIModelConfig,
  AIProvider,
  AIRequest,
  AIResponse,
  AISettings,
  AIStats,
  CacheEntry,
  CacheStats,
  ChatMessage,
  ChatSession,
  CommandExplanation,
  ErrorAnalysis,
  TerminalContext,
} from '@/types'

// 导入需要在本文件中使用的类型
import type { AIModelConfig, AIProvider } from '@/types'

// AI设置组件特有的类型
export interface AISettingsFormData {
  models: AIModelConfig[]
  defaultModelId: string | null
  features: {
    chat: {
      enabled: boolean
      model: string
      explanation: boolean
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

// AI模型表单数据
export interface AIModelFormData {
  id?: string
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

// AI设置验证错误
export interface AISettingsValidationError {
  field: string
  message: string
}

// AI模型测试结果
export interface AIModelTestResult {
  success: boolean
  responseTime?: number
  error?: string
  metadata?: {
    model?: string
    tokensUsed?: number
  }
}

// AI功能配置选项
export interface AIFeatureConfig {
  key: string
  label: string
  description: string
  enabled: boolean
  options?: Record<string, unknown>
}

// AI性能配置选项
export interface AIPerformanceConfig {
  key: string
  label: string
  description: string
  value: number
  min: number
  max: number
  unit?: string
}
