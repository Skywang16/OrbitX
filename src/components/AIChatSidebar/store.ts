import { aiApi } from '@/api'
import type { PersistedStep } from '@/api/ai/types'
import { useAISettingsStore } from '@/components/settings/components/AI'
import { useSessionStore } from '@/stores/session'

import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import type { ChatMode } from '@/types'
import { useAgentStateSyncAdapter } from '@/stores/agentStateSyncAdapter'
import type { Conversation, Message } from '@/types'
import { debounce } from 'lodash-es'

export const useAIChatStore = defineStore('ai-chat', () => {
  const sessionStore = useSessionStore()
  // TaskManager 已移除：前端不再承担任务管理逻辑

  const isVisible = ref(false)
  const sidebarWidth = ref(350)
  const currentConversationId = ref<number | null>(null)
  const messageList = ref<Message[]>([])
  const streamingContent = ref('')
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const conversations = ref<Conversation[]>([])
  const cancelFunction = ref<(() => void) | null>(null)

  // 任务节点与旧 TaskManager 逻辑已移除

  const chatMode = ref<ChatMode>('agent')

  const isInitialized = ref(false)
  // Agent 后端适配器（统一走后端 TaskExecutor 通道）
  const {
    initialize: initializeAgentAdapter,
    executeAgentTask: executeAgentTaskViaAdapter,
    cancelTask: cancelAgentTask,
    currentAgentTaskId,
  } = useAgentStateSyncAdapter()

  const debouncedSaveSteps = debounce(async (messageId: number, steps: PersistedStep[]) => {
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

  const currentConversation = computed(() => {
    if (!currentConversationId.value) return null
    const conversation = conversations.value.find(c => c.id === currentConversationId.value)
    if (!conversation) return null

    return {
      ...conversation,
      messages: messageList.value,
    }
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
      // 任务节点与 TaskManager 已移除，无需切换
    } catch (err) {
      error.value = '创建会话失败'
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
      error.value = '加载会话失败'
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
        // 新架构下，无需手动清理任务节点
      }
    } catch (err) {
      error.value = '删除会话失败'
    }
  }

  const refreshConversations = async (): Promise<void> => {
    try {
      conversations.value = await aiApi.getConversations()
    } catch (err) {
      error.value = '刷新会话列表失败'
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

      // 统一通过后端 Agent 通道执行（不再走 EKO 路径）
      await initializeAgentAdapter()

      // 创建 assistant 占位消息（供后续进度事件渲染步骤）
      const messageId = await aiApi.saveMessage(currentConversationId.value, 'assistant', 'Thinking...')

      tempAIMessage = {
        id: messageId,
        conversationId: currentConversationId.value,
        role: 'assistant',
        createdAt: new Date(),
        steps: [],
        status: 'streaming',
      }
      messageList.value.push(tempAIMessage)

      // 设置取消函数：调用后端 Agent 取消当前任务
      cancelFunction.value = () => {
        const taskId = currentAgentTaskId?.value
        if (taskId) {
          void cancelAgentTask(taskId)
        }
      }

      // 交给后端 Agent 执行；UI 渲染由适配器基于 Channel 事件更新
      await executeAgentTaskViaAdapter(content)

      // 后续的完成/错误状态将由事件驱动更新，这里即可返回
      return
    } catch (err) {
      error.value = '发送消息失败'
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
      error.value = '截断重问失败'
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

  // EKO 工具结果处理已移除
  /* EKO removed: initializeEko legacy function
  const initializeEko = async (): Promise<void> => {
    try {
      if (!ekoInstance.value) {
        const handleStreamMessage = async (message: StreamCallbackMessage) => {
          try {
            // Forward all messages to TaskManager for processing
            if (currentConversationId.value) {
              taskManager.handleEkoMessage(message, currentConversationId.value)
            }

            const tempMessage = messageList.value[messageList.value.length - 1]
            if (!tempMessage || tempMessage.role !== 'assistant') {
              return
            }

            tempMessage.steps = tempMessage.steps || []

            // Handle message rendering (keep existing logic for UI steps)
            switch (message.type) {
              // Task-related events are now handled by TaskManager
              // We only keep the UI rendering logic here
              case 'task_spawn':
              case 'task_pause':
              case 'task_resume':
              case 'task_child_result':
              case 'task_status':
              case 'task_tree_update':
                // These are now handled by TaskManager
                // No UI rendering needed for these events
                break
              case 'tool_use':
                handleToolUse(tempMessage, message)
                break

              case 'tool_streaming':
                handleToolStreaming(tempMessage, message)
                break

              case 'tool_result':
                handleToolResult(tempMessage, message)
                break

              case 'thinking':
                if (message.type === 'thinking') {
                  if (!isForCurrentTask(message)) break
                  updateOrCreateStep(tempMessage, {
                    type: 'thinking',
                    content: message.text || '',
                    streamId: message.streamId,
                    streamDone: message.streamDone,
                  })

                  // 根据nodeId更新节点状态
                  if (message.nodeId && currentTaskId.value) {
                    const nodeInfo = parseNodeId(message.nodeId, currentTaskId.value)
                    if (nodeInfo && nodeInfo.type === 'thinking') {
                      // 思考阶段，保持当前节点为运行中
                      updateNodeStatus(currentNodeIndex.value, 'running')
                    }
                  }
                }
                break

              case 'text':
                if (message.type === 'text') {
                  if (!isForCurrentTask(message)) break
                  updateOrCreateStep(tempMessage, {
                    type: 'text',
                    content: message.text || '',
                    streamId: message.streamId,
                    streamDone: message.streamDone,
                  })

                  // 根据nodeId更新节点状态
                  if (message.nodeId && currentTaskId.value) {
                    const nodeInfo = parseNodeId(message.nodeId, currentTaskId.value)
                    if (nodeInfo && nodeInfo.type === 'execution') {
                      // 执行阶段，当前节点运行中
                      updateNodeStatus(currentNodeIndex.value, 'running')

                      // 如果是流式完成，可能需要推进到下一个节点
                      if (message.streamDone && currentNodeIndex.value < currentTaskNodes.value.length - 1) {
                        updateNodeStatus(currentNodeIndex.value, 'completed')
                        currentNodeIndex.value++
                        updateNodeStatus(currentNodeIndex.value, 'running')
                      }
                    }
                  }

                  if (message.streamDone) {
                    tempMessage.content = message.text || ''
                  }
                }
                break

              case 'task':
                if (message.type === 'task') {
                  // Task data is now handled by TaskManager
                  // Only handle UI rendering for current task
                  const isCurrent = taskManager.activeTaskId === message.taskId || !taskManager.activeTaskId

                  if (isCurrent) {
                    // Update legacy node state for backward compatibility
                    const newNodes = message.task?.nodes
                      ?.filter(node => node.type === 'normal' && 'text' in node)
                      .map((node, index) => ({
                        type: node.type,
                        text: 'text' in node ? node.text : '',
                        status: (index === 0 ? 'pending' : 'pending') as 'pending' | 'running' | 'completed',
                      }))

                    if (newNodes && newNodes.length > 0) {
                      currentTaskNodes.value = newNodes
                      currentNodeIndex.value = 0
                      taskStreamDone.value = false
                    }

                    // Render task thought for current task
                    updateOrCreateStep(tempMessage, {
                      type: 'task_thought',
                      content: message.task?.thought || '',
                      streamId: `task_${message.taskId}`,
                      streamDone: message.streamDone,
                    })

                    if (message.streamDone) {
                      tempMessage.content = message.task?.thought || ''
                      taskStreamDone.value = true
                    }
                  }
                }
                break

              case 'agent_start':
                // 代理开始执行，可以添加状态指示
                if (message.type === 'agent_start') {
                  if (!isForCurrentTask(message)) break
                  // 任务开始，将第一个节点设为运行中
                  if (currentTaskNodes.value.length > 0) {
                    updateNodeStatus(0, 'running')
                    currentNodeIndex.value = 0
                  }
                }
                break

              case 'agent_result':
                // 代理执行完成
                if (message.type === 'agent_result') {
                  if (!isForCurrentTask(message)) break
                  if (message.stopReason === 'abort') {
                    // 手动中断：不作为错误处理，标记为完成以关闭流式态
                    console.warn('代理执行已被用户中断')
                    tempMessage.status = 'complete'
                  } else if (message.stopReason === 'error' || message.error) {
                    console.error(`代理执行失败:`, message.error)
                    tempMessage.status = 'error'
                  } else {
                    tempMessage.status = 'complete'
                    // 标记所有节点为已完成
                    currentTaskNodes.value.forEach((_, index) => {
                      updateNodeStatus(index, 'completed')
                    })
                  }
                }
                break

              case 'error':
                // 错误处理
                if (message.type === 'error') {
                  const errorContent =
                    typeof message.error === 'string' ? message.error : String(message.error || '执行过程中发生错误')
                  const errorStep: NonToolStep = {
                    type: 'error',
                    content: errorContent,
                    timestamp: Date.now(),
                    metadata: {
                      errorType: 'execution_error',
                      errorDetails: String(message.error),
                    },
                  }
                  tempMessage.steps?.push(errorStep)
                  tempMessage.status = 'error'
                }
                break

              case 'finish':
                // 执行完成
                if (message.type === 'finish') {
                  tempMessage.status = 'complete'
                  tempMessage.duration = Date.now() - (tempMessage.createdAt?.getTime() || Date.now())
                }
                break
            }

            // 强制触发响应式更新
            const messageIndex = messageList.value.findIndex(m => m.id === tempMessage.id)
            if (messageIndex !== -1) {
              messageList.value[messageIndex] = { ...tempMessage }
            }

            debouncedSaveSteps(tempMessage.id, tempMessage.steps as PersistedStep[])
          } catch (error) {
            console.error('处理流式消息时发生错误:', error)
            // 不要抛出错误，避免中断执行流程
          }
        }

        const callback = createSidebarCallback(handleStreamMessage)

        // 获取当前选中的模型ID
        const selectedModelId = sessionStore.aiState.selectedModelId

        ekoInstance.value = await createTerminalEko({
          callback,
          debug: true,
          selectedModelId,
        })
      }
    } catch (err) {
      // 获取当前选中的模型ID
      const selectedModelId = sessionStore.aiState.selectedModelId
      ekoInstance.value = await createTerminalEko({
        debug: true,
        selectedModelId,
      })
    }
  }
  */

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

    // 首先恢复基本状态
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
    currentConversation,
    messageList,
    streamingContent,
    isLoading,
    error,
    conversations,
    cancelFunction,
    chatMode,
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
    debouncedSaveSteps,
    initialize,
    restoreFromSessionState,
    saveToSessionState,
  }
})
