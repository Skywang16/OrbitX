/**
 * 回调管理器实现
 *
 * 负责管理和触发所有的回调事件
 */

import type {
  ExecutionCallback,
  ProgressCallback,
  ProgressMessage,
  ICallbackManager,
  CallbackOptions,
} from '../types/callbacks'
import type { ExecutionEvent } from '../types/execution'

/**
 * 回调管理器实现
 */
export class CallbackManager implements ICallbackManager {
  private executionCallbacks: ExecutionCallback[] = []
  private progressCallbacks: ProgressCallback[] = []
  private options: CallbackOptions

  constructor(options: CallbackOptions = {}) {
    this.options = {
      enableVerboseLogging: false,
      enablePerformanceTracking: false,
      enableStackTrace: false,
      ...options,
    }
  }

  /**
   * 注册执行回调
   */
  onExecution(callback: ExecutionCallback): void {
    this.executionCallbacks.push(callback)
  }

  /**
   * 注册进度回调
   */
  onProgress(callback: ProgressCallback): void {
    this.progressCallbacks.push(callback)
  }

  /**
   * 触发执行事件
   */
  async triggerExecutionEvent(event: ExecutionEvent): Promise<void> {
    // 添加性能跟踪
    if (this.options.enablePerformanceTracking) {
      event.metadata = {
        ...event.metadata,
        performanceTimestamp: performance.now(),
      }
    }

    // 添加详细日志
    if (this.options.enableVerboseLogging) {
      console.log(`[CallbackManager] 触发执行事件: ${event.type}`, event)
    }

    // 并行执行所有回调
    const promises = this.executionCallbacks.map(async callback => {
      try {
        await callback(event)
      } catch (error) {
        console.error(`[CallbackManager] 执行回调失败:`, error)
        if (this.options.enableStackTrace) {
          console.error(error)
        }
      }
    })

    await Promise.allSettled(promises)
  }

  /**
   * 触发进度事件
   */
  triggerProgressEvent(message: ProgressMessage): void {
    // 添加时间戳
    if (!message.timestamp) {
      message.timestamp = new Date().toISOString()
    }

    // 添加详细日志
    if (this.options.enableVerboseLogging) {
      console.log(`[CallbackManager] 触发进度事件: ${message.type}`, message)
    }

    // 执行所有进度回调
    this.progressCallbacks.forEach(callback => {
      try {
        callback(message)
      } catch (error) {
        console.error(`[CallbackManager] 进度回调失败:`, error)
        if (this.options.enableStackTrace) {
          console.error(error)
        }
      }
    })
  }

  /**
   * 清理所有回调
   */
  clear(): void {
    this.executionCallbacks = []
    this.progressCallbacks = []
  }

  /**
   * 获取回调统计信息
   */
  getStats() {
    return {
      executionCallbacks: this.executionCallbacks.length,
      progressCallbacks: this.progressCallbacks.length,
      options: this.options,
    }
  }

  /**
   * 更新配置选项
   */
  updateOptions(options: Partial<CallbackOptions>): void {
    this.options = { ...this.options, ...options }
  }
}

/**
 * 全局回调管理器实例
 */
export const globalCallbackManager = new CallbackManager()
