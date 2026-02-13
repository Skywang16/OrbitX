/**
 * Agent API 类型定义
 *
 * 定义Agent系统的所有接口类型，与后端TaskExecutor保持一致
 */

import type { TaskEvent } from '@/types'

// ===== 核心类型定义 =====

/**
 * 聊天模式类型
 */
export type ChatMode = 'chat' | 'agent'

/**
 * 任务执行参数
 */
export interface ExecuteTaskParams {
  /** 工作区路径 */
  workspacePath: string
  /** 会话ID */
  sessionId: number
  /** 用户提示 */
  userPrompt: string
  /** 模型ID - 必填！ */
  modelId: string
  /** 图片附件（可选） */
  images?: Array<{ type: 'image'; dataUrl: string; mimeType: string }>
}

/**
 * 任务摘要信息
 */
export interface TaskSummary {
  /** 任务ID */
  taskId: string
  /** 会话ID */
  sessionId: number
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
export type TaskProgressPayload = TaskEvent

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
export type TaskControlCommand = CancelCommand

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
  sessionId?: number
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

  const candidate = event as { type?: unknown }
  return typeof candidate.type === 'string'
}

/**
 * 判断是否为终止事件
 */
export const isTerminalEvent = (event: TaskProgressPayload): boolean => {
  return event.type === 'task_completed' || event.type === 'task_cancelled' || event.type === 'task_error'
}

/**
 * 获取事件的任务ID
 */
export const getEventTaskId = (event: TaskProgressPayload): string => {
  return 'taskId' in event && typeof event.taskId === 'string' ? event.taskId : ''
}

/**
 * 判断是否为错误事件
 */
export const isErrorEvent = (event: TaskProgressPayload): boolean => {
  return event.type === 'task_error'
}

/**
 * 文件上下文状态
 */
export interface FileContextStatus {
  workspacePath: string
  fileCount: number
  files: string[]
}
