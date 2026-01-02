import { agentApi } from '@/api/agent'
import type { TaskProgressPayload, TaskProgressStream } from '@/api/agent/types'
import { useAISettingsStore } from '@/components/settings/components/AI'
import { useWorkspaceStore } from '@/stores/workspace'
import { useSessionStore } from '@/stores/session'
import type { ImageAttachment } from '@/stores/imageLightbox'
import type { ChatMode, Conversation } from '@/types'
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'

export const useAIChatStore = defineStore('ai-chat', () => {
  const workspaceStore = useWorkspaceStore()
  const sessionStore = useSessionStore()
  const aiSettingsStore = useAISettingsStore()

  const isVisible = ref(false)
  const sidebarWidth = ref(350)
  const chatMode = ref<ChatMode>('agent')
  const isInitialized = ref(false)
  const isSending = ref(false)
  const error = ref<string | null>(null)
  const cancelFunction = ref<(() => void) | null>(null)
  const currentTaskId = ref<string | null>(null)
  const cancelRequested = ref(false)

  // 工作区路径直接来自 workspaceStore（它从终端派生）
  const currentWorkspacePath = computed(() => workspaceStore.currentWorkspacePath)
  const currentSession = computed(() => workspaceStore.currentSession)
  const sessions = computed<Conversation[]>(() =>
    workspaceStore.sessions.map(session => ({
      id: session.id,
      title: session.title ?? '',
      workspacePath: session.workspacePath,
      messageCount: session.messageCount,
      createdAt: new Date(session.createdAt * 1000),
      updatedAt: new Date(session.updatedAt * 1000),
    }))
  )
  const messageList = computed(() => workspaceStore.messages)

  const canSendMessage = computed(() => {
    return !isSending.value && aiSettingsStore.hasModels
  })

  const toggleSidebar = async () => {
    isVisible.value = !isVisible.value
    if (isVisible.value) {
      if (!aiSettingsStore.hasModels && !aiSettingsStore.isLoading) {
        await aiSettingsStore.loadSettings()
      }
    }
  }

  const setSidebarWidth = (width: number) => {
    sidebarWidth.value = Math.max(300, Math.min(800, width))
  }

  const refreshSessions = async () => {
    const path = currentWorkspacePath.value
    if (!path) return
    await workspaceStore.loadWorkspaceData(path, true)
  }

  const switchSession = async (sessionId: number) => {
    await workspaceStore.switchSession(sessionId)
  }

  const createSession = async (title?: string) => {
    await workspaceStore.createSession(title)
  }

  const handleAgentEvent = (event: TaskProgressPayload) => {
    switch (event.type) {
      case 'message_created': {
        workspaceStore.messages.push(event.message)
        break
      }
      case 'block_appended': {
        const msg = workspaceStore.messages.find(m => m.id === event.messageId)
        if (msg) msg.blocks.push(event.block)
        break
      }
      case 'block_updated': {
        const msg = workspaceStore.messages.find(m => m.id === event.messageId)
        if (!msg) break
        const idx = msg.blocks.findIndex(b => 'id' in b && b.id === event.blockId)
        if (idx >= 0) msg.blocks[idx] = event.block
        break
      }
      case 'message_finished': {
        const msg = workspaceStore.messages.find(m => m.id === event.messageId)
        if (!msg) break
        msg.status = event.status
        msg.finishedAt = event.finishedAt
        msg.durationMs = event.durationMs
        msg.tokenUsage = event.tokenUsage
        isSending.value = false
        break
      }
      case 'task_completed':
      case 'task_cancelled':
      case 'task_error':
        isSending.value = false
        break
    }
  }

  const attachStreamHandlers = (stream: TaskProgressStream) => {
    let cancelSent = false

    stream.onProgress(async event => {
      // TaskCreated: 后端返回权威的 sessionId，用它加载消息
      if (event.type === 'task_created') {
        currentTaskId.value = event.taskId
        const workspacePath = currentWorkspacePath.value || event.workspacePath
        if (workspacePath) {
          await workspaceStore.loadWorkspaceData(workspacePath, true)
        } else {
          await workspaceStore.fetchMessages(event.sessionId)
        }
        return
      }

      // 取消逻辑
      if (!cancelSent && cancelRequested.value && currentTaskId.value) {
        await agentApi.cancelTask(currentTaskId.value)
        cancelSent = true
      }

      handleAgentEvent(event)
    })

    stream.onError((streamError: Error) => {
      console.error('Agent task error:', streamError)
      isSending.value = false
    })

    stream.onClose(() => {
      cancelFunction.value = null
      currentTaskId.value = null
      cancelRequested.value = false
      isSending.value = false
    })

    cancelFunction.value = () => {
      cancelRequested.value = true
      if (currentTaskId.value) {
        void agentApi.cancelTask(currentTaskId.value)
      }
      stream.close()
      isSending.value = false
    }
  }

  const sendMessage = async (content: string, images?: ImageAttachment[]): Promise<void> => {
    if (!aiSettingsStore.hasModels && !aiSettingsStore.isLoading) {
      await aiSettingsStore.loadSettings()
    }

    const selectedModelId = sessionStore.aiState?.selectedModelId || aiSettingsStore.chatModels[0]?.id
    if (!selectedModelId) {
      throw new Error('请先在设置中选择模型')
    }

    isSending.value = true
    error.value = null

    // 后端会自动处理 session：有则用，无则创建
    const stream = await agentApi.executeTask({
      workspacePath: currentWorkspacePath.value || '',
      sessionId: currentSession.value?.id ?? 0,
      userPrompt: content,
      modelId: selectedModelId,
      images: images?.map(img => ({
        type: 'image' as const,
        dataUrl: img.dataUrl,
        mimeType: img.mimeType,
      })),
    })

    if (!stream) {
      isSending.value = false
      throw new Error('无法创建任务流')
    }

    attachStreamHandlers(stream)
  }

  const stopCurrentTask = (): void => {
    if (isSending.value && cancelFunction.value) {
      try {
        cancelFunction.value()
      } catch (e) {
        console.warn('停止任务失败:', e)
      } finally {
        cancelFunction.value = null
        isSending.value = false
      }
    }
  }

  const clearError = (): void => {
    error.value = null
  }

  // 当工作区路径变化时，加载对应的工作区数据
  watch(currentWorkspacePath, async newPath => {
    if (!newPath) return
    await workspaceStore.loadWorkspaceData(newPath)
  })

  // 保存 UI 状态（不包含 workspacePath）
  const saveToSessionState = (): void => {
    sessionStore.updateAiState({
      visible: isVisible.value,
      width: sidebarWidth.value,
      mode: chatMode.value,
      selectedModelId: sessionStore.aiState?.selectedModelId,
    })
  }

  // 恢复 UI 状态
  const restoreFromSessionState = (): void => {
    const aiState = sessionStore.aiState
    if (!aiState) return

    isVisible.value = aiState.visible
    sidebarWidth.value = aiState.width
    chatMode.value = aiState.mode as ChatMode
  }

  watch([isVisible, sidebarWidth, chatMode], () => {
    if (isInitialized.value) {
      saveToSessionState()
    }
  })

  const initialize = async (): Promise<void> => {
    if (isInitialized.value) return

    await sessionStore.initialize()
    restoreFromSessionState()

    // 加载当前工作区数据（有终端用终端 cwd，无终端用未分组）
    await workspaceStore.loadWorkspaceData(currentWorkspacePath.value, true)

    isInitialized.value = true
  }

  return {
    isVisible,
    sidebarWidth,
    chatMode,
    isInitialized,
    isSending,
    error,
    canSendMessage,
    messageList,
    sessions,
    currentSession,
    currentWorkspacePath,
    toggleSidebar,
    setSidebarWidth,
    refreshSessions,
    switchSession,
    createSession,
    sendMessage,
    stopCurrentTask,
    clearError,
    initialize,
    saveToSessionState,
    restoreFromSessionState,
  }
})
