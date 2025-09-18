/**
 * AI 模块类型定义
 */

export type { AIHealthStatus, AIModelConfig, AISettings, AIStats, Conversation, Message } from '@/types'

// ===== 会话管理类型 =====

/** 原始会话数据格式（后端返回） */
export interface RawConversation {
  id: number
  title: string
  messageCount: number
  createdAt: string
  updatedAt: string
}

export type PersistedNonToolStepType = 'thinking' | 'task' | 'task_thought' | 'text' | 'error'
export type PersistedToolStepType = 'tool_use' | 'tool_result'
export type PersistedStepType = PersistedNonToolStepType | PersistedToolStepType

export interface PersistedNonToolStep {
  type: PersistedNonToolStepType
  content?: string
  timestamp?: number
  metadata?: {
    thinkingDuration?: number
    errorType?: string
    errorDetails?: string
    streamId?: string // 流式ID，用于识别同一轮流式更新
  }
}

export interface PersistedToolExecution {
  name: string
  status: 'running' | 'completed' | 'error' | 'failed'
  params?: Record<string, unknown>
  result?: unknown
  error?: string
  toolId?: string
  startTime?: number
  endTime?: number
}

export interface PersistedToolStep {
  type: PersistedToolStepType
  content?: string
  timestamp?: number
  toolExecution: PersistedToolExecution
}

export type PersistedStep = PersistedNonToolStep | PersistedToolStep

/** 原始消息数据格式（后端返回） */
export interface RawMessage {
  id: number
  conversationId: number
  role: 'user' | 'assistant' | 'system'
  content: string
  stepsJson?: string | null
  status?: 'pending' | 'streaming' | 'complete' | 'error'
  durationMs?: number | null
  createdAt: string
}

export interface WebFetchRequest {
  /** 请求URL */
  url: string
  /** 请求方法 */
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE'
  /** 请求头 */
  headers?: Record<string, string>
  /** 请求体 */
  body?: string
  /** 超时时间（毫秒） */
  timeout?: number
}

export interface WebFetchResponse {
  /** 响应状态码 */
  status: number
  /** 响应状态文本 */
  status_text: string
  /** 响应头 */
  headers: Record<string, string>
  /** 响应数据 */
  data: string
  /** 最终URL */
  final_url: string
  /** 是否成功 */
  success: boolean
  /** 错误信息 */
  error?: string
  /** 响应时间 */
  response_time: number
  /** 内容类型 */
  content_type?: string
  /** 内容长度 */
  content_length?: number
}
