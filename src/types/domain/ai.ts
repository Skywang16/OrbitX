import type { BaseConfig } from '../core'

export interface ToolExecution {
  name: string
  params: Record<string, unknown>
  status: 'running' | 'completed' | 'error'
  startTime: number
  endTime?: number
  result?: unknown
  error?: string
}

export type AIProvider = 'openAI' | 'claude' | 'custom'

export interface AIModelConfig {
  id: string
  name: string
  provider: AIProvider
  apiUrl: string
  apiKey: string
  model: string
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
  metadata?: Record<string, any>
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
  type: 'thinking' | 'workflow' | 'text' | 'error'
}

export type AIOutputStep = ToolStep | NonToolStep

export interface Message {
  id: number
  conversationId: number
  role: 'user' | 'assistant' | 'system'
  createdAt: Date

  steps?: AIOutputStep[]
  status?: 'pending' | 'streaming' | 'complete' | 'error'
  duration?: number

  content?: string
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

export interface TruncateAndResendRequest {
  conversationId: number
  truncateAfterMessageId: number
  newContent: string
  modelId?: string
}

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

export interface ChatSidebarConfig {
  width: number
  minWidth: number
  maxWidth: number
  defaultWidth: number
  resizable: boolean
  collapsible: boolean
}

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
  if (duration === null) return 'Running...'

  if (duration < 1000) return `${duration}ms`
  if (duration < 60000) return `${(duration / 1000).toFixed(1)}s`
  return `${(duration / 60000).toFixed(1)}min`
}
