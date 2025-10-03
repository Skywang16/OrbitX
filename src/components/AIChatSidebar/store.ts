import { agentApi } from '@/api/agent'
import { useAISettingsStore } from '@/components/settings/components/AI'
import { useSessionStore } from '@/stores/session'
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import type { ChatMode } from '@/types'
import type { Conversation, Message } from '@/types'
import type { TaskProgressPayload } from '@/api/agent/types'

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

  const findEmptyConversation = (): Conversation | null => {
    return conversations.value.find(conv => conv.messageCount === 0) || null
  }

  const createConversation = async (title?: string): Promise<void> => {
    try {
      stopCurrentConversation()

      const existingEmptyConversation = findEmptyConversation()
      if (existingEmptyConversation) {
        currentConversationId.value = existingEmptyConversation.id
        messageList.value = []
        return
      }

      isLoading.value = true
      const conversationId = await agentApi.createConversation(title)
      const newConversation = await agentApi.getConversation(conversationId)
      conversations.value.unshift(newConversation)
      currentConversationId.value = newConversation.id
      messageList.value = []
    } catch (err) {
      error.value = '创建会话失败'
    } finally {
      isLoading.value = false
    }
  }

  const loadConversation = async (conversationId: number): Promise<void> => {
    try {
      isLoading.value = true
      currentConversationId.value = conversationId
      messageList.value = await agentApi.getMessages(conversationId)
    } catch (err) {
      error.value = '加载会话失败'
    } finally {
      isLoading.value = false
    }
  }

  const switchToConversation = async (conversationId: number): Promise<void> => {
    stopCurrentConversation()
    messageList.value = []
    await loadConversation(conversationId)
  }

  const deleteConversation = async (conversationId: number): Promise<void> => {
    try {
      await agentApi.deleteConversation(conversationId)
      conversations.value = conversations.value.filter(c => c.id !== conversationId)

      if (currentConversationId.value === conversationId) {
        currentConversationId.value = null
        messageList.value = []
      }
    } catch (err) {
      error.value = '删除会话失败'
    }
  }

  const refreshConversations = async (): Promise<void> => {
    try {
      conversations.value = await agentApi.listConversations()
    } catch (err) {
      error.value = '刷新会话列表失败'
    }
  }

  const handleAgentEvent = (event: TaskProgressPayload) => {
    const currentMessage = messageList.value[messageList.value.length - 1]
    if (!currentMessage || currentMessage.role !== 'assistant') return

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
      if (!delta) return

      const timestamp = getTimestamp(payload.timestamp)
      const steps = currentMessage.steps!
      const lastStep = steps.length > 0 ? steps[steps.length - 1] : null
      const isMatchingLastStep = lastStep && lastStep.stepType === stepType

      if (isMatchingLastStep) {
        lastStep.content += delta
        lastStep.timestamp = timestamp
        lastStep.metadata = {
          streamId: payload.streamId,
          streamDone: payload.streamDone,
          iteration: payload.iteration,
        }
        if (stepType === 'text') {
          currentMessage.content = lastStep.content
        }
      } else {
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
    if (!currentConversationId.value) {
      await createConversation()
    }

    if (!currentConversationId.value) {
      throw new Error('无法创建会话')
    }

    try {
      isLoading.value = true
      error.value = null

      // 调用Agent API执行任务（后端会创建消息）
      const stream = await agentApi.executeTask(content, currentConversationId.value)

      if (!stream) throw new Error('无法创建任务流')

      // 等待TaskCreated事件，然后获取后端创建的消息
      let messagesLoaded = false
      stream.onProgress(async event => {
        // 在收到TaskCreated事件时，从后端获取最新消息
        if (event.type === 'TaskCreated' && !messagesLoaded) {
          messagesLoaded = true
          try {
            // 获取后端创建的消息（包含真实的ID和时间戳）
            const messages = await agentApi.getMessages(currentConversationId.value!)
            messageList.value = messages
          } catch (err) {
            console.error('获取消息失败:', err)
          }
        }
        // 处理其他进度事件
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
      })

      // 保存取消函数
      cancelFunction.value = () => {
        stream.close()
        isLoading.value = false
      }
    } catch (err) {
      error.value = '发送消息失败'
      isLoading.value = false
      throw err
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
    sessionStore.updateAiState({
      visible: isVisible.value,
      width: sidebarWidth.value,
      mode: chatMode.value,
      conversationId: currentConversationId.value || undefined,
    })
  }

  const handleStateChange = () => {
    if (isInitialized.value) {
      saveToSessionState()
    }
  }

  watch([isVisible, sidebarWidth, chatMode, currentConversationId], handleStateChange)

  const initialize = async (): Promise<void> => {
    if (isInitialized.value) return

    await sessionStore.initialize()
    restoreFromSessionState()

    if (currentConversationId.value) {
      try {
        await switchToConversation(currentConversationId.value)
      } catch {
        currentConversationId.value = null
      }
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
