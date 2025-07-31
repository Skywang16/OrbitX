/**
 * AI流式通信相关类型定义
 */

// ===== 基础流式类型 =====

export interface StreamChunk {
  /** 流式内容 */
  content: string
  /** 是否完成 */
  isComplete: boolean
  /** 元数据 */
  metadata?: Record<string, any>
}

export type StreamCallback = (chunk: StreamChunk) => void

// ===== Channel相关类型 =====

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

// ===== 流式聊天管理器类型 =====

export interface StreamingSession {
  /** 会话ID */
  sessionId: string
  /** 消息内容 */
  message: string
  /** 模型ID */
  modelId?: string
  /** 开始时间 */
  startTime: number
  /** 是否活跃 */
  isActive: boolean
}

export interface StreamingStats {
  /** 活跃流数量 */
  activeStreams: number
  /** 总流数量 */
  totalStreams: number
  /** 平均响应时间 */
  averageResponseTime: number
  /** 成功率 */
  successRate: number
}

// ===== 流式错误类型 =====

export interface StreamingError {
  /** 错误类型 */
  type: 'network' | 'timeout' | 'cancelled' | 'model' | 'unknown'
  /** 错误消息 */
  message: string
  /** 会话ID */
  sessionId?: string
  /** 模型ID */
  modelId?: string
  /** 错误时间 */
  timestamp: number
}

// ===== 流式事件类型 =====

export interface StreamingEvents {
  /** 流开始 */
  'stream:start': {
    sessionId: string
    modelId?: string
    message: string
  }

  /** 流数据块 */
  'stream:chunk': {
    sessionId: string
    chunk: StreamChunk
  }

  /** 流完成 */
  'stream:complete': {
    sessionId: string
    totalChunks: number
    duration: number
  }

  /** 流错误 */
  'stream:error': {
    sessionId: string
    error: StreamingError
  }

  /** 流取消 */
  'stream:cancel': {
    sessionId: string
    reason: string
  }
}

// ===== 流式配置类型 =====

export interface StreamingConfig {
  /** 默认超时时间 */
  defaultTimeout: number
  /** 最大并发流数量 */
  maxConcurrentStreams: number
  /** 重试配置 */
  retry: {
    maxRetries: number
    retryDelay: number
    backoffMultiplier: number
  }
  /** 缓存配置 */
  cache: {
    enabled: boolean
    ttl: number
    maxSize: number
  }
}

// ===== 流式监控类型 =====

export interface StreamingMetrics {
  /** 请求总数 */
  totalRequests: number
  /** 成功请求数 */
  successfulRequests: number
  /** 失败请求数 */
  failedRequests: number
  /** 取消请求数 */
  cancelledRequests: number
  /** 平均响应时间 */
  averageResponseTime: number
  /** 平均数据块大小 */
  averageChunkSize: number
  /** 吞吐量(chunks/second) */
  throughput: number
}

// ===== 流式适配器类型 =====

export interface StreamingAdapter {
  /** 适配器名称 */
  name: string
  /** 是否支持流式 */
  supportsStreaming: boolean
  /** 是否支持取消 */
  supportsCancellation: boolean
  /** 最大并发数 */
  maxConcurrency: number
  /** 创建流式请求 */
  createStream: (message: string, options?: ChannelStreamOptions) => Promise<AsyncIterable<StreamChunk>>
}

// ===== 批量流式类型 =====

export interface BatchStreamRequest {
  /** 请求ID */
  requestId: string
  /** 消息内容 */
  message: string
  /** 模型ID */
  modelId?: string
  /** 优先级 */
  priority?: number
}

export interface BatchStreamResponse {
  /** 请求ID */
  requestId: string
  /** 流式数据 */
  stream: AsyncIterable<StreamChunk>
  /** 取消函数 */
  cancel: () => void
}

// ===== 流式队列类型 =====

export interface StreamingQueue {
  /** 队列中的请求 */
  pending: BatchStreamRequest[]
  /** 正在处理的请求 */
  processing: BatchStreamRequest[]
  /** 已完成的请求 */
  completed: BatchStreamRequest[]
  /** 队列统计 */
  stats: {
    totalRequests: number
    processedRequests: number
    failedRequests: number
    averageWaitTime: number
  }
}

// ===== 实用工具类型 =====

export type StreamingEventHandler<T extends keyof StreamingEvents> = (event: StreamingEvents[T]) => void | Promise<void>

export type StreamingMiddleware = (
  chunk: StreamChunk,
  context: { sessionId: string; modelId?: string }
) => StreamChunk | Promise<StreamChunk>

export interface StreamingPlugin {
  /** 插件名称 */
  name: string
  /** 插件版本 */
  version: string
  /** 初始化插件 */
  initialize: () => void | Promise<void>
  /** 处理流式数据 */
  processChunk?: StreamingMiddleware
  /** 清理资源 */
  cleanup?: () => void | Promise<void>
}
