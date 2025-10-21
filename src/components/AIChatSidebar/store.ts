import { agentApi } from '@/api/agent'
import { useAISettingsStore } from '@/components/settings/components/AI'
import { useSessionStore } from '@/stores/session'
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { debounce } from 'lodash-es'
import type { ChatMode } from '@/types'
import type { Conversation, Message } from '@/types'
import type { TaskProgressPayload } from '@/api/agent/types'
import { getEventTaskId } from '@/api/agent/types'

// 特殊标识：表示用户正处于"新建会话"的临时状态
const NEW_SESSION_FLAG = -1

export const useAIChatStore = defineStore('ai-chat', () => {
  const sessionStore = useSessionStore()

  // 基本状态
  const isVisible = ref(false)
  const sidebarWidth = ref(350)
  const currentConversationId = ref<number | null>(null)
  const messageList = ref<Message[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const conversations = ref<Conversation[]>([])
  const chatMode = ref<ChatMode>('agent')
  const isInitialized = ref(false)

  // 当前任务状态
  const cancelFunction = ref<(() => void) | null>(null)
  const currentTaskId = ref<string | null>(null)
  const cancelRequested = ref(false)

  // 计算属性
  const canSendMessage = computed(() => {
    const aiSettingsStore = useAISettingsStore()
    return !isLoading.value && aiSettingsStore.hasModels
  })

  const currentConversation = computed(() => {
    if (!currentConversationId.value) return null
    const conversation = conversations.value.find(c => c.id === currentConversationId.value)
    if (!conversation) return null
    return {
      ...conversation,
      messages: messageList.value,
    }
  })

  // 工具函数
  const toggleSidebar = async () => {
    isVisible.value = !isVisible.value
    if (isVisible.value) {
      const aiSettingsStore = useAISettingsStore()
      if (!aiSettingsStore.hasModels && !aiSettingsStore.isLoading) {
        await aiSettingsStore.loadSettings()
      }
      await refreshConversations()
    }
  }

  const setSidebarWidth = (width: number) => {
    sidebarWidth.value = Math.max(300, Math.min(800, width))
  }

  const createConversation = async (): Promise<void> => {
    stopCurrentConversation()

    // 进入临时新建状态，不触碰数据库
    currentConversationId.value = NEW_SESSION_FLAG
    messageList.value = []
  }

  const loadConversation = async (conversationId: number): Promise<void> => {
    isLoading.value = true
    currentConversationId.value = conversationId
    messageList.value = await agentApi.getMessages(conversationId)
    isLoading.value = false
  }

  const switchToConversation = async (conversationId: number): Promise<void> => {
    if (currentConversationId.value === conversationId) {
      return
    }

    stopCurrentConversation()
    messageList.value = []

    // 如果是新建状态，直接切换不加载
    if (conversationId === NEW_SESSION_FLAG) {
      currentConversationId.value = NEW_SESSION_FLAG
      return
    }

    await loadConversation(conversationId)
  }

  const deleteConversation = async (conversationId: number): Promise<void> => {
    await agentApi.deleteConversation(conversationId)
    conversations.value = conversations.value.filter(c => c.id !== conversationId)

    if (currentConversationId.value === conversationId) {
      currentConversationId.value = null
      messageList.value = []
    }
  }

  const refreshConversations = async (): Promise<void> => {
    conversations.value = await agentApi.listConversations()
  }

  const handleAgentEvent = (event: TaskProgressPayload) => {
    // 选择目标消息：优先最后一个处于 streaming 状态的助手消息
    const findTargetAssistantMessage = (): Message | null => {
      for (let i = messageList.value.length - 1; i >= 0; i--) {
        const m = messageList.value[i]
        if (m.role === 'assistant' && (m.status === 'streaming' || !m.status)) {
          return m
        }
      }
      return null
    }

    const currentMessage = findTargetAssistantMessage()
    if (!currentMessage) return

    if (!currentMessage.steps) currentMessage.steps = []

    const getTimestamp = (backendTimestamp: string | number): number => {
      if (typeof backendTimestamp === 'number') return backendTimestamp
      return new Date(backendTimestamp).getTime()
    }

    const upsertStreamStep = (
      stepType: 'thinking' | 'text',
      payload: { streamId: string; streamDone: boolean; iteration: number; timestamp: string },
      delta: string
    ) => {
      // 允许空内容但streamDone=true的事件（结束标记）
      if (!delta && !payload.streamDone) return

      const timestamp = getTimestamp(payload.timestamp)
      const steps = currentMessage.steps!

      // 必须同时匹配stepType和streamId，避免多个流混乱
      const matchingStep = steps.find(
        step => step.stepType === stepType && step.metadata?.streamId === payload.streamId
      )

      if (matchingStep) {
        // 更新现有step
        if (delta) {
          matchingStep.content += delta
        }
        matchingStep.timestamp = timestamp
        // 使用Object.assign保持metadata对象引用，确保Vue响应式
        if (!matchingStep.metadata) {
          matchingStep.metadata = {}
        }
        Object.assign(matchingStep.metadata, {
          streamId: payload.streamId,
          streamDone: payload.streamDone,
          iteration: payload.iteration,
        })
        if (stepType === 'text') {
          currentMessage.content = matchingStep.content
        }
      } else {
        // 创建新step
        const newStep = {
          stepType,
          content: delta,
          timestamp,
          metadata: {
            streamId: payload.streamId,
            streamDone: payload.streamDone,
            iteration: payload.iteration,
          },
        }
        steps.push(newStep)
        if (stepType === 'text') {
          currentMessage.content = newStep.content
        }
      }
    }

    switch (event.type) {
      case 'Thinking': {
        upsertStreamStep(
          'thinking',
          {
            streamId: event.payload.streamId,
            streamDone: event.payload.streamDone,
            iteration: event.payload.iteration,
            timestamp: event.payload.timestamp,
          },
          event.payload.thought
        )
        break
      }

      case 'Text': {
        upsertStreamStep(
          'text',
          {
            streamId: event.payload.streamId,
            streamDone: event.payload.streamDone,
            iteration: event.payload.iteration,
            timestamp: event.payload.timestamp,
          },
          event.payload.text
        )
        break
      }

      case 'ToolUse':
        currentMessage.steps.push({
          stepType: 'tool_use',
          content: `调用工具: ${event.payload.toolName}`,
          timestamp: getTimestamp(event.payload.timestamp),
          metadata: {
            iteration: event.payload.iteration,
            toolId: event.payload.toolId,
            toolName: event.payload.toolName,
            params: event.payload.params,
          },
        })
        break

      case 'ToolResult': {
        const lastStep = currentMessage.steps[currentMessage.steps.length - 1]
        if (lastStep && lastStep.stepType === 'tool_use') {
          lastStep.stepType = 'tool_result'
          lastStep.content = event.payload.isError ? '工具执行出错' : '工具执行完成'
          lastStep.timestamp = getTimestamp(event.payload.timestamp)
          lastStep.metadata = {
            ...lastStep.metadata,
            result: event.payload.result,
            isError: event.payload.isError,
          }
        }
        break
      }

      case 'TaskCompleted':
        currentMessage.status = 'complete'
        currentMessage.duration = Date.now() - currentMessage.createdAt.getTime()
        isLoading.value = false
        break

      case 'Finish':
        break

      case 'TaskCancelled':
        currentMessage.status = 'error'
        isLoading.value = false
        break

      case 'TaskError':
        currentMessage.steps.push({
          stepType: 'error',
          content: event.payload.errorMessage,
          timestamp: getTimestamp(event.payload.timestamp),
          metadata: {
            iteration: event.payload.iteration,
            errorType: event.payload.errorType,
          },
        })
        currentMessage.status = 'error'
        isLoading.value = false
        break
    }
  }

  const sendMessage = async (content: string): Promise<void> => {
    // 如果处于新建状态或无会话，先创建真实会话
    if (!currentConversationId.value || currentConversationId.value === NEW_SESSION_FLAG) {
      isLoading.value = true
      try {
        const conversationId = await agentApi.createConversation()
        const newConversation = await agentApi.getConversation(conversationId)
        conversations.value.unshift(newConversation)
        currentConversationId.value = newConversation.id
        messageList.value = []
      } catch (error) {
        isLoading.value = false
        throw new Error('无法创建会话: ' + (error instanceof Error ? error.message : String(error)))
      }
    }

    isLoading.value = true
    error.value = null

    // 获取选中的模型ID
    const selectedModelId = sessionStore.aiState?.selectedModelId

    if (!selectedModelId) {
      isLoading.value = false
      throw new Error('没有选择模型，请先选择一个模型')
    }

    const stream = await agentApi.executeTask(content, currentConversationId.value, chatMode.value, selectedModelId)

    if (!stream) throw new Error('无法创建任务流')

    let messagesLoaded = false
    let cancelSent = false
    stream.onProgress(async event => {
      const taskId = getEventTaskId(event)
      if (taskId && !currentTaskId.value) currentTaskId.value = taskId
      if (!cancelSent && cancelRequested.value && currentTaskId.value) {
        await agentApi.cancelTask(currentTaskId.value)
        cancelSent = true
      }

      if (!messagesLoaded && event.type === 'TaskStarted') {
        const messages = await agentApi.getMessages(currentConversationId.value!)
        messageList.value = messages
        messagesLoaded = true
        return
      }

      if (!messagesLoaded) return

      handleAgentEvent(event)
    })

    stream.onError((error: Error) => {
      console.error('Agent任务错误:', error)
      const currentMessage = messageList.value[messageList.value.length - 1]
      if (currentMessage && currentMessage.role === 'assistant') {
        currentMessage.status = 'error'
      }
      isLoading.value = false
    })

    stream.onClose(() => {
      cancelFunction.value = null
      currentTaskId.value = null
      cancelRequested.value = false
      isLoading.value = false
    })

    cancelFunction.value = () => {
      cancelRequested.value = true
      if (currentTaskId.value) {
        void agentApi.cancelTask(currentTaskId.value)
      }
      stream.close()
      isLoading.value = false
    }
  }

  const stopCurrentConversation = (): void => {
    if (isLoading.value && cancelFunction.value) {
      try {
        cancelFunction.value()
      } catch (error) {
        console.warn('停止对话时出现错误:', error)
      } finally {
        cancelFunction.value = null
        isLoading.value = false
      }
    }
  }

  const clearError = (): void => {
    error.value = null
  }

  // 会话状态持久化
  const restoreFromSessionState = (): void => {
    const aiState = sessionStore.aiState
    if (aiState) {
      isVisible.value = aiState.visible
      sidebarWidth.value = aiState.width
      chatMode.value = aiState.mode as 'chat' | 'agent'
      currentConversationId.value = aiState.conversationId || null
    }
  }

  const saveToSessionState = (): void => {
    // 过滤掉临时新建状态，不持久化
    const conversationIdToSave =
      currentConversationId.value === NEW_SESSION_FLAG ? undefined : currentConversationId.value || undefined

    sessionStore.updateAiState({
      visible: isVisible.value,
      width: sidebarWidth.value,
      mode: chatMode.value,
      conversationId: conversationIdToSave,
    })
  }

  // 防抖保存状态 - 避免拖拽等高频操作频繁调用后端
  const debouncedSaveState = debounce(() => {
    if (isInitialized.value) {
      saveToSessionState()
    }
  }, 500)

  watch([isVisible, sidebarWidth, chatMode, currentConversationId], debouncedSaveState)

  const initialize = async (): Promise<void> => {
    if (isInitialized.value) return

    await sessionStore.initialize()
    restoreFromSessionState()

    if (currentConversationId.value) {
      await loadConversation(currentConversationId.value)
    }

    await refreshConversations()
    isInitialized.value = true
  }

  return {
    // 状态
    isVisible,
    sidebarWidth,
    currentConversationId,
    currentConversation,
    messageList,
    isLoading,
    error,
    conversations,
    cancelFunction,
    chatMode,
    isInitialized,

    // 计算属性
    canSendMessage,

    // 方法
    toggleSidebar,
    setSidebarWidth,
    createConversation,
    loadConversation,
    switchToConversation,
    deleteConversation,
    refreshConversations,
    sendMessage,
    stopCurrentConversation,
    clearError,
    initialize,
    restoreFromSessionState,
    saveToSessionState,
  }
})
