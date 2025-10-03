/**
 * Agent API - 后端Agent系统的前端接口封装
 *
 * 提供任务执行、状态管理、实时进度监听等功能
 */

import { invoke } from '@/utils/request'
import { agentChannelApi } from '@/api/channel/agent'
import type {
  ExecuteTaskParams,
  TaskControlCommand,
  TaskListFilter,
  TaskProgressPayload,
  TaskProgressStream,
  TaskSummary,
  UiConversation,
  UiMessage,
} from './types'
import type { Conversation as ChatConversation, Message } from '@/types'

/**
 * Agent API 主类
 *
 * 封装后端TaskExecutor的所有功能，提供类型安全的接口
 */
export class AgentApi {
  /**
   * 执行Agent任务
   * @param userPrompt 用户输入
   * @param conversationId 会话ID
   * @returns 返回任务进度流
   */
  async executeTask(userPrompt: string, conversationId: number): Promise<TaskProgressStream> {
    const params: ExecuteTaskParams = {
      conversationId,
      userPrompt,
    }

    const stream = agentChannelApi.createTaskStream(params)

    return this.createProgressStreamFromReadableStream(stream)
  }

  /**
   * 暂停正在执行的任务
   * @param taskId 任务ID
   */
  async pauseTask(taskId: string): Promise<void> {
    await invoke('agent_pause_task', { taskId })
  }

  /**
   * 恢复已暂停的任务
   * @param taskId 任务ID
   * @returns 返回任务进度流
   */
  async resumeTask(taskId: string): Promise<TaskProgressStream> {
    const stream = agentChannelApi.createResumeStream(taskId)

    return this.createProgressStreamFromReadableStream(stream)
  }

  /**
   * 取消任务
   * @param taskId 任务ID
   * @param reason 取消原因
   */
  async cancelTask(taskId: string, reason?: string): Promise<void> {
    await invoke('agent_cancel_task', { taskId, reason })
  }

  /**
   * 列出任务
   * @param filters 过滤条件
   * @returns 任务摘要列表
   */
  async listTasks(filters?: TaskListFilter): Promise<TaskSummary[]> {
    return await invoke<TaskSummary[]>('agent_list_tasks', {
      conversationId: filters?.conversationId,
      statusFilter: filters?.status,
    })
  }

  // === 双轨架构新增方法 ===

  /**
   * 创建新会话
   * @param title 会话标题
   * @param workspacePath 工作空间路径
   * @returns 会话ID
   */
  async createConversation(title?: string, workspacePath?: string): Promise<number> {
    return await invoke<number>('agent_create_conversation', { title, workspacePath })
  }

  /**
   * 获取会话列表
   * @param limit 限制数量
   * @param offset 偏移量
   * @returns 会话列表
   */
  async listConversations(): Promise<ChatConversation[]> {
    const conversations = await invoke<UiConversation[]>('agent_ui_get_conversations')
    return conversations.map(record => this.convertUiConversation(record))
  }

  /**
   * 删除会话
   * @param conversationId 会话ID
   */
  async deleteConversation(conversationId: number): Promise<void> {
    await invoke('agent_delete_conversation', { conversationId })
  }

  /**
   * 更新会话标题
   * @param conversationId 会话ID
   * @param title 新标题
   */
  async updateConversationTitle(conversationId: number, title: string): Promise<void> {
    await invoke('agent_update_conversation_title', { conversationId, title })
  }

  /** 获取单个会话 */
  async getConversation(conversationId: number): Promise<ChatConversation> {
    const conversations = await this.listConversations()
    const target = conversations.find(convo => convo.id === conversationId)
    if (!target) {
      throw new Error(`Conversation ${conversationId} not found`)
    }
    return target
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
      throw new Error(`Task ${taskId} not found`)
    }

    return task
  }

  /**
   * 获取会话消息（UI轨）
   */
  async getMessages(conversationId: number): Promise<Message[]> {
    const uiMessages = await invoke<UiMessage[]>('agent_ui_get_messages', {
      conversationId,
    })
    return uiMessages.map(record => this.convertUiMessage(record))
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
        throw new Error(`Unsupported command: ${(_exhaustiveCheck as TaskControlCommand).type}`)
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

          // 打印Channel输出的内容
          console.log('[Channel输出]', {
            type: value.type,
            payload: value.payload,
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

  private convertUiMessage(message: UiMessage): Message {
    const toDate = (timestamp: number) => new Date(timestamp * 1000)
    const base: Message = {
      id: message.id,
      conversationId: message.conversationId,
      role: message.role,
      createdAt: toDate(message.createdAt),
      status: message.status ?? (message.role === 'assistant' ? 'streaming' : undefined),
      duration: message.durationMs ?? undefined,
    }

    if (message.role === 'user') {
      return {
        ...base,
        content: message.content,
      }
    }

    return {
      ...base,
      steps: message.steps || [],
    }
  }

  private convertUiConversation(record: UiConversation): ChatConversation {
    const toDate = (timestamp: number) => new Date(timestamp * 1000)
    return {
      id: record.id,
      title: record.title ?? '',
      messageCount: record.messageCount,
      createdAt: toDate(record.createdAt),
      updatedAt: toDate(record.updatedAt),
    }
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
