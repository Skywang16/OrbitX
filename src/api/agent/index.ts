/**
 * Agent API - 后端Agent系统的前端接口封装
 *
 * 提供任务执行、状态管理、实时进度监听等功能
 * 按照设计文档 agent-backend-migration/design.md 实现
 */

import { invoke } from '@/utils/request'
import { agentChannelApi } from '@/api/channel/agent'
import type {
  ExecuteTaskParams,
  TaskProgressPayload,
  TaskSummary,
  TaskProgressStream,
  TaskListFilter,
  TaskControlCommand,
} from './types'
import { AgentApiError } from './types'

/**
 * Agent API 主类
 *
 * 封装后端TaskExecutor的所有功能，提供类型安全的接口
 */
export class AgentApi {
  /**
   * 执行Agent任务
   * @param params 任务执行参数
   * @returns 返回任务进度流
   */
  async executeTask(params: ExecuteTaskParams): Promise<TaskProgressStream> {
    try {
      console.warn('🔌 调用Agent Channel API executeTask:', params)

      const stream = agentChannelApi.createTaskStream(params)

      console.warn('🔌 Agent Channel流创建成功')

      return this.createProgressStreamFromReadableStream(stream)
    } catch (error) {
      console.error('❌ Agent Channel调用失败:', error)
      throw this.transformError(error, 'execute_task')
    }
  }

  /**
   * 暂停正在执行的任务
   * @param taskId 任务ID
   */
  async pauseTask(taskId: string): Promise<void> {
    try {
      await invoke('agent_pause_task', { taskId })
    } catch (error) {
      throw this.transformError(error, 'pause_task')
    }
  }

  /**
   * 恢复已暂停的任务
   * @param taskId 任务ID
   * @returns 返回任务进度流
   */
  async resumeTask(taskId: string): Promise<TaskProgressStream> {
    try {
      const stream = agentChannelApi.createResumeStream(taskId)
      return this.createProgressStreamFromReadableStream(stream)
    } catch (error) {
      throw this.transformError(error, 'resume_task')
    }
  }

  /**
   * 取消任务
   * @param taskId 任务ID
   * @param reason 取消原因
   */
  async cancelTask(taskId: string, reason?: string): Promise<void> {
    try {
      await invoke('agent_cancel_task', { taskId, reason })
    } catch (error) {
      throw this.transformError(error, 'cancel_task')
    }
  }

  /**
   * 列出任务
   * @param filters 过滤条件
   * @returns 任务摘要列表
   */
  async listTasks(filters?: TaskListFilter): Promise<TaskSummary[]> {
    try {
      return await invoke<TaskSummary[]>('agent_list_tasks', {
        conversationId: filters?.conversationId,
        statusFilter: filters?.status,
      })
    } catch (error) {
      throw this.transformError(error, 'list_tasks')
    }
  }

  /**
   * 获取任务详情
   * @param taskId 任务ID
   * @returns 任务详细信息
   */
  async getTask(taskId: string): Promise<TaskSummary> {
    const tasks = await this.listTasks()
    const task = tasks.find(t => t.taskId === taskId)

    if (!task) {
      throw new AgentApiError('task_not_found', `Task ${taskId} not found`)
    }

    return task
  }

  /**
   * 发送任务控制命令
   * @param taskId 任务ID
   * @param command 控制命令
   */
  async sendCommand(taskId: string, command: TaskControlCommand): Promise<void> {
    switch (command.type) {
      case 'pause':
        await this.pauseTask(taskId)
        break
      case 'cancel':
        await this.cancelTask(taskId, command.reason)
        break
      default: {
        const _exhaustiveCheck: never = command
        throw new AgentApiError(
          'invalid_command',
          `Unsupported command: ${(_exhaustiveCheck as TaskControlCommand).type}`
        )
      }
    }
  }

  /**
   * 从ReadableStream创建任务进度流
   * @private
   * @param stream ReadableStream
   * @returns TaskProgressStream
   */
  private createProgressStreamFromReadableStream(stream: ReadableStream<TaskProgressPayload>): TaskProgressStream {
    let isClosed = false
    const callbacks: Array<(event: TaskProgressPayload) => void> = []
    const errorCallbacks: Array<(error: Error) => void> = []
    const closeCallbacks: Array<() => void> = []
    let reader: ReadableStreamDefaultReader<TaskProgressPayload> | null = null
    // 用于在还没有任何订阅者时暂存事件，避免丢失 TaskCreated 等早期事件
    const pendingEvents: TaskProgressPayload[] = []

    const startReading = async () => {
      try {
        reader = stream.getReader()

        while (!isClosed) {
          const { done, value } = await reader.read()

          if (done || isClosed) {
            closeStream()
            break
          }

          if (callbacks.length === 0) {
            // 尚无订阅者，暂存事件
            pendingEvents.push(value)
          } else {
            // 通知所有监听器
            callbacks.forEach(callback => {
              try {
                callback(value)
              } catch (error) {
                console.error('[AgentApi] 进度回调错误:', error)
              }
            })
          }
        }
      } catch (error) {
        if (!isClosed) {
          const agentError = this.transformError(error, 'stream_error')
          errorCallbacks.forEach(callback => {
            try {
              callback(agentError)
            } catch (err) {
              console.error('[AgentApi] 错误回调错误:', err)
            }
          })
          closeStream()
        }
      }
    }

    const closeStream = () => {
      if (isClosed) return
      isClosed = true

      if (reader) {
        reader.cancel().catch(console.error)
        reader = null
      }

      closeCallbacks.forEach(callback => {
        try {
          callback()
        } catch (error) {
          console.error('[AgentApi] 关闭回调错误:', error)
        }
      })

      // 清理回调数组
      callbacks.length = 0
      errorCallbacks.length = 0
      closeCallbacks.length = 0
    }

    // 立即开始读取流，防止 ReadableStream 上游缓冲溢出；若此时无订阅者，会先暂存到 pendingEvents
    startReading()

    // 创建流对象
    const taskProgressStream: TaskProgressStream = {
      onProgress: callback => {
        if (!isClosed) {
          callbacks.push(callback)
          // 首次有订阅者时，立即把 pendingEvents 回放给当前订阅者
          if (pendingEvents.length > 0) {
            try {
              for (const ev of pendingEvents.splice(0, pendingEvents.length)) {
                callback(ev)
              }
            } catch (error) {
              console.error('[AgentApi] 回放暂存事件错误:', error)
            }
          }
        }
        return taskProgressStream
      },

      onError: callback => {
        if (!isClosed) {
          errorCallbacks.push(callback)
        }
        return taskProgressStream
      },

      onClose: callback => {
        if (!isClosed) {
          closeCallbacks.push(callback)
        } else {
          callback()
        }
        return taskProgressStream
      },

      close: () => {
        closeStream()
      },

      get isClosed() {
        return isClosed
      },
    }

    return taskProgressStream
  }

  /**
   * 转换错误为AgentApiError
   * @private
   * @param error 原始错误
   * @param operation 操作名称
   * @returns AgentApiError
   */
  private transformError(error: unknown, operation: string): AgentApiError {
    if (error instanceof AgentApiError) {
      return error
    }

    let message = 'Unknown error'
    let code = 'unknown_error'

    if (typeof error === 'string') {
      message = error
    } else if (error instanceof Error) {
      message = error.message

      // 解析Tauri API错误
      if (message.includes('agent.')) {
        const parts = message.split('.')
        if (parts.length >= 2) {
          code = parts[1]
          message = this.getErrorMessage(code)
        }
      }
    } else if (typeof error === 'object' && error !== null) {
      const errorObj = error as Record<string, unknown>

      if (typeof errorObj.message === 'string') {
        message = errorObj.message
      }

      if (typeof errorObj.code === 'string') {
        code = errorObj.code
      }
    }

    return new AgentApiError(code, `${operation}: ${message}`, error)
  }

  /**
   * 获取错误消息
   * @private
   * @param code 错误代码
   * @returns 错误消息
   */
  private getErrorMessage(code: string): string {
    const errorMessages: Record<string, string> = {
      execute_failed: '任务执行失败',
      pause_failed: '任务暂停失败',
      resume_failed: '任务恢复失败',
      cancel_failed: '任务取消失败',
      list_failed: '任务列表获取失败',
      task_not_found: '任务不存在',
      invalid_params: '参数无效',
      invalid_command: '命令无效',
      stream_error: '进度流错误',
      unknown_error: '未知错误',
    }

    return errorMessages[code] || `错误: ${code}`
  }
}

/**
 * Agent API 单例实例
 */
export const agentApi = new AgentApi()

/**
 * 导出类型
 */
export * from './types'

/**
 * 前端扩展类型
 */
export interface AgentTaskState extends TaskSummary {
  /** 是否正在监听进度 */
  isListening?: boolean
  /** 最后更新时间 */
  lastUpdated?: Date
  /** 进度流引用 */
  progressStream?: TaskProgressStream
  /** 最近的进度事件 */
  recentEvents?: TaskProgressPayload[]
  /** 错误信息 */
  error?: string
}

/**
 * 默认导出
 */
export default agentApi
