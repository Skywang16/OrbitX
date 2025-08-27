import { aiApi } from '@/api'
import { useAISettingsStore } from '@/components/settings/components/AI'
import { useSessionStore } from '@/stores/session'
import { useTerminalStore } from '@/stores/Terminal'
import { handleErrorWithMessage } from '@/utils/errorHandler'
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import type { ChatMode } from '@/types'
import { createTerminalEko, createSidebarCallback, type TerminalEko } from '@/eko'
import type { Conversation, Message } from '@/types'
import { createToolExecution } from '@/types'
import { debounce } from 'lodash-es'

interface StreamMessage {
  type:
    | 'tool_use'
    | 'tool_result'
    | 'workflow'
    | 'text'
    | 'thinking'
    | 'agent_start'
    | 'agent_result'
    | 'tool_streaming'
    | 'tool_running'
    | 'file'
    | 'error'
    | 'finish'
  toolName?: string
  params?: Record<string, any>
  toolResult?: any
  thought?: string
  text?: string
  streamId?: string
  streamDone?: boolean
  workflow?: {
    thought?: string
  }
  // 新增字段支持更多回调类型
  agentName?: string
  agentResult?: any
  toolStreaming?: {
    paramName?: string
    paramValue?: any
    isComplete?: boolean
  }
  fileData?: {
    fileName?: string
    filePath?: string
    content?: string
    mimeType?: string
  }
  error?: {
    message?: string
    code?: string
    details?: any
  }
  finish?: {
    tokenUsage?: {
      promptTokens?: number
      completionTokens?: number
      totalTokens?: number
    }
    duration?: number
    status?: 'success' | 'error' | 'cancelled'
  }
}

const isToolResultError = (toolResult: any): boolean => {
  return toolResult?.isError === true
}

export const useAIChatStore = defineStore('ai-chat', () => {
  const sessionStore = useSessionStore()

  const isVisible = ref(false)
  const sidebarWidth = ref(350)
  const currentConversationId = ref<number | null>(null)
  const messageList = ref<Message[]>([])
  const streamingContent = ref('')
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const conversations = ref<Conversation[]>([])
  const cancelFunction = ref<(() => void) | null>(null)

  const chatMode = ref<ChatMode>('chat')
  const ekoInstance = ref<TerminalEko | null>(null)
  const currentAgentId = ref<string | null>(null)

  const isInitialized = ref(false)

  const debouncedSaveSteps = debounce(async (messageId: number, steps: any[]) => {
    try {
      await aiApi.updateMessageSteps(messageId, steps)
    } catch {
      // Ignore non-critical step save failures
    }
  }, 100)

  const hasMessages = computed(() => messageList.value.length > 0)
  const canSendMessage = computed(() => {
    const aiSettingsStore = useAISettingsStore()
    return !isLoading.value && aiSettingsStore.hasModels
  })

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
      const conversationId = await aiApi.createConversation(title)
      const newConversation = await aiApi.getConversation(conversationId)
      conversations.value.unshift(newConversation)
      currentConversationId.value = newConversation.id
      messageList.value = []
    } catch (err) {
      error.value = handleErrorWithMessage(err, '创建会话失败')
    } finally {
      isLoading.value = false
    }
  }

  const loadConversation = async (conversationId: number, forceReload = false): Promise<void> => {
    try {
      isLoading.value = true
      currentConversationId.value = conversationId

      const loadedMessages = await aiApi.getCompressedContext(conversationId)

      if (forceReload) {
        messageList.value = loadedMessages
      } else {
        const existingIds = new Set(messageList.value.map(msg => msg.id))
        const newMessages = loadedMessages.filter(msg => !existingIds.has(msg.id))

        messageList.value = [...messageList.value, ...newMessages].sort(
          (a, b) => a.createdAt.getTime() - b.createdAt.getTime()
        )
      }
    } catch (err) {
      error.value = handleErrorWithMessage(err, '加载会话失败')
    } finally {
      isLoading.value = false
    }
  }

  const switchToConversation = async (conversationId: number): Promise<void> => {
    stopCurrentConversation()
    messageList.value = []
    await loadConversation(conversationId, true)
  }

  const deleteConversation = async (conversationId: number): Promise<void> => {
    try {
      await aiApi.deleteConversation(conversationId)
      conversations.value = conversations.value.filter(c => c.id !== conversationId)

      if (currentConversationId.value === conversationId) {
        currentConversationId.value = null
        messageList.value = []
      }
    } catch (err) {
      error.value = handleErrorWithMessage(err, '删除会话失败')
    }
  }

  const refreshConversations = async (): Promise<void> => {
    try {
      conversations.value = await aiApi.getConversations()
    } catch (err) {
      error.value = handleErrorWithMessage(err, '刷新会话列表失败')
    }
  }

  const sendMessage = async (content: string): Promise<void> => {
    if (!currentConversationId.value) {
      await createConversation()
    }

    if (!currentConversationId.value) {
      throw new Error('无法创建会话')
    }

    let tempAIMessage: Message | null = null

    try {
      isLoading.value = true
      error.value = null

      const userMessageId = await aiApi.saveMessage(currentConversationId.value, 'user', content)

      const userMessage: Message = {
        id: userMessageId,
        conversationId: currentConversationId.value,
        role: 'user',
        content,
        createdAt: new Date(),
      }
      messageList.value.push(userMessage)

      if (!ekoInstance.value) {
        await initializeEko()
      }

      ekoInstance.value?.setMode(chatMode.value)

      const terminalStore = useTerminalStore()
      const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)
      const currentWorkingDirectory = activeTerminal?.cwd

      const fullPrompt = await aiApi.buildPromptWithContext(
        currentConversationId.value,
        content,
        userMessageId,
        currentWorkingDirectory
      )

      const messageId = await aiApi.saveMessage(currentConversationId.value, 'assistant', '正在生成回复...')

      tempAIMessage = {
        id: messageId,
        conversationId: currentConversationId.value,
        role: 'assistant',
        createdAt: new Date(),
        steps: [],
        status: 'streaming',
      }

      messageList.value.push(tempAIMessage)

      cancelFunction.value = () => {
        if (ekoInstance.value) {
          ekoInstance.value.abort()
        }
      }

      streamingContent.value = ''
      const response = await ekoInstance.value!.run(fullPrompt)

      if (tempAIMessage && response.success) {
        tempAIMessage.content = (tempAIMessage.content as string | undefined) ?? ((response.result as string) || '')
        tempAIMessage.status = 'complete'
        tempAIMessage.duration = Date.now() - tempAIMessage.createdAt.getTime()

        const messageIndex = messageList.value.findIndex(m => m.id === tempAIMessage!.id)
        if (messageIndex !== -1) {
          messageList.value[messageIndex] = { ...tempAIMessage }
        }

        await aiApi.updateMessageContent(tempAIMessage.id, tempAIMessage.content)
        await aiApi.updateMessageStatus(tempAIMessage.id, tempAIMessage.status, tempAIMessage.duration)
      } else if (tempAIMessage) {
        tempAIMessage.status = 'error'
        tempAIMessage.duration = Date.now() - tempAIMessage.createdAt.getTime()

        tempAIMessage.steps?.push({
          type: 'error',
          content: ``,
          timestamp: Date.now(),
          metadata: {
            errorType: 'EkoError',
            errorDetails: response.error,
          },
        })

        if (tempAIMessage) {
          const messageIndex = messageList.value.findIndex(m => m.id === tempAIMessage!.id)
          if (messageIndex !== -1) {
            messageList.value[messageIndex] = { ...tempAIMessage }
          }

          if (tempAIMessage.steps) {
            try {
              await aiApi.updateMessageStatus(tempAIMessage.id, tempAIMessage.status, tempAIMessage.duration)
              await aiApi.updateMessageSteps(tempAIMessage.id, tempAIMessage.steps)
            } catch {
              // Ignore non-critical database failures
            }
          }
        }
      }

      await refreshConversations()
    } catch (err) {
      error.value = handleErrorWithMessage(err, '发送消息失败')
      throw err
    } finally {
      isLoading.value = false
      cancelFunction.value = null
    }
  }

  const truncateAndResend = async (truncateAfterMessageId: number, newContent: string): Promise<void> => {
    if (!currentConversationId.value) {
      throw new Error('没有选择会话')
    }

    try {
      isLoading.value = true
      error.value = null

      await aiApi.truncateConversation(currentConversationId.value, truncateAfterMessageId)
      await sendMessage(newContent)
    } catch (err) {
      error.value = handleErrorWithMessage(err, '截断重问失败')
      throw err
    } finally {
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
  const initializeEko = async (): Promise<void> => {
    try {
      if (!ekoInstance.value) {
        const handleStreamMessage = async (message: StreamMessage) => {
          const tempMessage = messageList.value[messageList.value.length - 1]
          if (!tempMessage || tempMessage.role !== 'assistant') return

          tempMessage.steps = tempMessage.steps || []

          const updateOrCreateStep = (stepData: { type: string; content: string; streamId?: string }) => {
            let targetStep: any = null

            if (stepData.type === 'thinking') {
              if (stepData.streamId) {
                targetStep = tempMessage.steps?.find(
                  step => step.type === 'thinking' && step.metadata?.streamId === stepData.streamId
                )
              } else {
                const thinkingSteps = tempMessage.steps?.filter(step => step.type === 'thinking') || []
                targetStep = thinkingSteps[thinkingSteps.length - 1] || null
              }
            } else {
              targetStep = stepData.streamId
                ? tempMessage.steps?.find(
                    step => step.type === stepData.type && step.metadata?.streamId === stepData.streamId
                  )
                : null
            }

            if (targetStep) {
              targetStep.content = stepData.content
            } else {
              tempMessage.steps?.push({
                type: stepData.type as any,
                content: stepData.content,
                timestamp: Date.now(),
                metadata: stepData.streamId ? { streamId: stepData.streamId } : undefined,
              })
            }
          }

          switch (message.type) {
            case 'tool_use':
              tempMessage.steps.push({
                type: 'tool_use',
                content: '',
                timestamp: Date.now(),
                toolExecution: createToolExecution(message.toolName || '', message.params || {}, 'running'),
              })
              break

            case 'tool_result': {
              const toolSteps = tempMessage.steps.filter((step: any) => step.type === 'tool_use')
              const toolStep = toolSteps[toolSteps.length - 1] as any
              if (toolStep?.toolExecution) {
                const hasError = isToolResultError(message.toolResult)
                toolStep.toolExecution.status = hasError ? 'error' : 'completed'
                toolStep.toolExecution.endTime = Date.now()
                toolStep.toolExecution.result = message.toolResult

                if (hasError) {
                  toolStep.toolExecution.error = '工具执行失败'
                }
              }
              break
            }

            case 'thinking':
              updateOrCreateStep({
                type: 'thinking',
                content: message.thought || message.text || '',
                streamId: message.streamId,
              })
              break

            case 'workflow':
              if (message.workflow?.thought) {
                let thinkingStep = tempMessage.steps?.find(step => step.type === 'thinking')

                if (thinkingStep) {
                  thinkingStep.content = message.workflow.thought
                  if (message.streamDone) {
                    thinkingStep.metadata = {
                      ...thinkingStep.metadata,
                      thinkingDuration: Date.now() - thinkingStep.timestamp,
                    }
                  }
                } else {
                  tempMessage.steps?.push({
                    type: 'thinking' as any,
                    content: message.workflow.thought,
                    timestamp: Date.now(),
                    metadata: {
                      thinkingDuration: message.streamDone ? 0 : undefined,
                    },
                  })
                }
              }
              break

            case 'text':
              updateOrCreateStep({
                type: 'text',
                content: message.text || '',
                streamId: message.streamId,
              })

              if (message.streamDone) {
                tempMessage.content = message.text || ''
              }
              break

            case 'agent_start':
              console.log('🚀 [侧边栏] Agent开始执行:', {
                agentName: message.agentName,
                timestamp: new Date().toISOString(),
              })
              break

            case 'agent_result':
              console.log('✅ [侧边栏] Agent执行结果:', {
                agentName: message.agentName,
                result: message.agentResult,
                timestamp: new Date().toISOString(),
              })
              break

            case 'tool_streaming':
              console.log('📡 [侧边栏] 工具参数流式输出:', {
                toolName: message.toolName,
                streaming: message.toolStreaming,
                timestamp: new Date().toISOString(),
              })
              break

            case 'tool_running':
              console.log('⚙️ [侧边栏] 工具执行中:', {
                toolName: message.toolName,
                params: message.params,
                timestamp: new Date().toISOString(),
              })
              break

            case 'file':
              console.log('📁 [侧边栏] 文件输出:', {
                fileData: message.fileData,
                timestamp: new Date().toISOString(),
              })
              break

            case 'error':
              console.log('❌ [侧边栏] 错误信息:', {
                error: message.error,
                timestamp: new Date().toISOString(),
              })
              break

            case 'finish':
              console.log('🏁 [侧边栏] 完成信息:', {
                finish: message.finish,
                timestamp: new Date().toISOString(),
              })
              break
          }

          debouncedSaveSteps(tempMessage.id, tempMessage.steps)
        }

        const callback = createSidebarCallback(handleStreamMessage)

        ekoInstance.value = await createTerminalEko({
          callback,
          debug: true,
        })
      }
    } catch (err) {
      ekoInstance.value = await createTerminalEko({ debug: true })
    }
  }

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
    isVisible,
    sidebarWidth,
    currentConversationId,
    messageList,
    streamingContent,
    isLoading,
    error,
    conversations,
    cancelFunction,
    chatMode,
    ekoInstance,
    currentAgentId,
    isInitialized,
    hasMessages,
    canSendMessage,
    toggleSidebar,
    setSidebarWidth,
    createConversation,
    loadConversation,
    switchToConversation,
    deleteConversation,
    refreshConversations,
    sendMessage,
    truncateAndResend,
    stopCurrentConversation,
    clearError,
    initializeEko,
    initialize,
    restoreFromSessionState,
    saveToSessionState,
  }
})
