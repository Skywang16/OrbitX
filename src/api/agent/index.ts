/**
 * Agent API - 后端Agent系统的前端接口封装
 *
 * 提供任务执行、状态管理、实时进度监听等功能
 */

import { invoke } from '@/utils/request'
import { agentChannelApi } from '@/api/channel/agent'
import type {
  CommandRenderResult,
  CommandSummary,
  ExecuteTaskParams,
  SkillSummary,
  SkillValidationResult,
  TaskListFilter,
  TaskProgressPayload,
  TaskProgressStream,
  TaskSummary,
} from './types'

/**
 * Agent API 主类
 *
 * 封装后端TaskExecutor的所有功能，提供类型安全的接口
 */
export class AgentApi {
  /**
   * 执行 Agent 任务
   * @param userPrompt 用户输入
   * @param sessionId 会话ID
   * @param chatMode 聊天模式 ('chat' | 'agent')
   * @param modelId 模型ID
   * @param images 图片附件（可选）
   * @returns 返回任务进度流
   */
  executeTask = async (params: ExecuteTaskParams): Promise<TaskProgressStream> => {
    const stream = agentChannelApi.createTaskStream(params)
    return this.createProgressStreamFromReadableStream(stream)
  }

  /**
   * 取消任务
   * @param taskId 任务ID
   * @param reason 取消原因
   */
  cancelTask = async (taskId: string, reason?: string): Promise<void> => {
    await invoke('agent_cancel_task', { taskId, reason })
  }

  confirmTool = async (requestId: string, decision: 'allow_once' | 'allow_always' | 'deny'): Promise<void> => {
    await invoke('agent_tool_confirm', {
      params: { requestId, decision },
    })
  }

  /**
   * 列出任务
   * @param filters 过滤条件
   * @returns 任务摘要列表
   */
  listTasks = async (filters?: TaskListFilter): Promise<TaskSummary[]> => {
    return await invoke<TaskSummary[]>('agent_list_tasks', {
      sessionId: filters?.sessionId,
      statusFilter: filters?.status,
    })
  }

  listCommands = async (workspacePath: string): Promise<CommandSummary[]> => {
    return await invoke<CommandSummary[]>('agent_list_commands', {
      params: { workspacePath },
    })
  }

  renderCommand = async (workspacePath: string, name: string, input: string): Promise<CommandRenderResult> => {
    return await invoke<CommandRenderResult>('agent_render_command', {
      params: { workspacePath, name, input },
    })
  }

  listSkills = async (workspacePath: string): Promise<SkillSummary[]> => {
    return await invoke<SkillSummary[]>('agent_list_skills', {
      params: { workspacePath },
    })
  }

  validateSkill = async (skillPath: string): Promise<SkillValidationResult> => {
    return await invoke<SkillValidationResult>('agent_validate_skill', {
      skillPath,
    })
  }

  /**
   * 获取任务详情
   * @param taskId 任务ID
   * @returns 任务详细信息
   */
  getTask = async (taskId: string): Promise<TaskSummary> => {
    const tasks = await this.listTasks()
    const task = tasks.find(t => t.taskId === taskId)

    if (!task) {
      throw new Error(`Task ${taskId} not found`)
    }

    return task
  }

  sendCommand = async (taskId: string, command: { type: 'cancel'; reason?: string }): Promise<void> => {
    await this.cancelTask(taskId, command.reason)
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

          // 打印Channel输出的内容（使用 warn 以符合 no-console 规则）
          console.warn('[Channel输出]', {
            type: value.type,
            data: value,
            timestamp: new Date().toISOString(),
          })

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
          errorCallbacks.forEach(callback => {
            try {
              callback(error as Error)
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
