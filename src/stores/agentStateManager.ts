/**
 * Agent状态同步管理器
 *
 * 负责管理后端Agent任务的前端状态同步，包括：
 * - 任务状态实时监听
 * - UI状态更新
 * - 断线重连处理
 * - 状态持久化
 */

import { defineStore } from 'pinia'
import { ref, computed, readonly, watch } from 'vue'
import { agentApi } from '@/api/agent'
import type { TaskSummary, TaskProgressPayload, TaskProgressStream } from '@/api/agent/types'
import { AgentApiError } from '@/api/agent/types'

/**
 * Agent任务状态信息（前端扩展）
 */
export interface AgentTaskState extends TaskSummary {
  /** 是否正在监听进度 */
  isListening: boolean
  /** 最后更新时间 */
  lastUpdated: Date
  /** 进度流引用 */
  progressStream?: TaskProgressStream
  /** 最近的进度事件 */
  recentEvents: TaskProgressPayload[]
  /** 错误信息 */
  error?: string
}

/**
 * 连接状态
 */
export type ConnectionStatus = 'connected' | 'disconnected' | 'reconnecting' | 'error'

/**
 * Agent状态管理Store
 */
export const useAgentStateManager = defineStore('agent-state-manager', () => {
  // ===== 状态定义 =====

  /** 所有任务状态映射 task_id -> AgentTaskState */
  const tasks = ref<Map<string, AgentTaskState>>(new Map())

  /** 当前活跃会话ID */
  const currentConversationId = ref<number | null>(null)

  /** 连接状态 */
  const connectionStatus = ref<ConnectionStatus>('disconnected')

  /** 是否已初始化 */
  const isInitialized = ref(false)

  /** 重连计时器 */
  const reconnectTimer = ref<NodeJS.Timeout | null>(null)

  /** 重连尝试次数 */
  const reconnectAttempts = ref(0)

  /** 最大重连尝试次数 */
  const maxReconnectAttempts = 5

  /** 重连间隔（毫秒） */
  const reconnectInterval = 3000

  /** 全局错误信息 */
  const globalError = ref<string | null>(null)

  /** 严格单通道模式：仅通过任务事件通道驱动渲染，不主动调用 listTasks 等接口 */
  const strictChannelMode = ref(true)

  // ===== 计算属性 =====

  /** 当前会话的任务列表 */
  const currentTasks = computed(() => {
    if (!currentConversationId.value) return []

    return Array.from(tasks.value.values())
      .filter(task => task.conversationId === currentConversationId.value)
      .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime())
  })

  /** 活跃任务列表 */
  const activeTasks = computed(() => {
    return currentTasks.value.filter(task => task.status === 'running' || task.status === 'paused')
  })

  /** 已完成任务列表 */
  const completedTasks = computed(() => {
    return currentTasks.value.filter(task => task.status === 'completed')
  })

  /** 错误任务列表 */
  const errorTasks = computed(() => {
    return currentTasks.value.filter(task => task.status === 'error')
  })

  /** 是否有活跃任务 */
  const hasActiveTasks = computed(() => activeTasks.value.length > 0)

  /** 连接状态描述 */
  const connectionStatusText = computed(() => {
    switch (connectionStatus.value) {
      case 'connected':
        return '已连接'
      case 'disconnected':
        return '未连接'
      case 'reconnecting':
        return `重连中 (${reconnectAttempts.value}/${maxReconnectAttempts})`
      case 'error':
        return '连接错误'
      default:
        return '未知状态'
    }
  })

  // ===== 核心方法 =====

  /**
   * 初始化状态管理器
   */
  const initialize = async (): Promise<void> => {
    if (isInitialized.value) return

    try {
      connectionStatus.value = 'connected'
      isInitialized.value = true

      // 清除之前的重连定时器
      if (reconnectTimer.value) {
        clearTimeout(reconnectTimer.value)
        reconnectTimer.value = null
      }

      console.log('🤖 Agent状态管理器已初始化')
    } catch (error) {
      console.error('❌ Agent状态管理器初始化失败:', error)
      connectionStatus.value = 'error'
      globalError.value = error instanceof Error ? error.message : '初始化失败'
    }
  }

  /**
   * 切换到指定会话
   */
  const switchToConversation = async (conversationId: number): Promise<void> => {
    try {
      currentConversationId.value = conversationId

      // 停止旧会话的所有任务监听
      await stopAllTaskListening()

      // 在严格单通道模式下，不主动拉取任务列表
      if (!strictChannelMode.value) {
        await refreshTasks()
      }

      console.log('🔄 已切换到会话:', conversationId)
    } catch (error) {
      console.error('❌ 切换会话失败:', error)
      handleError(error, 'switch_conversation')
    }
  }

  /**
   * 刷新任务列表
   */
  const refreshTasks = async (): Promise<void> => {
    if (!currentConversationId.value) return

    try {
      const taskList = await agentApi.listTasks({
        conversationId: currentConversationId.value,
      })

      // 更新任务状态
      for (const taskSummary of taskList) {
        const existingTask = tasks.value.get(taskSummary.taskId)

        const taskState: AgentTaskState = {
          ...taskSummary,
          isListening: existingTask?.isListening || false,
          lastUpdated: new Date(),
          progressStream: existingTask?.progressStream,
          recentEvents: existingTask?.recentEvents || [],
          error: existingTask?.error,
        }

        tasks.value.set(taskSummary.taskId, taskState)
      }

      connectionStatus.value = 'connected'
      reconnectAttempts.value = 0
      globalError.value = null

      console.log('📋 任务列表已刷新, 任务数量:', taskList.length)
    } catch (error) {
      console.error('❌ 刷新任务列表失败:', error)
      handleError(error, 'refresh_tasks')
    }
  }

  /**
   * 开始监听任务进度
   */
  const startTaskListening = async (taskId: string): Promise<TaskProgressStream | null> => {
    try {
      const task = tasks.value.get(taskId)
      if (!task) {
        console.warn('⚠️ 任务不存在:', taskId)
        return null
      }

      if (task.isListening && task.progressStream) {
        console.log('🔄 任务已在监听中:', taskId)
        return task.progressStream
      }

      // 根据任务状态选择合适的操作
      let stream: TaskProgressStream

      if (task.status === 'paused') {
        stream = await agentApi.resumeTask(taskId)
      } else if (task.status === 'running') {
        // 如果任务正在运行，尝试重新连接到进度流
        // 这里可能需要后端支持获取运行中任务的进度流
        console.warn('⚠️ 任务正在运行，但无法重连到进度流:', taskId)
        return null
      } else {
        console.warn('⚠️ 任务状态不支持监听:', task.status, taskId)
        return null
      }

      // 设置进度监听
      stream
        .onProgress(event => handleTaskProgress(taskId, event))
        .onError(error => handleTaskError(taskId, error))
        .onClose(() => handleTaskClose(taskId))

      // 更新任务状态
      task.isListening = true
      task.progressStream = stream
      task.error = undefined
      tasks.value.set(taskId, task)

      console.log('🎧 开始监听任务进度:', taskId)
      return stream
    } catch (error) {
      console.error('❌ 开始任务监听失败:', error)
      handleError(error, 'start_listening')
      return null
    }
  }

  /**
   * 停止监听任务进度
   */
  const stopTaskListening = async (taskId: string): Promise<void> => {
    try {
      const task = tasks.value.get(taskId)
      if (!task || !task.isListening) return

      if (task.progressStream) {
        task.progressStream.close()
      }

      task.isListening = false
      task.progressStream = undefined
      tasks.value.set(taskId, task)

      console.log('🛑 停止监听任务进度:', taskId)
    } catch (error) {
      console.error('❌ 停止任务监听失败:', error)
    }
  }

  /**
   * 停止所有任务监听
   */
  const stopAllTaskListening = async (): Promise<void> => {
    const listeningTasks = Array.from(tasks.value.entries()).filter(([_, task]) => task.isListening)

    for (const [taskId] of listeningTasks) {
      await stopTaskListening(taskId)
    }
  }

  /**
   * 执行新任务
   */
  const executeTask = async (userPrompt: string, conversationId?: number): Promise<TaskProgressStream | null> => {
    try {
      const targetConversationId = conversationId || currentConversationId.value
      if (!targetConversationId) {
        throw new Error('未选择会话')
      }

      console.warn('📡 调用agentApi.executeTask:', { userPrompt, conversationId: targetConversationId })

      const stream = await agentApi.executeTask({
        conversationId: targetConversationId,
        userPrompt,
      })

      console.warn('📡 agentApi.executeTask返回stream:', !!stream)

      // 监听第一个TaskCreated事件来获取任务ID
      let taskId: string | null = null

      stream.onProgress(event => {
        if (event.type === 'TaskCreated') {
          taskId = event.payload.taskId

          // 创建新的任务状态
          const newTask: AgentTaskState = {
            taskId: taskId,
            conversationId: targetConversationId,
            status: 'created',
            currentIteration: 0,
            errorCount: 0,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            userPrompt,
            isListening: true,
            lastUpdated: new Date(),
            progressStream: stream,
            recentEvents: [event],
          }

          tasks.value.set(taskId!, newTask)

          // 继续监听后续进度
          handleTaskProgress(taskId!, event)
        } else if (taskId) {
          handleTaskProgress(taskId!, event)
        }
      })

      stream
        .onError(error => {
          if (taskId) {
            handleTaskError(taskId!, error)
          } else {
            handleError(error, 'execute_task')
          }
        })
        .onClose(() => {
          if (taskId) {
            handleTaskClose(taskId!)
          }
        })

      return stream
    } catch (error) {
      console.error('❌ 执行任务失败:', error)
      handleError(error, 'execute_task')
      return null
    }
  }

  /**
   * 处理任务进度事件
   */
  const handleTaskProgress = (taskId: string, event: TaskProgressPayload): void => {
    const task = tasks.value.get(taskId)
    if (!task) return

    // 更新任务状态
    if (event.type === 'StatusChanged') {
      task.status = event.payload.status
    } else if (event.type === 'StatusUpdate') {
      task.currentIteration = event.payload.currentIteration
      task.errorCount = event.payload.errorCount
    } else if (event.type === 'TaskCompleted') {
      task.status = 'completed'
      task.currentIteration = event.payload.finalIteration
    } else if (event.type === 'TaskError') {
      task.status = 'error'
      task.error = event.payload.errorMessage
    } else if (event.type === 'TaskCancelled') {
      task.status = 'cancelled'
    }

    // 添加到最近事件（保持最近20个事件）
    task.recentEvents.unshift(event)
    if (task.recentEvents.length > 20) {
      task.recentEvents = task.recentEvents.slice(0, 20)
    }

    task.lastUpdated = new Date()
    tasks.value.set(taskId, task)

    // 重置连接状态
    if (connectionStatus.value !== 'connected') {
      connectionStatus.value = 'connected'
      reconnectAttempts.value = 0
      globalError.value = null
    }
  }

  /**
   * 处理任务错误
   */
  const handleTaskError = (taskId: string, error: Error): void => {
    const task = tasks.value.get(taskId)
    if (task) {
      task.error = error.message
      task.lastUpdated = new Date()

      // 如果是API错误且可恢复，尝试重连
      if (error instanceof AgentApiError && error.isRecoverable()) {
        task.isListening = false
        task.progressStream = undefined

        // 启动重连逻辑
        scheduleReconnect(taskId)
      } else {
        task.status = 'error'
        task.isListening = false
        task.progressStream = undefined
      }

      tasks.value.set(taskId, task)
    }

    handleError(error, 'task_progress')
  }

  /**
   * 处理任务流关闭
   */
  const handleTaskClose = (taskId: string): void => {
    const task = tasks.value.get(taskId)
    if (task) {
      task.isListening = false
      task.progressStream = undefined
      task.lastUpdated = new Date()
      tasks.value.set(taskId, task)
    }

    console.log('🔌 任务进度流已关闭:', taskId)
  }

  /**
   * 处理全局错误
   */
  const handleError = (error: unknown, context: string): void => {
    const errorMessage = error instanceof Error ? error.message : String(error)

    console.error(`❌ Agent状态管理错误 [${context}]:`, error)

    // 根据错误类型决定连接状态
    if (error instanceof AgentApiError) {
      if (error.isRecoverable()) {
        connectionStatus.value = 'error'
        scheduleGlobalReconnect()
      } else {
        connectionStatus.value = 'error'
        globalError.value = errorMessage
      }
    } else {
      connectionStatus.value = 'error'
      globalError.value = errorMessage
    }
  }

  /**
   * 安排任务重连
   */
  const scheduleReconnect = (taskId: string): void => {
    if (reconnectAttempts.value >= maxReconnectAttempts) {
      console.error('❌ 任务重连次数已达上限:', taskId)
      return
    }

    reconnectAttempts.value++
    connectionStatus.value = 'reconnecting'

    reconnectTimer.value = setTimeout(async () => {
      try {
        console.log('🔄 尝试重连任务:', taskId)
        await startTaskListening(taskId)
      } catch (error) {
        console.error('❌ 任务重连失败:', error)
        if (reconnectAttempts.value < maxReconnectAttempts) {
          scheduleReconnect(taskId)
        } else {
          connectionStatus.value = 'error'
          globalError.value = '任务重连失败'
        }
      }
    }, reconnectInterval)
  }

  /**
   * 安排全局重连
   */
  const scheduleGlobalReconnect = (): void => {
    if (reconnectAttempts.value >= maxReconnectAttempts) {
      console.error('❌ 全局重连次数已达上限')
      return
    }

    reconnectAttempts.value++
    connectionStatus.value = 'reconnecting'

    reconnectTimer.value = setTimeout(async () => {
      try {
        console.log('🔄 尝试全局重连')
        if (!strictChannelMode.value) {
          await refreshTasks()
        }

        // 重启活跃任务的监听（严格模式下也仅通过已有任务事件流恢复）
        for (const task of activeTasks.value) {
          if (!task.isListening) {
            await startTaskListening(task.taskId)
          }
        }
      } catch (error) {
        console.error('❌ 全局重连失败:', error)
        if (reconnectAttempts.value < maxReconnectAttempts) {
          scheduleGlobalReconnect()
        } else {
          connectionStatus.value = 'error'
          globalError.value = '连接重试失败'
        }
      }
    }, reconnectInterval)
  }

  /**
   * 手动重连
   */
  const manualReconnect = async (): Promise<void> => {
    reconnectAttempts.value = 0
    globalError.value = null

    if (reconnectTimer.value) {
      clearTimeout(reconnectTimer.value)
      reconnectTimer.value = null
    }

    if (!strictChannelMode.value) {
      await refreshTasks()
    }
  }

  /**
   * 获取任务详情
   */
  const getTask = (taskId: string): AgentTaskState | undefined => {
    return tasks.value.get(taskId)
  }

  /**
   * 清理资源
   */
  const cleanup = async (): Promise<void> => {
    if (reconnectTimer.value) {
      clearTimeout(reconnectTimer.value)
      reconnectTimer.value = null
    }

    await stopAllTaskListening()

    tasks.value.clear()
    currentConversationId.value = null
    connectionStatus.value = 'disconnected'
    isInitialized.value = false
    reconnectAttempts.value = 0
    globalError.value = null
  }

  // ===== 监听器 =====

  /**
   * 暂停任务
   */
  const pauseTask = async (taskId: string): Promise<boolean> => {
    try {
      await agentApi.pauseTask(taskId)

      // 更新本地状态
      const task = tasks.value.get(taskId)
      if (task) {
        task.status = 'paused'
        task.lastUpdated = new Date()
        tasks.value.set(taskId, task)
      }

      console.log('⏸️ 任务已暂停:', taskId)
      return true
    } catch (error) {
      console.error('❌ 暂停任务失败:', error)
      handleTaskError(taskId, error as Error)
      return false
    }
  }

  /**
   * 恢复任务
   */
  const resumeTask = async (taskId: string): Promise<TaskProgressStream | null> => {
    try {
      const stream = await agentApi.resumeTask(taskId)

      // 更新本地状态
      const task = tasks.value.get(taskId)
      if (task) {
        task.status = 'running'
        task.isListening = true
        task.progressStream = stream
        task.lastUpdated = new Date()
        task.error = undefined
        tasks.value.set(taskId, task)
      }

      // 设置进度监听
      stream
        .onProgress(event => handleTaskProgress(taskId, event))
        .onError(error => handleTaskError(taskId, error))
        .onClose(() => handleTaskClose(taskId))

      console.log('▶️ 任务已恢复:', taskId)
      return stream
    } catch (error) {
      console.error('❌ 恢复任务失败:', error)
      handleTaskError(taskId, error as Error)
      return null
    }
  }

  /**
   * 取消任务
   */
  const cancelTask = async (taskId: string, reason?: string): Promise<boolean> => {
    try {
      await agentApi.cancelTask(taskId, reason)

      // 更新本地状态
      const task = tasks.value.get(taskId)
      if (task) {
        task.status = 'cancelled'
        task.isListening = false
        task.progressStream?.close()
        task.progressStream = undefined
        task.lastUpdated = new Date()
        tasks.value.set(taskId, task)
      }

      console.log('❌ 任务已取消:', taskId)
      return true
    } catch (error) {
      console.error('❌ 取消任务失败:', error)
      handleTaskError(taskId, error as Error)
      return false
    }
  }

  // 监听会话变化，自动刷新任务
  watch(currentConversationId, async newId => {
    if (newId && isInitialized.value && !strictChannelMode.value) {
      await refreshTasks()
    }
  })

  // ===== 返回值 =====

  return {
    // 只读状态
    tasks: readonly(tasks),
    currentConversationId: readonly(currentConversationId),
    connectionStatus: readonly(connectionStatus),
    isInitialized: readonly(isInitialized),
    reconnectAttempts: readonly(reconnectAttempts),
    globalError: readonly(globalError),

    // 计算属性
    currentTasks,
    activeTasks,
    completedTasks,
    errorTasks,
    hasActiveTasks,
    connectionStatusText,

    // 方法
    initialize,
    switchToConversation,
    refreshTasks,
    startTaskListening,
    stopTaskListening,
    stopAllTaskListening,
    executeTask,
    pauseTask,
    resumeTask,
    cancelTask,
    manualReconnect,
    getTask,
    cleanup,
    // 配置
    strictChannelMode,
  }
})
