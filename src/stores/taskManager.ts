import { defineStore } from 'pinia'
import { ref, computed, readonly } from 'vue'
import type { StreamCallbackMessage } from '@/eko/types'
import { TaskAPI, type UITask } from '@/api/tasks'

// ============================================================================
// 双轨制任务管理器 - 按照 task-system-architecture-final.md 设计
// ============================================================================

interface TaskData {
  taskId?: string
  name?: string
  description?: string
  thought?: string
  xml?: string
  taskPrompt?: string
  parentTaskId?: string
  rootTaskId?: string
  childTaskIds?: string[]
  nodes?: Array<{
    type: string
    text?: string
  }>
}

// 简化的Eko事件处理器 - 双轨制架构
class EkoEventHandler {
  public conversationId: number

  constructor(conversationId: number) {
    this.conversationId = conversationId
  }

  async handleEvent(message: StreamCallbackMessage): Promise<void> {
    try {
      // 1. 写入原始上下文轨（事件）- 保存完整的eko事件数据
      await TaskAPI.ekoCtxAppendEvent(
        message.taskId,
        JSON.stringify(message),
        'nodeId' in message ? (message as any).nodeId : undefined
      )

      // 2. 特殊处理：agent_start事件保存为state（包含完整上下文）
      if (message.type === 'agent_start') {
        await this.saveAgentStartState(message)
      }

      // 3. 如果是任务相关事件，更新UI轨
      if (this.isTaskEvent(message)) {
        await this.updateUITask(message)
      }
    } catch (error) {
      console.error('处理Eko事件失败:', error)
    }
  }

  private async saveAgentStartState(message: StreamCallbackMessage): Promise<void> {
    if (message.type !== 'agent_start' || !('task' in message)) return

    const task = (message as any).task
    if (!task) return

    // 构建完整的上下文状态
    const contextState = {
      type: 'agent_context',
      taskId: message.taskId,
      task: task,
      timestamp: Date.now(),
      // 保存eko的完整上下文信息
      taskPrompt: task.taskPrompt || task.description || '',
      xml: task.xml || '',
      nodes: task.nodes || [],
      status: task.status || 'init',
    }

    // 保存为state类型，用于后续prompt构建
    await TaskAPI.ekoCtxUpsertState(
      message.taskId,
      JSON.stringify(contextState),
      this.conversationId,
      'nodeId' in message ? (message as any).nodeId : undefined,
      task.status || 'init'
    )
  }

  private isTaskEvent(message: StreamCallbackMessage): boolean {
    return ['task', 'task_spawn', 'task_status'].includes(message.type)
  }

  private async updateUITask(message: StreamCallbackMessage): Promise<void> {
    const taskData = this.extractTaskData(message)
    if (!taskData) return

    // 从消息中提取状态
    let status = 'init'
    if ('status' in message) {
      status = (message as any).status
    }

    const uiTask: UITask = {
      conversation_id: this.conversationId,
      task_id: message.taskId,
      name: taskData.name || this.generateTaskName(message.taskId, taskData),
      status: this.mapStatus(status),
      render_json: JSON.stringify({
        nodes: taskData.nodes || [],
        parentTaskId: taskData.parentTaskId,
        rootTaskId: taskData.rootTaskId,
        description: taskData.description,
        thought: taskData.thought,
      }),
    }

    await TaskAPI.uiTaskUpsert(uiTask)
  }

  private extractTaskData(message: StreamCallbackMessage): TaskData | null {
    if ('task' in message) {
      return (message as any).task
    }
    return null
  }

  private generateTaskName(taskId: string, task: TaskData): string {
    // 简化的命名逻辑
    if (task.name?.trim()) return task.name.trim()
    if (task.description?.trim()) return task.description.trim().split('\n')[0].substring(0, 50)
    if (task.thought?.trim()) return task.thought.trim().split('\n')[0].substring(0, 50)
    return `Task-${taskId.slice(-8)}`
  }

  private mapStatus(ekoStatus: string): UITask['status'] {
    // 状态映射：引擎状态 -> UI状态
    const statusMap: Record<string, UITask['status']> = {
      init: 'init',
      running: 'active',
      paused: 'paused',
      done: 'completed',
      error: 'error',
      aborted: 'error',
    }
    return statusMap[ekoStatus] || 'init'
  }
}

// 双轨制任务管理器 - 简化版
export const useTaskManager = defineStore('task-manager', () => {
  const eventHandler = ref<EkoEventHandler | null>(null)
  const uiTasks = ref<UITask[]>([])
  const activeTaskId = ref<string | null>(null)
  const currentConversationId = ref<number | null>(null)
  const isInitialized = ref(false)

  // 计算属性
  const currentTasks = computed(() => {
    if (!currentConversationId.value) return []
    return uiTasks.value.filter(task => task.conversation_id === currentConversationId.value)
  })

  const activeTasks = computed(() => {
    return currentTasks.value.filter(task => task.status === 'active' || task.status === 'init')
  })

  const completedTasks = computed(() => {
    return currentTasks.value.filter(task => task.status === 'completed')
  })

  // 初始化
  const initialize = async (): Promise<void> => {
    if (isInitialized.value) return
    isInitialized.value = true
  }

  // 处理Eko消息 - 双轨制入口
  const handleEkoMessage = async (message: StreamCallbackMessage, conversationId?: number): Promise<void> => {
    if (!conversationId) {
      console.warn('⚠️ Missing conversationId for Eko message')
      return
    }

    if (!eventHandler.value || eventHandler.value.conversationId !== conversationId) {
      eventHandler.value = new EkoEventHandler(conversationId)
    }

    try {
      await eventHandler.value.handleEvent(message)
      // 处理完事件后，刷新UI任务列表
      if (conversationId === currentConversationId.value) {
        await refreshTasks()
      }
    } catch (error) {
      console.error('❌ Error handling Eko message:', error)
    }
  }

  // 切换会话
  const switchToConversation = async (conversationId: number): Promise<void> => {
    if (currentConversationId.value === conversationId) return

    currentConversationId.value = conversationId
    activeTaskId.value = null

    // 创建新的事件处理器
    eventHandler.value = new EkoEventHandler(conversationId)

    // 加载任务
    await refreshTasks()

    // 设置活跃任务
    if (activeTasks.value.length > 0) {
      activeTaskId.value = activeTasks.value[activeTasks.value.length - 1].task_id
    }
  }

  // 刷新任务列表
  const refreshTasks = async (): Promise<void> => {
    if (!currentConversationId.value) return

    try {
      const tasks = await TaskAPI.uiTaskList(currentConversationId.value)
      uiTasks.value = tasks
    } catch (error) {
      console.error('❌ Failed to refresh tasks:', error)
    }
  }

  // 切换任务
  const switchToTask = async (taskId: string): Promise<void> => {
    const task = currentTasks.value.find(t => t.task_id === taskId)
    if (task) {
      activeTaskId.value = taskId
    }
  }

  // 获取任务
  const getTask = (taskId: string): UITask | undefined => {
    return currentTasks.value.find(t => t.task_id === taskId)
  }

  // 获取会话统计
  const getConversationStats = (conversationId: number) => {
    const tasks = uiTasks.value.filter(t => t.conversation_id === conversationId)
    return {
      totalTasks: tasks.length,
      activeTasks: tasks.filter(t => t.status === 'active' || t.status === 'init').length,
      completedTasks: tasks.filter(t => t.status === 'completed').length,
      errorTasks: tasks.filter(t => t.status === 'error').length,
    }
  }

  // 清理
  const cleanup = (): void => {
    eventHandler.value = null
    uiTasks.value = []
    activeTaskId.value = null
    currentConversationId.value = null
    isInitialized.value = false
  }

  return {
    // 状态
    uiTasks: readonly(uiTasks),
    activeTaskId: readonly(activeTaskId),
    currentConversationId: readonly(currentConversationId),
    isInitialized: readonly(isInitialized),

    // 计算属性
    currentTasks,
    activeTasks,
    completedTasks,

    // 方法
    initialize,
    handleEkoMessage,
    switchToConversation,
    switchToTask,
    getTask,
    refreshTasks,
    getConversationStats,
    cleanup,
  }
})
