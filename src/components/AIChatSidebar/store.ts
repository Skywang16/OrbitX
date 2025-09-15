import { aiApi } from '@/api'
import { useAISettingsStore } from '@/components/settings/components/AI'
import { useSessionStore } from '@/stores/session'
import { useTerminalStore } from '@/stores/Terminal'

import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import type { ChatMode } from '@/types'
import { createTerminalEko, createSidebarCallback, type TerminalEko } from '@/eko'
import type { Conversation, Message, ToolStep, NonToolStep } from '@/types'
import { createToolExecution } from '@/types'
import { debounce } from 'lodash-es'
import type { StreamCallbackMessage } from '@/eko/types'

const isToolResultError = (toolResult: unknown): boolean => {
  return (toolResult as { isError?: boolean })?.isError === true
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

  // 任务节点管理
  const currentTaskNodes = ref<
    Array<{
      type: string
      text: string
      status?: 'pending' | 'running' | 'completed'
    }>
  >([])
  const currentTaskId = ref<string | null>(null)
  const currentNodeIndex = ref<number>(0) // 当前执行的节点索引
  const taskStreamDone = ref(false) // 任务流是否完成

  // 节点状态更新函数
  const updateNodeStatus = (nodeIndex: number, status: 'pending' | 'running' | 'completed') => {
    if (currentTaskNodes.value[nodeIndex]) {
      currentTaskNodes.value[nodeIndex].status = status
    }
  }

  // 根据nodeId解析节点信息
  const parseNodeId = (nodeId: string, taskId: string) => {
    if (!nodeId || !taskId) return null

    // 移除taskId前缀
    const suffix = nodeId.replace(`${taskId}_`, '')

    if (suffix.startsWith('node_')) {
      // 具体节点: taskId_node_0, taskId_node_1
      const nodeIndex = parseInt(suffix.replace('node_', ''))
      return { type: 'node', index: nodeIndex }
    } else if (suffix === 'start') {
      return { type: 'start', index: 0 }
    } else if (suffix === 'execution') {
      return { type: 'execution', index: currentNodeIndex.value }
    } else if (suffix === 'thinking') {
      return { type: 'thinking', index: currentNodeIndex.value }
    }

    return null
  }

  // 智能推进节点状态
  const advanceNodeProgress = () => {
    const currentIndex = currentNodeIndex.value
    const totalNodes = currentTaskNodes.value.length

    if (currentIndex < totalNodes) {
      // 标记当前节点为已完成
      updateNodeStatus(currentIndex, 'completed')

      // 如果还有下一个节点，推进到下一个
      if (currentIndex + 1 < totalNodes) {
        currentNodeIndex.value = currentIndex + 1
        updateNodeStatus(currentNodeIndex.value, 'running')
      }
    }
  }

  const chatMode = ref<ChatMode>('chat')
  const ekoInstance = ref<TerminalEko | null>(null)
  const currentAgentId = ref<string | null>(null)

  const isInitialized = ref(false)

  const debouncedSaveSteps = debounce(async (messageId: number, steps: unknown[]) => {
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
        // 清空任务列表
        currentTaskNodes.value = []
        currentTaskId.value = null
        return
      }

      isLoading.value = true
      const conversationId = await aiApi.createConversation(title)
      const newConversation = await aiApi.getConversation(conversationId)
      conversations.value.unshift(newConversation)
      currentConversationId.value = newConversation.id
      messageList.value = []
      // 清空任务列表
      currentTaskNodes.value = []
      currentTaskId.value = null
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
    // 清空任务列表
    currentTaskNodes.value = []
    currentTaskId.value = null
    await loadConversation(conversationId, true)
  }

  const deleteConversation = async (conversationId: number): Promise<void> => {
    try {
      await aiApi.deleteConversation(conversationId)
      conversations.value = conversations.value.filter(c => c.id !== conversationId)

      if (currentConversationId.value === conversationId) {
        currentConversationId.value = null
        messageList.value = []
        // 清空任务列表
        currentTaskNodes.value = []
        currentTaskId.value = null
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

      if (!ekoInstance.value) {
        await initializeEko()
      }

      ekoInstance.value?.setMode(chatMode.value)

      const terminalStore = useTerminalStore()
      const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)

      // 使用 activeTerminal.backendId 作为 paneId 参数，移除 currentWorkingDirectory 逻辑
      const paneId = activeTerminal?.backendId || undefined

      let fullPrompt: string
      try {
        fullPrompt = await aiApi.buildPromptWithContext(currentConversationId.value, content, userMessageId, paneId)
      } catch (contextError) {
        console.warn('获取终端上下文失败，使用回退逻辑:', contextError)
        // 回退逻辑：不传递 paneId，让后端使用默认上下文
        try {
          fullPrompt = await aiApi.buildPromptWithContext(currentConversationId.value, content, userMessageId)
        } catch (fallbackError) {
          console.error('构建AI提示失败:', fallbackError)
          throw new Error('无法构建AI提示，请检查终端状态')
        }
      }

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

  const updateSelectedModel = async (modelId: string | null): Promise<void> => {
    try {
      // 更新EKO实例的模型配置
      if (ekoInstance.value) {
        await ekoInstance.value.setSelectedModelId(modelId)
      }
    } catch (error) {
      console.error('更新模型配置失败:', error)
    }
  }
  // 工具步骤处理相关函数
  const findOrCreateToolStep = (tempMessage: Message, toolName: string, toolId?: string): ToolStep => {
    // 优先根据 toolId 查找，如果没有则根据 toolName 查找最新的运行中工具
    let existingStep = null

    if (toolId) {
      existingStep = tempMessage.steps?.find(
        step => step.type === 'tool_use' && 'toolExecution' in step && step.toolExecution.toolId === toolId
      ) as ToolStep | undefined
    }

    if (!existingStep) {
      existingStep = tempMessage.steps?.find(
        step =>
          step.type === 'tool_use' &&
          'toolExecution' in step &&
          step.toolExecution.name === toolName &&
          step.toolExecution.status === 'running'
      ) as ToolStep | undefined
    }

    if (existingStep) {
      return existingStep
    }

    // 创建新的工具步骤
    const toolExecution = createToolExecution(toolName, {}, 'running')
    // 保存 toolId 以便后续查找
    if (toolId) {
      toolExecution.toolId = toolId
    }

    const newStep: ToolStep = {
      type: 'tool_use',
      content: `Executing ${toolName}...`,
      timestamp: Date.now(),
      toolExecution,
    }
    tempMessage.steps?.push(newStep)

    return newStep
  }

  const updateToolStepParams = (toolStep: ToolStep, params: Record<string, unknown>) => {
    toolStep.toolExecution.params = {
      ...toolStep.toolExecution.params,
      ...params,
    }
  }

  const handleToolUse = (tempMessage: Message, message: StreamCallbackMessage) => {
    if (message.type !== 'tool_use' || !message.toolName) {
      return
    }

    // 使用 toolId 查找已存在的工具步骤（可能由 tool_streaming 创建）
    const toolStep = findOrCreateToolStep(tempMessage, message.toolName, message.toolId)

    // 更新工具状态
    if (toolStep.toolExecution) {
      toolStep.toolExecution.status = 'running'
    }

    if (message.params) {
      updateToolStepParams(toolStep, message.params)
    }
  }

  const handleToolStreaming = (tempMessage: Message, message: StreamCallbackMessage) => {
    if (message.type !== 'tool_streaming' || !message.toolName) return

    // 立即创建或获取工具步骤以显示执行状态，使用 toolId 进行精确匹配
    const toolStep = findOrCreateToolStep(tempMessage, message.toolName, message.toolId)

    // 更新工具状态为"准备参数中"
    if (toolStep.toolExecution) {
      toolStep.toolExecution.status = 'running'
    }

    // 对于 tool_streaming 类型，paramsText 包含参数信息
    // 实时更新参数显示
    if (message.paramsText) {
      try {
        // 尝试解析参数文本（如果是JSON格式）
        const params = JSON.parse(message.paramsText)
        updateToolStepParams(toolStep, params)
      } catch {
        // 如果不是完整JSON，显示当前的参数文本
        updateToolStepParams(toolStep, {
          _streamingParams: message.paramsText,
          _isStreaming: true,
        })
      }
    }
  }

  const handleToolResult = (tempMessage: Message, message: StreamCallbackMessage) => {
    if (message.type !== 'tool_result') {
      return
    }

    // 优先根据 toolId 查找对应的工具步骤
    let toolStep: ToolStep | undefined = undefined
    if (message.toolId) {
      toolStep = tempMessage.steps?.find(
        step => step.type === 'tool_use' && 'toolExecution' in step && step.toolExecution.toolId === message.toolId
      ) as ToolStep | undefined
    }

    if (toolStep && toolStep.toolExecution) {
      const hasError = isToolResultError(message.toolResult)
      toolStep.toolExecution.status = hasError ? 'error' : 'completed'
      toolStep.toolExecution.endTime = Date.now()
      toolStep.toolExecution.result = message.toolResult

      if (hasError) {
        toolStep.toolExecution.error = 'Tool execution failed'
      }

      // 工具执行完成后，推进节点状态
      if (!hasError && currentTaskId.value && message.nodeId) {
        const nodeInfo = parseNodeId(message.nodeId, currentTaskId.value)
        if (nodeInfo && nodeInfo.type === 'execution') {
          // 工具执行成功，可能需要推进到下一个节点
          advanceNodeProgress()
        }
      }
    }
  }

  const updateOrCreateStep = (
    tempMessage: Message,
    stepData: {
      type: 'thinking' | 'text' | 'task_thought' | 'error'
      content: string
      streamId?: string
      streamDone?: boolean
    }
  ) => {
    let targetStep: NonToolStep | undefined = undefined

    // 优化查找逻辑：基于 streamId 和类型查找现有步骤
    if (stepData.streamId) {
      targetStep = tempMessage.steps?.find(
        step => step.type === stepData.type && step.metadata?.streamId === stepData.streamId
      ) as NonToolStep | undefined
    } else if (stepData.type === 'thinking') {
      // 对于 thinking 类型，如果没有 streamId，查找最后一个 thinking 步骤
      const thinkingSteps = tempMessage.steps?.filter(step => step.type === 'thinking') || []
      targetStep = thinkingSteps[thinkingSteps.length - 1] as NonToolStep | undefined
    }

    if (targetStep) {
      // 更新现有步骤
      targetStep.content = stepData.content

      // 如果流式完成，添加完成标记
      if (stepData.streamDone) {
        targetStep.metadata = {
          ...targetStep.metadata,
          streamId: stepData.streamId,
        }

        // 对于 thinking 类型，计算思考持续时间
        if (stepData.type === 'thinking') {
          targetStep.metadata = {
            ...targetStep.metadata,
            thinkingDuration: Date.now() - targetStep.timestamp,
          }
        }
      }
    } else {
      // 创建新步骤
      const newStep: NonToolStep = {
        type: stepData.type,
        content: stepData.content,
        timestamp: Date.now(),
        metadata: {
          ...(stepData.streamId ? { streamId: stepData.streamId } : {}),
        },
      }

      tempMessage.steps?.push(newStep)
    }
  }

  const initializeEko = async (): Promise<void> => {
    try {
      if (!ekoInstance.value) {
        const handleStreamMessage = async (message: StreamCallbackMessage) => {
          try {
            const tempMessage = messageList.value[messageList.value.length - 1]
            if (!tempMessage || tempMessage.role !== 'assistant') {
              return
            }

            tempMessage.steps = tempMessage.steps || []

            // 处理消息
            switch (message.type) {
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
                  updateOrCreateStep(tempMessage, {
                    type: 'task_thought',
                    content: message.task?.thought || '',
                    streamId: `task_${message.taskId}`,
                    streamDone: message.streamDone,
                  })

                  // 更新任务节点 - 只处理 TaskTextNode 类型
                  if (message.task?.nodes && message.task.nodes.length > 0) {
                    currentTaskNodes.value = message.task.nodes
                      .filter(node => node.type === 'normal' && 'text' in node)
                      .map((node, index) => ({
                        type: node.type,
                        text: 'text' in node ? node.text : '',
                        status: (index === 0 ? 'pending' : 'pending') as 'pending' | 'running' | 'completed',
                      }))
                    currentTaskId.value = message.taskId
                    currentNodeIndex.value = 0
                    taskStreamDone.value = false // 重置状态
                  }

                  if (message.streamDone) {
                    tempMessage.content = message.task?.thought || ''
                    taskStreamDone.value = true // 标记任务流完成
                  }
                }
                break

              case 'agent_start':
                // 代理开始执行，可以添加状态指示
                if (message.type === 'agent_start') {
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
                  if (message.error) {
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
                  const errorStep: NonToolStep = {
                    type: 'error',
                    content: '',
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

            debouncedSaveSteps(tempMessage.id, tempMessage.steps)
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
    currentConversation,
    messageList,
    streamingContent,
    isLoading,
    error,
    conversations,
    cancelFunction,
    chatMode,
    ekoInstance,
    currentAgentId,
    currentTaskNodes,
    currentTaskId,
    currentNodeIndex,
    taskStreamDone,
    updateNodeStatus,
    advanceNodeProgress,
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
    updateSelectedModel,
  }
})
