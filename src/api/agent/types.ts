/**
 * Agent API 类型定义
 *
 * 定义Agent系统的所有接口类型，与后端TaskExecutor保持一致
 */

// ===== 核心类型定义 =====

/**
 * 任务执行参数
 */
export interface ExecuteTaskParams {
  /** 会话ID */
  conversationId: number
  /** 用户提示 */
  userPrompt: string
  /** 配置覆盖 */
  configOverrides?: Record<string, unknown>
  /** 要恢复的任务ID */
  restoreTaskId?: string
}

/**
 * 任务摘要信息
 */
export interface TaskSummary {
  /** 任务ID */
  taskId: string
  /** 会话ID */
  conversationId: number
  /** 任务状态 */
  status: TaskStatus
  /** 当前迭代次数 */
  currentIteration: number
  /** 错误次数 */
  errorCount: number
  /** 创建时间 */
  createdAt: string
  /** 更新时间 */
  updatedAt: string
  /** 用户提示 */
  userPrompt?: string
  /** 完成时间 */
  completedAt?: string
}

/**
 * 任务状态
 */
export type TaskStatus =
  | 'created' // 已创建
  | 'running' // 运行中
  | 'paused' // 已暂停
  | 'completed' // 已完成
  | 'error' // 出错
  | 'cancelled' // 已取消

/**
 * 任务进度事件负载（与Rust TaskProgressPayload保持一致）
 */
export type TaskProgressPayload =
  | TaskCreatedEvent
  | StatusChangedEvent
  | TaskStartedEvent
  | ThinkingEvent
  | TextEvent
  | ToolUseEvent
  | ToolResultEvent
  | FinalAnswerEvent
  | FinishEvent
  | TaskPausedEvent
  | TaskResumedEvent
  | TaskCompletedEvent
  | TaskErrorEvent
  | TaskCancelledEvent
  | StatusUpdateEvent
  | SystemMessageEvent
  | ToolPreparingEvent
  | ErrorEvent

/**
 * 任务已创建事件
 */
export interface TaskCreatedEvent {
  type: 'TaskCreated'
  payload: {
    taskId: string
    conversationId: number
    userPrompt: string
  }
}

/**
 * 状态变更事件
 */
export interface StatusChangedEvent {
  type: 'StatusChanged'
  payload: {
    taskId: string
    status: TaskStatus
    timestamp: string
  }
}

/**
 * 任务开始执行事件
 */
export interface TaskStartedEvent {
  type: 'TaskStarted'
  payload: {
    taskId: string
    iteration: number
  }
}

/**
 * Agent正在思考事件
 */
export interface ThinkingEvent {
  type: 'Thinking'
  payload: {
    taskId: string
    iteration: number
    thought: string
    streamId: string
    streamDone: boolean
    timestamp: string
  }
}

/**
 * 文本流事件（EKO风格：带streamId/streamDone）
 */
export interface TextEvent {
  type: 'Text'
  payload: {
    taskId: string
    iteration: number
    text: string
    streamId: string
    streamDone: boolean
    timestamp: string
  }
}

/**
 * 开始调用工具事件
 */
export interface ToolUseEvent {
  type: 'ToolUse'
  payload: {
    taskId: string
    iteration: number
    toolId: string
    toolName: string
    params: Record<string, unknown>
    timestamp: string
  }
}

/**
 * 工具调用结果事件
 */
export interface ToolResultEvent {
  type: 'ToolResult'
  payload: {
    taskId: string
    iteration: number
    toolId: string
    toolName: string
    result: unknown
    isError: boolean
    timestamp: string
  }
}

/**
 * 最终答案事件
 */
export interface FinalAnswerEvent {
  type: 'FinalAnswer'
  payload: {
    taskId: string
    iteration: number
    answer: string
    timestamp: string
  }
}

/**
 * 结束事件（EKO风格）
 */
export interface FinishEvent {
  type: 'Finish'
  payload: {
    taskId: string
    iteration: number
    finishReason: string
    usage?: {
      promptTokens: number
      completionTokens: number
      totalTokens: number
    }
    timestamp: string
  }
}

/**
 * 任务暂停事件
 */
export interface TaskPausedEvent {
  type: 'TaskPaused'
  payload: {
    taskId: string
    reason: string
    timestamp: string
  }
}

/**
 * 任务恢复事件
 */
export interface TaskResumedEvent {
  type: 'TaskResumed'
  payload: {
    taskId: string
    fromIteration: number
    timestamp: string
  }
}

/**
 * 任务完成事件
 */
export interface TaskCompletedEvent {
  type: 'TaskCompleted'
  payload: {
    taskId: string
    finalIteration: number
    completionReason: string
    timestamp: string
  }
}

/**
 * 任务错误事件
 */
export interface TaskErrorEvent {
  type: 'TaskError'
  payload: {
    taskId: string
    iteration: number
    errorMessage: string
    errorType: string
    isRecoverable: boolean
    timestamp: string
  }
}

/**
 * 任务取消事件
 */
export interface TaskCancelledEvent {
  type: 'TaskCancelled'
  payload: {
    taskId: string
    reason: string
    timestamp: string
  }
}

/**
 * 状态更新事件
 */
export interface StatusUpdateEvent {
  type: 'StatusUpdate'
  payload: {
    taskId: string
    status: string
    currentIteration: number
    errorCount: number
    timestamp: string
  }
}

/**
 * 系统消息事件
 */
export interface SystemMessageEvent {
  type: 'SystemMessage'
  payload: {
    taskId: string
    message: string
    level: 'info' | 'warning' | 'error'
    timestamp: string
  }
}

/**
 * 准备调用工具事件
 */
export interface ToolPreparingEvent {
  type: 'ToolPreparing'
  payload: {
    toolName: string
    confidence: number
  }
}

/**
 * 通用错误事件
 */
export interface ErrorEvent {
  type: 'Error'
  payload: {
    message: string
    recoverable: boolean
  }
}

// ===== 流式接口类型 =====

/**
 * 任务进度流接口
 *
 * 提供链式调用的事件监听API
 */
export interface TaskProgressStream {
  /**
   * 监听进度事件
   * @param callback 进度回调函数
   * @returns 流对象（支持链式调用）
   */
  onProgress(callback: (event: TaskProgressPayload) => void): TaskProgressStream

  /**
   * 监听错误事件
   * @param callback 错误回调函数
   * @returns 流对象（支持链式调用）
   */
  onError(callback: (error: Error) => void): TaskProgressStream

  /**
   * 监听流关闭事件
   * @param callback 关闭回调函数
   * @returns 流对象（支持链式调用）
   */
  onClose(callback: () => void): TaskProgressStream

  /**
   * 手动关闭流
   */
  close(): void

  /**
   * 流是否已关闭
   */
  readonly isClosed: boolean
}

// ===== 控制命令类型 =====

/**
 * 任务控制命令
 */
export type TaskControlCommand = PauseCommand | CancelCommand

/**
 * 暂停命令
 */
export interface PauseCommand {
  type: 'pause'
}

/**
 * 取消命令
 */
export interface CancelCommand {
  type: 'cancel'
  reason?: string
}

// ===== 查询过滤类型 =====

/**
 * 任务列表过滤条件
 */
export interface TaskListFilter {
  /** 会话ID过滤 */
  conversationId?: number
  /** 状态过滤 */
  status?: TaskStatus | string
  /** 分页偏移 */
  offset?: number
  /** 分页限制 */
  limit?: number
}

// ===== 错误处理类型 =====

/**
 * Agent API 错误类
 */
export class AgentApiError extends Error {
  public readonly code: string
  public readonly originalError?: unknown

  constructor(code: string, message: string, originalError?: unknown) {
    super(message)
    this.name = 'AgentApiError'
    this.code = code
    this.originalError = originalError
  }

  /**
   * 判断是否为特定类型的错误
   */
  is(code: string): boolean {
    return this.code === code
  }

  /**
   * 判断是否为可恢复的错误
   */
  isRecoverable(): boolean {
    const recoverableCodes = ['pause_failed', 'resume_failed', 'list_failed', 'stream_error']
    return recoverableCodes.includes(this.code)
  }
}

// ===== 工具类型 =====

/**
 * 事件类型守护函数
 */
export const isTaskProgressEvent = (event: any): event is TaskProgressPayload => {
  return event && typeof event === 'object' && 'type' in event && 'payload' in event
}

/**
 * 判断是否为终止事件
 */
export const isTerminalEvent = (event: TaskProgressPayload): boolean => {
  return (
    event.type === 'TaskCompleted' ||
    event.type === 'TaskCancelled' ||
    (event.type === 'TaskError' && !event.payload.isRecoverable)
  )
}

/**
 * 获取事件的任务ID
 */
export const getEventTaskId = (event: TaskProgressPayload): string => {
  if (event.type === 'ToolPreparing' || event.type === 'Error') {
    return '' // 这些事件没有taskId
  }
  const maybePayload = (event as any).payload as Record<string, unknown> | undefined
  if (maybePayload && typeof maybePayload === 'object' && 'taskId' in maybePayload) {
    return String((maybePayload as any).taskId)
  }
  return ''
}

/**
 * 判断是否为错误事件
 */
export const isErrorEvent = (event: TaskProgressPayload): boolean => {
  return event.type === 'TaskError' || event.type === 'Error'
}
