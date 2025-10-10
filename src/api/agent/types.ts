/**
 * Agent API 类型定义
 *
 * 定义Agent系统的所有接口类型，与后端TaskExecutor保持一致
 */

// ===== 核心类型定义 =====

/**
 * 聊天模式类型
 */
export type ChatMode = 'chat' | 'agent'

/**
 * 任务执行参数
 */
export interface ExecuteTaskParams {
  /** 会话ID */
  conversationId: number
  /** 用户提示 */
  userPrompt: string
  /** 聊天模式 - 必填！类型系统强制传递 */
  chatMode: ChatMode
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
 * 文本流事件
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
 * 结束事件
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

// ===== 工具类型 =====

/**
 * 事件类型守护函数
 */
export const isTaskProgressEvent = (event: unknown): event is TaskProgressPayload => {
  if (!event || typeof event !== 'object') {
    return false
  }

  const candidate = event as { type?: unknown; payload?: unknown }
  return typeof candidate.type === 'string' && candidate.payload !== undefined
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
  const payload = event.payload as Record<string, unknown>
  if (payload && typeof payload === 'object' && 'taskId' in payload) {
    const taskId = payload.taskId
    if (typeof taskId === 'string') {
      return taskId
    }
    if (typeof taskId === 'number') {
      return String(taskId)
    }
  }
  return ''
}

/**
 * 判断是否为错误事件
 */
export const isErrorEvent = (event: TaskProgressPayload): boolean => {
  return event.type === 'TaskError' || event.type === 'Error'
}

// ===== 双轨架构新增类型 =====

/**
 * 会话上下文快照（核心轨）
 */
export interface ConversationContextSnapshot {
  conversation: Conversation
  summary?: ConversationSummary
  activeTaskIds: string[]
  executions: ExecutionSnapshot[]
}

/**
 * 会话信息
 */
export interface Conversation {
  id: number
  title?: string
  workspacePath?: string
  createdAt: string
  updatedAt: string
}

/**
 * 会话摘要
 */
export interface ConversationSummary {
  conversationId: number
  summaryContent: string
  summaryTokens: number
  messagesBeforeSummary: number
  tokensSaved: number
  compressionCost: number
  createdAt: string
  updatedAt: string
}

/**
 * 执行快照
 */
export interface ExecutionSnapshot {
  executionId: string
  conversationId: number
  userRequest: string
  status: 'running' | 'completed' | 'error' | 'cancelled'
  currentIteration: number
  errorCount: number
  maxIterations: number
  totalInputTokens: number
  totalOutputTokens: number
  totalCost: number
  contextTokens: number
  createdAt: string
  updatedAt: string
  startedAt?: string
  completedAt?: string
}

/**
 * UI 时间线快照（UI轨）
 */
export type UiStepType = 'thinking' | 'text' | 'tool_use' | 'tool_result' | 'error'

export interface UiStep {
  stepType: UiStepType
  content: string
  timestamp: number
  metadata?: Record<string, unknown>
}

export interface UiMessage {
  id: number
  conversationId: number
  role: 'user' | 'assistant'
  content?: string
  steps?: UiStep[]
  status?: 'streaming' | 'complete' | 'error'
  durationMs?: number
  createdAt: number
}

export interface UiConversation {
  id: number
  title?: string
  messageCount: number
  createdAt: number
  updatedAt: number
}

/**
 * 文件上下文状态
 */
export interface FileContextStatus {
  conversationId: number
  activeFiles: FileContextEntry[]
  staleFiles: FileContextEntry[]
  totalActive: number
  totalStale: number
}

/**
 * 文件上下文条目
 */
export interface FileContextEntry {
  id: number
  conversationId: number
  filePath: string
  recordState: 'active' | 'stale'
  recordSource: 'read_tool' | 'user_edited' | 'agent_edited' | 'file_mentioned'
  agentReadTimestamp?: number
  agentEditTimestamp?: number
  userEditTimestamp?: number
  createdAt: string
}
