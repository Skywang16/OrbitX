/**
 * Agent状态同步适配器（重置版）
 *
 * 前端仅发送用户消息；其余流程（任务管理、工具调用、事件推送、上下文持久化）统一由后端 Agent 模块负责。
 * 本适配器：
 * - 负责对接后端 Tauri Channel，转成前端 Message.steps 的渲染结构
 * - 仅使用 PersistedStep（thinking/text/tool_use/error）类型来驱动 UI
 */

import { ref, computed, watch } from 'vue'
import { useAgentStateManager } from '@/stores/agentStateManager'
import { useAIChatStore } from '@/components/AIChatSidebar/store'
import type { TaskProgressPayload, TaskProgressStream } from '@/api/agent/types'
import type { PersistedStep, PersistedNonToolStep, PersistedToolStep, PersistedToolExecution } from '@/api/ai/types'
import type { Message } from '@/types'

export class AgentStateSyncAdapter {
  private agentStateManager = useAgentStateManager()
  private chatStore = useAIChatStore()

  // task_id -> stream
  private activeStreams = new Map<string, TaskProgressStream>()

  public isAgentMode = ref(false)
  public currentAgentTaskId = ref<string | null>(null)

  async initialize(): Promise<void> {
    if (!this.agentStateManager.isInitialized) {
      await this.agentStateManager.initialize()
    }

    // 跟随聊天模式
    watch(
      () => this.chatStore.chatMode,
      mode => {
        this.isAgentMode.value = mode === 'agent'
      },
      { immediate: true }
    )

    // 跟随会话切换
    watch(
      () => this.chatStore.currentConversationId,
      async conversationId => {
        if (conversationId && this.isAgentMode.value) {
          await this.agentStateManager.switchToConversation(conversationId)
          await this.syncAgentTasks()
        }
      },
      { immediate: true }
    )

    console.warn('AgentStateSyncAdapter initialized')
  }

  async executeAgentTask(userPrompt: string): Promise<void> {
    if (!this.chatStore.currentConversationId) {
      throw new Error('未选择会话')
    }

    console.warn('🚀 开始执行Agent任务:', { userPrompt, conversationId: this.chatStore.currentConversationId })

    const stream = await this.agentStateManager.executeTask(userPrompt, this.chatStore.currentConversationId)

    if (!stream) throw new Error('无法创建任务进度流')

    let taskId: string | null = null

    stream.onProgress(event => {
      if (event.type === 'TaskCreated') {
        taskId = event.payload.taskId
        this.currentAgentTaskId.value = taskId
        this.activeStreams.set(taskId, stream)
      }
      if (taskId) this.handleAgentProgressEvent(taskId, event)
    })

    stream.onError(error => {
      console.error('Agent任务执行错误:', error)
      this.handleAgentError(taskId || 'unknown', error)
    })

    stream.onClose(() => {
      if (taskId) {
        this.activeStreams.delete(taskId)
        if (this.currentAgentTaskId.value === taskId) this.currentAgentTaskId.value = null
      }
    })
  }

  async pauseCurrentTask(): Promise<boolean> {
    const taskId = this.currentAgentTaskId.value
    if (!taskId) return false
    return await this.agentStateManager.pauseTask(taskId)
  }

  async resumeTask(taskId: string): Promise<boolean> {
    try {
      const stream = await this.agentStateManager.resumeTask(taskId)
      if (!stream) return false

      this.currentAgentTaskId.value = taskId
      this.activeStreams.set(taskId, stream)

      stream
        .onProgress(e => this.handleAgentProgressEvent(taskId, e))
        .onError(err => this.handleAgentError(taskId, err))
        .onClose(() => {
          this.activeStreams.delete(taskId)
          if (this.currentAgentTaskId.value === taskId) this.currentAgentTaskId.value = null
        })

      return true
    } catch (e) {
      console.error('恢复Agent任务失败:', e)
      return false
    }
  }

  async cancelTask(taskId: string): Promise<boolean> {
    const success = await this.agentStateManager.cancelTask(taskId, '用户取消')
    if (success) {
      this.activeStreams.delete(taskId)
      if (this.currentAgentTaskId.value === taskId) this.currentAgentTaskId.value = null
    }
    return success
  }

  getAgentTasks() {
    return this.agentStateManager.currentTasks
  }

  getActiveAgentTasks() {
    return this.agentStateManager.activeTasks
  }

  async switchToAgentTask(taskId: string): Promise<void> {
    const task = this.agentStateManager.getTask(taskId)
    if (!task) return

    this.currentAgentTaskId.value = taskId

    if (task.status === 'running' && !task.isListening) {
      const stream = await this.agentStateManager.startTaskListening(taskId)
      if (stream) {
        this.activeStreams.set(taskId, stream)
        stream
          .onProgress(e => this.handleAgentProgressEvent(taskId, e))
          .onError(err => this.handleAgentError(taskId, err))
          .onClose(() => {
            this.activeStreams.delete(taskId)
            if (this.currentAgentTaskId.value === taskId) this.currentAgentTaskId.value = null
          })
      }
    }
  }

  private handleAgentProgressEvent(taskId: string, event: TaskProgressPayload): void {
    if (this.currentAgentTaskId.value !== taskId) return

    // 添加调试日志
    console.warn(`📨 接收到Agent事件 [${event.type}]:`, event.payload)

    const currentMessage = this.chatStore.messageList[this.chatStore.messageList.length - 1] as Message | undefined
    if (!currentMessage || currentMessage.role !== 'assistant') return

    currentMessage.steps = currentMessage.steps || []

    try {
      switch (event.type) {
        case 'Thinking': {
          // EKO：使用传入的 streamId/streamDone
          this.upsertThinkingStep(currentMessage, event.payload.streamId, event.payload.thought)
          break
        }
        case 'Text': {
          // EKO：text 流（替换式更新），按 streamId 聚合
          this.upsertTextStep(currentMessage, event.payload.streamId, event.payload.text)
          break
        }
        case 'ToolUse': {
          const exec: PersistedToolExecution = {
            name: event.payload.toolName,
            status: 'running',
            params: (event.payload.params as Record<string, unknown>) || {},
            toolId: event.payload.toolId,
            startTime: Date.now(),
          }
          this.updateOrAddToolStep(currentMessage, exec)
          break
        }
        case 'ToolResult': {
          const exec: PersistedToolExecution = {
            name: event.payload.toolName,
            status: event.payload.isError ? 'error' : 'completed',
            result: event.payload.result,
            toolId: event.payload.toolId,
            endTime: Date.now(),
          }
          this.updateOrAddToolStep(currentMessage, exec)
          break
        }
        case 'FinalAnswer': {
          // 兼容旧事件：将最终答案写入以 iteration 为标识的 text 块
          const streamId = `final_${event.payload.iteration}`
          this.upsertTextStep(currentMessage, streamId, event.payload.answer)
          // 更新消息内容和状态
          currentMessage.content = event.payload.answer
          currentMessage.status = 'complete'
          console.warn('🎯 收到最终答案:', event.payload.answer)
          break
        }
        case 'Finish': {
          // 完整对齐 EKO：结束事件（可用于埋点/统计）
          currentMessage.status = 'complete'
          break
        }
        case 'TaskError': {
          const step: PersistedNonToolStep = {
            type: 'error',
            content: `任务错误: ${event.payload.errorMessage}`,
            timestamp: Date.now(),
            metadata: { errorType: event.payload.errorType, errorDetails: event.payload.errorMessage },
          }
          this.addAgentStep(currentMessage, step)
          currentMessage.status = 'error'
          break
        }
        case 'TaskCompleted': {
          currentMessage.status = 'complete'
          currentMessage.duration = Date.now() - currentMessage.createdAt.getTime()
          break
        }
        case 'TaskCancelled': {
          currentMessage.status = 'complete'
          break
        }
      }

      const idx = this.chatStore.messageList.findIndex(m => m.id === currentMessage.id)
      if (idx !== -1) this.chatStore.messageList[idx] = { ...currentMessage }

      if (currentMessage.id && currentMessage.steps) {
        const saveSteps = (
          this.chatStore as unknown as {
            debouncedSaveSteps?: (id: string, steps: PersistedStep[]) => void
          }
        ).debouncedSaveSteps
        if (typeof saveSteps === 'function') {
          saveSteps(String(currentMessage.id), currentMessage.steps as PersistedStep[])
        }
      }
    } catch (e) {
      console.error('处理Agent进度事件失败:', e)
    }
  }

  /**
   * 将“thinking”类型渲染统一成单个可展开的块：
   * - 使用稳定的 streamId（think_<taskId>）贯穿 TaskCreated/TaskStarted/Thinking
   * - 支持按需追加内容
   */
  private upsertThinkingStep(message: Message, streamId: string, content: string): void {
    message.steps = message.steps || []
    const idx = message.steps.findIndex(
      (s: PersistedStep) => s.type === 'thinking' && (s as PersistedNonToolStep).metadata?.streamId === streamId
    )

    const step: PersistedNonToolStep = {
      type: 'thinking',
      content,
      timestamp: Date.now(),
      metadata: { streamId },
    }

    if (idx >= 0) message.steps[idx] = step
    else message.steps.push(step)
  }

  /**
   * 文本增量更新：以 streamId 为键将多次 TextDelta 聚合为单一 text 块
   */
  private upsertTextStep(message: Message, streamId: string, delta: string): void {
    message.steps = message.steps || []
    const idx = message.steps.findIndex(
      (s: PersistedStep) => s.type === 'text' && (s as PersistedNonToolStep).metadata?.streamId === streamId
    )
    const content = delta // 替换式更新，与 EKO 语义保持一致
    const step: PersistedNonToolStep = {
      type: 'text',
      content,
      timestamp: Date.now(),
      metadata: { streamId },
    }
    if (idx >= 0) message.steps[idx] = step
    else message.steps.push(step)
  }

  private handleAgentError(taskId: string, error: Error): void {
    console.error('Agent任务错误:', taskId, error)
    const currentMessage = this.chatStore.messageList[this.chatStore.messageList.length - 1] as Message | undefined
    if (!currentMessage || currentMessage.role !== 'assistant') return

    const step: PersistedNonToolStep = {
      type: 'error',
      content: `系统错误: ${error.message}`,
      timestamp: Date.now(),
      metadata: { errorType: 'system_error', errorDetails: error.message },
    }
    this.addAgentStep(currentMessage, step)

    currentMessage.status = 'error'
    const idx = this.chatStore.messageList.findIndex(m => m.id === currentMessage.id)
    if (idx !== -1) this.chatStore.messageList[idx] = { ...currentMessage }
  }

  private addAgentStep(message: Message, step: PersistedStep): void {
    if (!message.steps) message.steps = []
    message.steps.push(step)
  }

  private updateOrAddToolStep(message: Message, exec: PersistedToolExecution): void {
    if (!message.steps) message.steps = []
    const idx = message.steps.findIndex(
      (s: PersistedStep) => s.type === 'tool_use' && (s as PersistedToolStep).toolExecution?.toolId === exec.toolId
    )
    const step: PersistedToolStep = { type: 'tool_use', timestamp: Date.now(), toolExecution: exec }
    if (idx >= 0) message.steps[idx] = step
    else message.steps.push(step)
  }

  private async syncAgentTasks(): Promise<void> {
    const active = this.agentStateManager.activeTasks
    for (const task of active) {
      if (!task.isListening && task.status === 'running') {
        console.warn('恢复任务监听:', task.taskId)
        await this.agentStateManager.startTaskListening(task.taskId)
      }
    }
  }

  async cleanup(): Promise<void> {
    for (const [, stream] of this.activeStreams.entries()) stream.close()
    this.activeStreams.clear()
    this.currentAgentTaskId.value = null
    this.isAgentMode.value = false
    await this.agentStateManager.cleanup()
  }
}

export function useAgentStateSyncAdapter() {
  const adapter = new AgentStateSyncAdapter()

  return {
    adapter,
    // 响应式状态
    isAgentMode: adapter.isAgentMode,
    currentAgentTaskId: adapter.currentAgentTaskId,
    // 计算属性
    agentTasks: computed(() => adapter.getAgentTasks()),
    activeAgentTasks: computed(() => adapter.getActiveAgentTasks()),
    hasActiveAgentTasks: computed(() => adapter.getActiveAgentTasks().length > 0),
    // 方法
    initialize: () => adapter.initialize(),
    executeAgentTask: (prompt: string) => adapter.executeAgentTask(prompt),
    pauseCurrentTask: () => adapter.pauseCurrentTask(),
    resumeTask: (taskId: string) => adapter.resumeTask(taskId),
    cancelTask: (taskId: string) => adapter.cancelTask(taskId),
    switchToAgentTask: (taskId: string) => adapter.switchToAgentTask(taskId),
    cleanup: () => adapter.cleanup(),
  }
}
