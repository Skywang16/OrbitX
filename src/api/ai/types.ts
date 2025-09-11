/**
 * AI 模块类型定义
 */

// 重新导出主要类型
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

// ===== 工具调用类型 =====

/** Web 请求类型 */
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

/** Web 响应类型 */
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
