/**
 * AI业务领域类型定义 - 统一合并版本
 * 包含所有AI相关类型，消除循环依赖
 */

import type { BaseConfig } from '../core'

// ===== 工具执行相关类型 =====

export interface ToolExecution {
  /** 工具名称 */
  name: string
  /** 所有参数 */
  params: Record<string, unknown>
  /** 执行状态 */
  status: 'running' | 'completed' | 'error'
  /** 开始时间 */
  startTime: number
  /** 结束时间 */
  endTime?: number
  /** 工具执行结果 */
  result?: unknown
  /** 错误信息 */
  error?: string
}

// ===== AI提供商和模型类型 =====

export type AIProvider = 'openAI' | 'claude' | 'custom'

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

// ===== 错误处理类型 =====

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

// ===== 流式通信类型 =====

export interface StreamChunk {
  /** 流式内容 */
  content: string
  /** 是否完成 */
  isComplete: boolean
  /** 元数据 */
  metadata?: Record<string, any>
}

export type StreamCallback = (chunk: StreamChunk) => void

export interface ChannelStreamOptions {
  /** 模型ID */
  modelId?: string
  /** 超时时间(毫秒) */
  timeout?: number
  /** 最大重试次数 */
  maxRetries?: number
}

export interface CancellableStream {
  /** 取消流式请求 */
  cancel: () => void
}

// ===== 会话管理类型 =====

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
  }
}

export interface ToolStep extends BaseStep {
  type: 'tool_use' | 'tool_result'
  toolExecution: ToolExecution
}

export interface NonToolStep extends BaseStep {
  type: 'thinking' | 'workflow' | 'text' | 'error'
}

export type AIOutputStep = ToolStep | NonToolStep

export interface Message {
  id: number
  conversationId: number
  role: 'user' | 'assistant' | 'system'
  createdAt: Date

  // AI消息的扩展字段
  steps?: AIOutputStep[]
  status?: 'pending' | 'streaming' | 'complete' | 'error'
  duration?: number

  // 用户消息内容
  content?: string
}

// ===== 聊天状态类型 =====

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

// ===== 请求和响应类型 =====

export interface SendMessageRequest {
  conversationId: number
  content: string
  modelId?: string
}

export interface TruncateAndResendRequest {
  conversationId: number
  truncateAfterMessageId: number
  newContent: string
  modelId?: string
}

// ===== 代码分析类型 =====

export interface CodeSymbol {
  name: string
  type: string
  line: number
  column: number
  range?: {
    start: { line: number; column: number }
    end: { line: number; column: number }
  }
}

export interface CodeAnalysis {
  file: string
  language: string
  symbols: CodeSymbol[]
  imports: string[]
  exports: string[]
}

export interface BatchCodeAnalysis {
  analyses: CodeAnalysis[]
  total_files: number
  success_count: number
  error_count: number
}

export interface AnalyzeCodeParams {
  path: string
  recursive?: boolean
  include?: string[]
  exclude?: string[]
}

export type AnalysisResult = BatchCodeAnalysis

// ===== Web请求类型 =====

export interface WebFetchRequest {
  url: string
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE'
  headers?: Record<string, string>
  body?: string
  timeout?: number
}

export interface WebFetchResponse {
  status: number
  status_text: string
  headers: Record<string, string>
  data: string
  final_url: string
  success: boolean
  error?: string
  response_time: number
  content_type?: string
  content_length?: number
}

// ===== 原始数据格式类型 =====

export interface RawConversation {
  id: number
  title: string
  messageCount: number
  createdAt: string
  updatedAt: string
}

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

// ===== AI配置类型 =====

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

// ===== 聊天配置类型 =====

export interface ChatSidebarConfig {
  width: number
  minWidth: number
  maxWidth: number
  defaultWidth: number
  resizable: boolean
  collapsible: boolean
}

// ===== Agent消息类型 =====

export interface AgentTextMessage {
  type: 'text'
  content: string
  timestamp: string
}

export interface AgentWorkflowMessage {
  type: 'workflow'
  stage: string
  content: string
  timestamp: string
  workflow?: Record<string, unknown>
  step?: Record<string, unknown>
}

export type AgentMessageData = AgentTextMessage | AgentWorkflowMessage

// ===== 工具函数 =====

export function createToolExecution(
  name: string,
  params: Record<string, unknown>,
  status: 'running' | 'completed' | 'error' = 'running'
): ToolExecution {
  return {
    name,
    params,
    status,
    startTime: Date.now(),
  }
}

export function getExecutionDuration(toolExecution: ToolExecution): number | null {
  if (!toolExecution.endTime) return null
  return toolExecution.endTime - toolExecution.startTime
}

export function formatExecutionDuration(toolExecution: ToolExecution): string {
  const duration = getExecutionDuration(toolExecution)
  if (duration === null) return '执行中...'

  if (duration < 1000) return `${duration}ms`
  if (duration < 60000) return `${(duration / 1000).toFixed(1)}s`
  return `${(duration / 60000).toFixed(1)}min`
}
