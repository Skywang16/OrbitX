import { agentApi } from '@/api/agent'
import type { TaskProgressPayload, TaskProgressStream } from '@/api/agent/types'
import { useAISettingsStore } from '@/components/settings/components/AI'
import { restoreAiSidebarState } from '@/persistence/session'
import type { ImageAttachment } from '@/stores/imageLightbox'
import { useSessionStore } from '@/stores/session'
import { useToolConfirmationDialogStore } from '@/stores/toolConfirmationDialog'
import { useWorkspaceStore } from '@/stores/workspace'
import type { ChatMode, Conversation } from '@/types'
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'

export const useAIChatStore = defineStore('ai-chat', () => {
  const workspaceStore = useWorkspaceStore()
  const sessionStore = useSessionStore()
  const aiSettingsStore = useAISettingsStore()
  const toolConfirmStore = useToolConfirmationDialogStore()

  const isVisible = ref(false)
  const sidebarWidth = ref(350)
  const chatMode = ref<ChatMode>('agent')
  const isInitialized = ref(false)
  const isSending = ref(false)
  const error = ref<string | null>(null)
  const cancelFunction = ref<(() => void) | null>(null)
  const currentTaskId = ref<string | null>(null)
  const cancelRequested = ref(false)
  const contextUsage = ref<{ tokensUsed: number; contextWindow: number } | null>(null)
  const streamSessionIds = ref<Set<number>>(new Set())

  // 从消息列表提取最新的 contextUsage
  const extractContextUsage = () => {
    for (let i = workspaceStore.messages.length - 1; i >= 0; i--) {
      const msg = workspaceStore.messages[i]
      if (msg.contextUsage) {
        contextUsage.value = msg.contextUsage
        return
      }
    }
    contextUsage.value = null
  }

  // 工作区路径直接来自 workspaceStore（它从终端派生）
  const currentWorkspacePath = computed(() => workspaceStore.currentWorkspacePath)
  const currentSession = computed(() => workspaceStore.currentSession)
  const sessions = computed<Conversation[]>(() =>
    workspaceStore.sessions.map(session => ({
      id: session.id,
      title: session.title!,
      workspacePath: session.workspacePath,
      messageCount: session.messageCount,
      createdAt: new Date(session.createdAt * 1000),
      updatedAt: new Date(session.updatedAt * 1000),
    }))
  )
  const relatedSessionIds = computed(() => {
    const rootSessionId = workspaceStore.currentSession?.id
    if (!rootSessionId) return []

    const ids = new Set<number>([rootSessionId])
    const rootMessages = workspaceStore.getCachedMessages(rootSessionId)
    for (const msg of rootMessages) {
      for (const block of msg.blocks) {
        if (block.type === 'subtask') {
          ids.add(block.childSessionId)
        }
      }
    }
    for (const id of streamSessionIds.value) {
      ids.add(id)
    }
    return Array.from(ids)
  })

  const combinedMessageList = computed(() => {
    // Nested view design: only show root session messages in the main timeline.
    // Child session messages are displayed inside SubtaskBlock when expanded.
    const rootSessionId = workspaceStore.currentSession?.id
    if (!rootSessionId) return []

    const list = workspaceStore.getCachedMessages(rootSessionId) || []
    return list.filter(m => !m.isInternal)
  })

  const canSendMessage = computed(() => {
    return !isSending.value && aiSettingsStore.hasModels
  })

  const persistIfInitialized = () => {
    if (!isInitialized.value) return
    saveToSessionState()
  }

  const toggleSidebar = async () => {
    isVisible.value = !isVisible.value
    if (isVisible.value) {
      if (!aiSettingsStore.hasModels && !aiSettingsStore.isLoading) {
        await aiSettingsStore.loadSettings()
      }
    }
    persistIfInitialized()
  }

  const setSidebarWidth = (width: number) => {
    sidebarWidth.value = Math.max(300, Math.min(800, width))
    persistIfInitialized()
  }

  const setChatMode = (mode: ChatMode) => {
    chatMode.value = mode
    persistIfInitialized()
  }

  const refreshSessions = async () => {
    const path = currentWorkspacePath.value
    if (!path) return
    await workspaceStore.loadWorkspaceData(path, true)
  }

  const switchSession = async (sessionId: number) => {
    await workspaceStore.switchSession(sessionId)
    extractContextUsage()
  }

  // 开始新对话：清空当前会话状态
  // 发送消息时后端会自动创建新会话
  const startNewChat = async () => {
    // 清空当前会话和消息（包括通知后端）
    await workspaceStore.clearCurrentSession()
    contextUsage.value = null
  }

  const handleAgentEvent = (event: TaskProgressPayload) => {
    switch (event.type) {
      case 'message_created': {
        workspaceStore.upsertMessage(event.message)
        break
      }
      case 'block_appended': {
        workspaceStore.appendBlock(event.messageId, event.block)
        break
      }
      case 'block_updated': {
        workspaceStore.updateBlock(event.messageId, event.blockId, event.block)
        break
      }
      case 'message_finished': {
        workspaceStore.finishMessage(event.messageId, {
          status: event.status,
          finishedAt: event.finishedAt,
          durationMs: event.durationMs,
          tokenUsage: event.tokenUsage,
          contextUsage: event.contextUsage,
        })
        if (event.contextUsage) {
          contextUsage.value = event.contextUsage
        }
        // Stop sending indicator when the main session finishes a non-summary message.
        const mainSessionId = workspaceStore.currentSession?.id
        const msg = workspaceStore.messages.find(m => m.id === event.messageId)
        if (mainSessionId && msg && msg.sessionId === mainSessionId && !msg.isSummary) {
          isSending.value = false
        }
        break
      }
      case 'tool_confirmation_requested': {
        toolConfirmStore.open({
          requestId: event.requestId,
          workspacePath: event.workspacePath,
          toolName: event.toolName,
          summary: event.summary,
        })
        break
      }
      case 'task_completed':
      case 'task_cancelled':
      case 'task_error':
        // Only stop sending for the root task created by this stream.
        if (!currentTaskId.value || event.taskId === currentTaskId.value) {
          isSending.value = false
          toolConfirmStore.close()
        }
        break
    }
  }

  const attachStreamHandlers = (stream: TaskProgressStream) => {
    let cancelSent = false

    stream.onProgress(async event => {
      // TaskCreated: 后端返回权威的 sessionId，用它加载消息
      if (event.type === 'task_created') {
        currentTaskId.value = event.taskId
        streamSessionIds.value = new Set([event.sessionId])
        const workspacePath = currentWorkspacePath.value || event.workspacePath
        if (workspacePath) {
          await workspaceStore.loadWorkspaceData(workspacePath, true)
        } else {
          await workspaceStore.fetchMessages(event.sessionId)
        }
        return
      }

      // Summary 是断点消息：由后端保证顺序，前端直接重新拉取，避免本地排序/插入逻辑。
      if (event.type === 'message_created' && event.message.isSummary) {
        streamSessionIds.value = new Set(streamSessionIds.value).add(event.message.sessionId)
        await workspaceStore.fetchMessages(event.message.sessionId)
        return
      }

      if (event.type === 'message_created') {
        streamSessionIds.value = new Set(streamSessionIds.value).add(event.message.sessionId)
      }

      // 取消逻辑
      if (!cancelSent && cancelRequested.value && currentTaskId.value) {
        cancelSent = true
        await agentApi.cancelTask(currentTaskId.value)
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
      streamSessionIds.value = new Set()
    })

    cancelFunction.value = () => {
      if (cancelRequested.value) return
      cancelRequested.value = true
      toolConfirmStore.close()
      if (currentTaskId.value && !cancelSent) {
        cancelSent = true
        void agentApi.cancelTask(currentTaskId.value)
      }
      isSending.value = false
    }
  }

  const sendMessage = async (content: string, images?: ImageAttachment[]): Promise<void> => {
    if (!aiSettingsStore.hasModels && !aiSettingsStore.isLoading) {
      await aiSettingsStore.loadSettings()
    }

    const parseAgentOverride = (text: string): { agentType?: string; prompt: string } => {
      const trimmed = text.trim()
      if (!trimmed) return { prompt: text }

      const lower = trimmed.toLowerCase()
      if (lower.startsWith('/explore ') || lower === '/explore') {
        return { agentType: 'explore', prompt: trimmed.replace(/^\/explore\b\s*/i, '') }
      }
      if (trimmed.startsWith('用explore') || trimmed.startsWith('使用explore')) {
        return { agentType: 'explore', prompt: trimmed.replace(/^(用|使用)explore\s*/i, '') }
      }

      return { prompt: text }
    }

    const selectedModelId = sessionStore.aiState?.selectedModelId || aiSettingsStore.chatModels[0]?.id
    if (!selectedModelId) {
      throw new Error('请先在设置中选择模型')
    }

    isSending.value = true
    error.value = null

    const { agentType, prompt } = parseAgentOverride(content)

    // 后端会自动处理 session：有则用，无则创建
    const stream = await agentApi.executeTask({
      workspacePath: currentWorkspacePath.value || '',
      sessionId: currentSession.value?.id ?? 0,
      userPrompt: prompt,
      modelId: selectedModelId,
      agentType,
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

  watch(
    relatedSessionIds,
    async (ids, prev) => {
      const prevSet = new Set(prev || [])
      const next = ids.filter(id => !prevSet.has(id))
      for (const sessionId of next) {
        // Best-effort: load persisted history so the merged timeline is complete.
        // Streaming updates will still upsert messages incrementally.
        try {
          await workspaceStore.fetchMessages(sessionId)
        } catch (e) {
          console.warn('[ai-chat] failed to fetch child session messages:', sessionId, e)
        }
      }
    },
    { immediate: true }
  )

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
    extractContextUsage()
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
    const restored = restoreAiSidebarState(sessionStore.aiState)
    if (typeof restored.visible === 'boolean') isVisible.value = restored.visible
    if (typeof restored.width === 'number') sidebarWidth.value = restored.width
    if (restored.mode) chatMode.value = restored.mode as ChatMode
  }

  const initialize = async (): Promise<void> => {
    if (isInitialized.value) return

    await sessionStore.initialize()
    restoreFromSessionState()

    // 加载当前工作区数据（有终端用终端 cwd，无终端用未分组）
    await workspaceStore.loadWorkspaceData(currentWorkspacePath.value, true)
    extractContextUsage()

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
    messageList: combinedMessageList,
    sessions,
    currentSession,
    currentWorkspacePath,
    contextUsage,
    toggleSidebar,
    setSidebarWidth,
    setChatMode,
    refreshSessions,
    switchSession,
    startNewChat,
    sendMessage,
    stopCurrentTask,
    clearError,
    initialize,
    saveToSessionState,
    restoreFromSessionState,
  }
})
