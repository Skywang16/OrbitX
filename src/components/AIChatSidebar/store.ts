import { agentApi } from '@/api/agent'
import type { AgentRunEvent, AgentRunStream } from '@/api/agent/types'
import { useAISettingsStore } from '@/components/settings/components/AI'
import type { ImageAttachment } from '@/stores/imageLightbox'
import { useLayoutStore } from '@/stores/layout'
import { useToolConfirmationDialogStore } from '@/stores/toolConfirmationDialog'
import { useWorkspaceStore } from '@/stores/workspace'
import type { RetryStatus } from '@/types'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

export interface QueuedMessage {
  id: string
  content: string
  images?: ImageAttachment[]
}

export const useAIChatStore = defineStore('ai-chat', () => {
  const formatErrorMessage = (error: unknown): string => {
    return error instanceof Error ? error.message : String(error)
  }

  const workspaceStore = useWorkspaceStore()
  const layoutStore = useLayoutStore()
  const aiSettingsStore = useAISettingsStore()
  const toolConfirmStore = useToolConfirmationDialogStore()

  // UI State (delegated to layoutStore)
  const isVisible = computed(() => layoutStore.aiSidebarVisible)
  const sidebarWidth = computed(() => layoutStore.aiSidebarWidth)
  // Local UI state
  const isInitialized = ref(false)
  const error = ref<string | null>(null)
  const cancelFunction = ref<(() => void) | null>(null)
  const cancelRequested = ref(false)
  const contextUsage = ref<{ tokensUsed: number; contextWindow: number } | null>(null)
  const retryStatus = ref<RetryStatus | null>(null)
  const pendingCommandId = ref<string | null>(null)

  // Message queue: per-thread, memory-only (no persistence)
  const messageQueueMap = ref<Map<number, QueuedMessage[]>>(new Map())
  const userCancelled = ref(false)

  // Pure read: returns current thread queue or empty array (no side effects)
  const currentThreadQueue = computed<QueuedMessage[]>(() => {
    const tid = currentThread.value?.id
    if (tid == null) return []
    return messageQueueMap.value.get(tid) ?? []
  })

  // Write path: ensures queue array exists for current thread
  const getOrCreateQueue = (): QueuedMessage[] | null => {
    const tid = currentThread.value?.id
    if (tid == null) return null
    let q = messageQueueMap.value.get(tid)
    if (!q) {
      q = []
      messageQueueMap.value.set(tid, q)
    }
    return q
  }

  const enqueueMessage = (content: string, images?: ImageAttachment[]) => {
    getOrCreateQueue()?.push({ id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`, content, images })
  }

  const removeQueuedMessage = (messageId: string) => {
    const q = getOrCreateQueue()
    if (!q) return
    const i = q.findIndex(m => m.id === messageId)
    if (i >= 0) q.splice(i, 1)
  }

  const updateQueuedMessage = (messageId: string, content: string) => {
    const msg = getOrCreateQueue()?.find(m => m.id === messageId)
    if (msg) msg.content = content
  }

  const reorderQueuedMessage = (from: number, to: number) => {
    const q = getOrCreateQueue()
    if (!q || from < 0 || from >= q.length || to < 0 || to >= q.length) return
    const [item] = q.splice(from, 1)
    q.splice(to, 0, item)
  }

  const sendQueuedMessageNow = async (messageId: string) => {
    const q = getOrCreateQueue()
    if (!q) return
    const i = q.findIndex(m => m.id === messageId)
    if (i < 0) return
    const [msg] = q.splice(i, 1)
    if (isSending.value) {
      stopCurrentTask()
    }
    await sendMessage(msg.content, msg.images)
  }

  const processQueue = async () => {
    const q = getOrCreateQueue()
    if (!q?.length) return
    const [next] = q.splice(0, 1)
    await sendMessage(next.content, next.images)
  }

  // Agent run state — single source of truth
  // idle: no agent run active
  // pending: sendMessage called, waiting for agent_run_created (thread unknown yet)
  // running: agent_run_created received, we know both run id and thread id
  type AgentRunState =
    | { status: 'idle' }
    | { status: 'pending' }
    | { status: 'running'; runId: string; threadId: number }

  const runState = ref<AgentRunState>({ status: 'idle' })

  const resetRunState = () => {
    runState.value = { status: 'idle' }
  }

  // All external queries derived from runState
  const isSending = computed(() => runState.value.status !== 'idle')
  const isCurrentThreadSending = computed(() => {
    const s = runState.value
    if (s.status === 'idle') return false
    if (s.status === 'pending') return true
    return s.threadId === currentThread.value?.id
  })
  const isThreadRunning = (threadId: number): boolean => {
    const s = runState.value
    return s.status === 'running' && s.threadId === threadId
  }

  // Derived
  const currentWorkspacePath = computed(() => workspaceStore.currentWorkspacePath)
  const hasWorkspace = computed(() => workspaceStore.hasWorkspace)
  const currentThread = computed(() => workspaceStore.selectedThread)
  const messageList = computed(() => workspaceStore.messages.filter(m => !m.isInternal))
  const canSendMessage = computed(() => !isSending.value && aiSettingsStore.hasModels && hasWorkspace.value)

  const extractContextUsage = () => {
    const msgs = workspaceStore.messages
    for (let i = msgs.length - 1; i >= 0; i--) {
      const msg = msgs[i]
      if (msg.contextUsage) {
        contextUsage.value = msg.contextUsage
        return
      }
    }
    contextUsage.value = null
  }

  // UI operations
  const toggleSidebar = async () => {
    layoutStore.setAiSidebarVisible(!layoutStore.aiSidebarVisible)
    if (layoutStore.aiSidebarVisible && !aiSettingsStore.hasModels && !aiSettingsStore.isLoading) {
      await aiSettingsStore.loadSettings()
    }
  }

  const setSidebarWidth = (width: number) => {
    layoutStore.setAiSidebarWidth(width)
  }

  // Thread operations
  const isCreatingThread = ref(false)

  const startNewChat = async () => {
    if (isCreatingThread.value) return
    isCreatingThread.value = true
    try {
      const path = currentWorkspacePath.value
      if (path) {
        await workspaceStore.createThread(path, '', 'agent')
      } else {
        workspaceStore.clearSelection()
      }
      contextUsage.value = null
    } finally {
      isCreatingThread.value = false
    }
  }

  const switchThread = async (threadId: number) => {
    const path = currentWorkspacePath.value
    if (!path) return
    await workspaceStore.selectThreadById(threadId, path)
    extractContextUsage()
  }

  const getCurrentThreadSubagentsForMessage = (messageId: number) => {
    const threadId = currentThread.value?.id
    if (!threadId) return []
    return workspaceStore.getSubagents(threadId).filter(subagent => subagent.parentMessageId === messageId)
  }

  // Agent event handling
  const handleAgentEvent = (event: AgentRunEvent) => {
    switch (event.type) {
      case 'agent_run_retrying':
        retryStatus.value = {
          attempt: event.attempt,
          maxAttempts: event.maxAttempts,
          reason: event.reason,
          errorMessage: event.errorMessage,
        }
        return
      case 'message_created':
        retryStatus.value = null
        workspaceStore.upsertMessage(event.message)
        if (event.message.role === 'user' && currentWorkspacePath.value) {
          void workspaceStore.loadThreadViews(currentWorkspacePath.value)
        }
        break
      case 'subagent_created':
      case 'subagent_updated':
        workspaceStore.upsertSubagent(event.subagent)
        break
      case 'block_appended':
        retryStatus.value = null
        workspaceStore.appendBlock(event.messageId, event.block)
        break
      case 'block_updated':
        workspaceStore.updateBlock(event.messageId, event.blockId, event.block)
        break
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
        const s = runState.value
        const msg = workspaceStore.messages.find(m => m.id === event.messageId)
        if (s.status === 'running' && msg && msg.threadId === s.threadId && !msg.isSummary) {
          resetRunState()
        }
        break
      }
      case 'tool_confirmation_requested':
        toolConfirmStore.open({
          requestId: event.requestId,
          workspacePath: event.workspacePath,
          toolName: event.toolName,
          summary: event.summary,
        })
        break
      case 'agent_run_completed':
        retryStatus.value = null
        if (runState.value.status !== 'running' || event.runId === runState.value.runId) {
          resetRunState()
          toolConfirmStore.close()
        }
        // Reload threads to get updated title
        if (currentWorkspacePath.value) {
          void workspaceStore.loadThreadViews(currentWorkspacePath.value)
        }
        break
      case 'agent_run_cancelled':
      case 'agent_run_error':
        retryStatus.value = null
        if (currentWorkspacePath.value) {
          void workspaceStore.loadThreadViews(currentWorkspacePath.value)
        }
        if (runState.value.status !== 'running' || event.runId === runState.value.runId) {
          resetRunState()
          toolConfirmStore.close()
        }
        break
    }
  }

  const attachStreamHandlers = (stream: AgentRunStream) => {
    let cancelSent = false
    let processingPromise: Promise<void> | null = null

    const processEvent = async (event: AgentRunEvent) => {
      if (event.type === 'agent_run_created') {
        runState.value = { status: 'running', runId: event.runId, threadId: event.threadId }
        await workspaceStore.selectThreadById(event.threadId, event.workspacePath)
        return
      }

      if (event.type === 'message_created' && event.message.isSummary) {
        await workspaceStore.fetchMessages(event.message.threadId)
        return
      }

      if (!cancelSent && cancelRequested.value && runState.value.status === 'running') {
        cancelSent = true
        await agentApi.cancelRun(runState.value.runId)
      }

      handleAgentEvent(event)
    }

    // Serialize event processing to avoid race conditions
    stream.onProgress(event => {
      const work = async () => {
        if (processingPromise) await processingPromise
        await processEvent(event)
      }
      processingPromise = work()
    })

    stream.onError((streamError: Error) => {
      console.error('Agent run error:', streamError)
      error.value = formatErrorMessage(streamError)
      resetRunState()
      retryStatus.value = null
      toolConfirmStore.close()
    })

    stream.onClose(() => {
      cancelFunction.value = null
      cancelRequested.value = false
      resetRunState()
      retryStatus.value = null
      // Auto-send next queued message only on natural completion
      if (!userCancelled.value) {
        void processQueue()
      }
      userCancelled.value = false
    })

    cancelFunction.value = () => {
      if (cancelRequested.value) return
      cancelRequested.value = true
      toolConfirmStore.close()
      const s = runState.value
      if (s.status === 'running' && !cancelSent) {
        cancelSent = true
        void agentApi.cancelRun(s.runId).catch(cancelError => {
          console.warn(`Failed to cancel running agent run '${s.runId}':`, cancelError)
        })
      }
      resetRunState()
    }
  }

  // Send message
  const sendMessage = async (content: string, images?: ImageAttachment[]): Promise<void> => {
    const workspacePath = currentWorkspacePath.value
    if (!workspacePath) {
      error.value = 'Please open a workspace folder first'
      return
    }

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
      if (lower.startsWith('/orchestrate ') || lower === '/orchestrate') {
        return { agentType: 'orchestrate', prompt: trimmed.replace(/^\/orchestrate\b\s*/i, '') }
      }
      if (trimmed.startsWith('用explore') || trimmed.startsWith('使用explore')) {
        return { agentType: 'explore', prompt: trimmed.replace(/^(用|使用)explore\s*/i, '') }
      }
      if (trimmed.startsWith('用orchestrate') || trimmed.startsWith('使用orchestrate')) {
        return { agentType: 'orchestrate', prompt: trimmed.replace(/^(用|使用)orchestrate\s*/i, '') }
      }
      return { prompt: text }
    }

    const selectedModelId = layoutStore.selectedModelId || aiSettingsStore.chatModels[0]?.id
    if (!selectedModelId) {
      throw new Error('Please select a model in settings first')
    }

    const threadId = currentThread.value?.id ?? 0

    runState.value = { status: 'pending' }
    error.value = null

    const { agentType, prompt } = parseAgentOverride(content)

    // Consume pending command ID (set by ChatInput, cleared after use)
    const commandId = pendingCommandId.value ?? undefined
    pendingCommandId.value = null

    let stream: AgentRunStream | null = null
    try {
      stream = await agentApi.executeRun({
        workspacePath,
        threadId,
        userPrompt: prompt,
        modelId: selectedModelId,
        agentType,
        commandId,
        images: images?.map(img => ({
          type: 'image' as const,
          dataUrl: img.dataUrl,
          mimeType: img.mimeType,
        })),
      })
    } catch (executeError) {
      resetRunState()
      error.value = formatErrorMessage(executeError)
      throw executeError
    }

    if (!stream) {
      resetRunState()
      throw new Error('Failed to create agent run stream')
    }

    attachStreamHandlers(stream)
  }

  const stopCurrentTask = (): void => {
    if (isSending.value && cancelFunction.value) {
      userCancelled.value = true
      try {
        cancelFunction.value()
      } catch (e) {
        console.warn('Failed to stop agent run:', e)
        error.value = formatErrorMessage(e)
      } finally {
        cancelFunction.value = null
        resetRunState()
      }
    }
  }

  const clearError = (): void => {
    error.value = null
  }

  const initialize = async (): Promise<void> => {
    if (isInitialized.value) return

    await layoutStore.initialize()

    // Load workspace tree
    await workspaceStore.loadTree()
    extractContextUsage()

    isInitialized.value = true
  }

  return {
    // UI state
    isVisible,
    sidebarWidth,
    isInitialized,
    error,
    canSendMessage,
    contextUsage,
    retryStatus,

    // Task state (derived from single source of truth)
    isSending,
    isCurrentThreadSending,
    isThreadRunning,

    // Derived
    messageList,
    currentThread,
    currentWorkspacePath,
    hasWorkspace,
    getCurrentThreadSubagentsForMessage,

    // Operations
    toggleSidebar,
    setSidebarWidth,
    startNewChat,
    switchThread,
    sendMessage,
    stopCurrentTask,
    clearError,
    initialize,
    pendingCommandId,

    // Message queue
    currentThreadQueue,
    enqueueMessage,
    removeQueuedMessage,
    updateQueuedMessage,
    reorderQueuedMessage,
    sendQueuedMessageNow,
  }
})
