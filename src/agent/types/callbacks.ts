/**
 * 回调系统类型定义
 *
 * 定义了Agent框架中所有的回调相关接口和类型
 */

import type { ExecutionEvent } from './execution'

/**
 * 执行回调函数类型
 */
export type ExecutionCallback = (event: ExecutionEvent) => Promise<void>

/**
 * 进度回调消息
 */
export interface ProgressMessage {
  type: 'thinking' | 'planning' | 'working' | 'progress' | 'complete' | 'error'
  content: string
  data?: unknown
  timestamp?: string
}

/**
 * 进度回调函数类型
 */
export type ProgressCallback = (message: ProgressMessage) => void

/**
 * 流式消息回调函数类型
 */
export type StreamCallback = (message: ProgressMessage & { timestamp: string }) => Promise<void>

/**
 * 回调事件类型枚举
 */
export enum CallbackEventType {
  // 工作流级别事件
  WORKFLOW_START = 'workflow_start',
  WORKFLOW_COMPLETED = 'workflow_completed',
  WORKFLOW_FAILED = 'workflow_failed',

  // Agent级别事件
  AGENT_START = 'agent_start',
  AGENT_COMPLETED = 'agent_completed',
  AGENT_FAILED = 'agent_failed',

  // 工具级别事件
  TOOL_START = 'tool_start',
  TOOL_COMPLETED = 'tool_completed',
  TOOL_FAILED = 'tool_failed',

  // 进度事件
  PROGRESS_UPDATE = 'progress_update',
  THINKING = 'thinking',
  PLANNING = 'planning',
  WORKING = 'working',
}

/**
 * 回调配置选项
 */
export interface CallbackOptions {
  /** 是否启用详细日志 */
  enableVerboseLogging?: boolean

  /** 是否启用性能监控 */
  enablePerformanceTracking?: boolean

  /** 是否启用错误堆栈跟踪 */
  enableStackTrace?: boolean

  /** 自定义元数据 */
  metadata?: Record<string, unknown>
}

/**
 * 回调管理器接口
 */
export interface ICallbackManager {
  /**
   * 注册执行回调
   */
  onExecution(callback: ExecutionCallback): void

  /**
   * 注册进度回调
   */
  onProgress(callback: ProgressCallback): void

  /**
   * 触发执行事件
   */
  triggerExecutionEvent(event: ExecutionEvent): Promise<void>

  /**
   * 触发进度事件
   */
  triggerProgressEvent(message: ProgressMessage): void

  /**
   * 清理所有回调
   */
  clear(): void
}
