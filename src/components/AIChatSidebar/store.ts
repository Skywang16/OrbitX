/**
 * AI聊天功能的状态管理 - 完全重构版本
 *
 * 使用新的会话上下文管理系统，不再向后兼容
 */

import { ai } from '@/api/ai'
import { useAISettingsStore } from '@/components/settings/components/AI'
import { useSessionStore } from '@/stores/session'
import { handleErrorWithMessage } from '@/utils/errorHandler'
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import type { ChatMode } from './types'
import { createTerminalEko, createSidebarCallback, type TerminalEko } from '@/eko'
import type { Conversation, Message } from '@/types/features/ai/chat'
import { debounce } from 'lodash-es'

// 流式消息类型定义
interface StreamMessage {
  type: 'tool_use' | 'tool_result' | 'workflow' | 'text'
  toolName?: string
  params?: Record<string, any>
  toolResult?: any
  workflow?: {
    name?: string
    thought?: string
  }
  agentName?: string
  taskId?: string
  text?: string
  streamDone?: boolean
}

// 工具函数
const generateSessionTitle = (content: string): string => {
  const title = content.trim().slice(0, 20)
  if (title.length === 0) return '新对话'
  return title.length < content.trim().length ? title + '...' : title
}

export const useAIChatStore = defineStore('ai-chat', () => {
  const sessionStore = useSessionStore()

  // 状态
  const isVisible = ref(false)
  const sidebarWidth = ref(350)
  const currentConversationId = ref<number | null>(null)
  const messageList = ref<Message[]>([])
  const streamingContent = ref('')
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const conversations = ref<Conversation[]>([])
  const cancelFunction = ref<(() => void) | null>(null)

  // 聊天模式相关状态
  const chatMode = ref<ChatMode>('chat')
  const ekoInstance = ref<TerminalEko | null>(null)
  const currentAgentId = ref<string | null>(null)

  // 初始化标志
  const isInitialized = ref(false)

  // 计算属性
  const hasMessages = computed(() => messageList.value.length > 0)
  const canSendMessage = computed(() => {
    const aiSettingsStore = useAISettingsStore()
    return !isLoading.value && aiSettingsStore.hasModels
  })

  // 操作方法
  const toggleSidebar = async () => {
    isVisible.value = !isVisible.value
    if (isVisible.value) {
      // 确保AI设置已加载
      const aiSettingsStore = useAISettingsStore()
      if (!aiSettingsStore.hasModels && !aiSettingsStore.isLoading) {
        try {
          await aiSettingsStore.loadSettings()
        } catch (_error) {
          /* ignore: 静默处理加载失败，不影响用户体验 */
        }
      }

      // 加载会话列表
      await refreshConversations()
    }
  }

  const setSidebarWidth = (width: number) => {
    sidebarWidth.value = Math.max(300, Math.min(800, width))
  }

  // 会话管理方法
  const createConversation = async (title?: string): Promise<void> => {
    try {
      isLoading.value = true
      const conversationId = await ai.conversations.create(title)
      const newConversation = await ai.conversations.get(conversationId)
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

      const loadedMessages = await ai.conversations.getCompressedContext(conversationId)

      if (forceReload) {
        // 强制重新加载：完全替换消息列表
        messageList.value = loadedMessages
      } else {
        // 增量更新：保留现有消息的步骤信息，只添加新消息
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

  // 会话切换方法
  const switchToConversation = async (conversationId: number): Promise<void> => {
    messageList.value = []
    await loadConversation(conversationId, true)
  }

  const deleteConversation = async (conversationId: number): Promise<void> => {
    try {
      await ai.conversations.delete(conversationId)
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
      conversations.value = await ai.conversations.getList()
    } catch (err) {
      error.value = handleErrorWithMessage(err, '刷新会话列表失败')
    }
  }

  // 发送消息方法（统一通过eko处理）
  const sendMessage = async (content: string): Promise<void> => {
    if (!currentConversationId.value) {
      // 如果没有当前会话，创建一个新会话
      const title = generateSessionTitle(content)
      await createConversation(title)
    }

    if (!currentConversationId.value) {
      throw new Error('无法创建会话')
    }

    try {
      isLoading.value = true
      error.value = null

      // 1. 立即保存用户消息（不等待Eko初始化）
      const userMessageId = await ai.conversations.saveMessage(currentConversationId.value, 'user', content)

      // 2. 立即更新UI显示用户消息（添加到当前消息列表而不是重新加载）
      const userMessage: Message = {
        id: userMessageId,
        conversationId: currentConversationId.value,
        role: 'user',
        content,
        createdAt: new Date(),
      }
      messageList.value.push(userMessage)

      // 3. 确保Eko实例可用（如果未初始化则自动初始化）
      if (!ekoInstance.value) {
        await initializeEko()

        // 如果初始化后仍然没有实例，则抛出错误
        if (!ekoInstance.value) {
          throw new Error('Eko实例初始化失败')
        }
      }

      // 4. 根据模式设置只读/全权限工具（若失败不影响整体发送流程）
      try {
        ekoInstance.value.setMode(chatMode.value)
      } catch {
        /* ignore */
      }

      // 5. 获取压缩上下文
      const contextMessages = await ai.conversations.getCompressedContext(currentConversationId.value)

      // 6. 构建完整的prompt（包含上下文，不重复当前用户消息）
      const fullPrompt =
        contextMessages.length > 0
          ? contextMessages.map(msg => `${msg.role}: ${msg.content}`).join('\n')
          : `user: ${content}`

      // 7. 创建临时AI消息
      const tempAIMessage: Message = {
        id: -Date.now(), // 使用负数作为临时ID，避免与数据库ID冲突
        conversationId: currentConversationId.value,
        role: 'assistant',
        createdAt: new Date(),
        steps: [],
        status: 'streaming',
      }

      // 添加临时消息到列表
      messageList.value.push(tempAIMessage)

      // 8. 通过eko处理消息（流式输出通过回调处理）
      streamingContent.value = ''
      const response = await ekoInstance.value.run(fullPrompt)

      // 9. 保存AI回复到数据库并更新消息状态
      if (response.success && response.result) {
        // 更新临时消息的内容和状态
        tempAIMessage.content = response.result
        tempAIMessage.status = 'complete'
        tempAIMessage.duration = Date.now() - tempAIMessage.createdAt.getTime()

        // 保存到数据库并更新消息ID
        const messageId = await ai.conversations.saveMessage(currentConversationId.value, 'assistant', response.result)
        tempAIMessage.id = messageId

        // 增量持久化：将steps/status/duration落库，保证历史回放一致
        try {
          await ai.conversations.updateMessageMeta(
            messageId,
            tempAIMessage.steps || [],
            tempAIMessage.status,
            tempAIMessage.duration
          )
        } catch {
          // 静默失败，不影响前端展示
        }
      } else {
        // 处理失败情况
        tempAIMessage.status = 'error'
        tempAIMessage.steps?.push({
          type: 'error',
          content: '消息发送失败',
          timestamp: Date.now(),
          metadata: {
            errorType: 'SendError',
            errorDetails: response.error || '未知错误',
          },
        })
      }

      // 10. 刷新会话列表以更新预览（不重新加载消息，保持步骤信息）
      await refreshConversations()
    } catch (err) {
      error.value = handleErrorWithMessage(err, '发送消息失败')
      throw err
    } finally {
      isLoading.value = false
    }
  }

  // 截断重问方法（使用新的eko架构）
  const truncateAndResend = async (truncateAfterMessageId: number, newContent: string): Promise<void> => {
    if (!currentConversationId.value) {
      throw new Error('没有选择会话')
    }

    try {
      isLoading.value = true
      error.value = null

      // 1. 截断会话
      await ai.conversations.truncateConversation(currentConversationId.value, truncateAfterMessageId)

      // 2. 发送新消息（复用sendMessage逻辑）
      await sendMessage(newContent)
    } catch (err) {
      error.value = handleErrorWithMessage(err, '截断重问失败')
      throw err
    } finally {
      isLoading.value = false
    }
  }

  // 清空错误
  const clearError = (): void => {
    error.value = null
  }

  // 初始化Eko实例（带流式回调）
  const initializeEko = async (): Promise<void> => {
    try {
      if (!ekoInstance.value) {
        // 处理流式消息更新UI
        const handleStreamMessage = async (message: StreamMessage) => {
          const tempMessage = messageList.value[messageList.value.length - 1]
          if (!tempMessage || tempMessage.role !== 'assistant') return

          // 确保steps数组存在
          if (!tempMessage.steps) {
            tempMessage.steps = []
          }

          if (message.type === 'tool_use') {
            tempMessage.steps.push({
              type: 'tool_use',
              content: `正在调用工具: ${message.toolName}`,
              timestamp: Date.now(),
              metadata: {
                toolName: message.toolName || '工具调用',
                toolParams: message.params || {},
                toolCommand: message.params?.command || message.params?.path || '',
                status: 'running',
              },
            })
          } else if (message.type === 'tool_result') {
            // 找到最后一个运行中的工具步骤
            const runningSteps = tempMessage.steps.filter(
              step => step.type === 'tool_use' && step.metadata?.status === 'running'
            )
            const toolStep = runningSteps[runningSteps.length - 1]

            if (toolStep) {
              toolStep.content = `工具执行完成: ${toolStep.metadata?.toolName}`
              toolStep.metadata = {
                ...toolStep.metadata,
                status: 'completed',
                completedAt: Date.now(),
                toolResult: message.toolResult,
              }
            }
          } else if (message.type === 'workflow' && message.workflow?.thought) {
            // 处理思考步骤
            let thinkingStep = tempMessage.steps?.find(step => step.type === 'thinking')
            if (thinkingStep) {
              thinkingStep.content = message.workflow.thought

              // 如果thinking完成，记录持续时间
              if (message.streamDone) {
                thinkingStep.metadata = {
                  ...thinkingStep.metadata,
                  thinkingDuration: Date.now() - thinkingStep.timestamp,
                }
              }
            } else {
              const newStep = {
                type: 'thinking' as const,
                content: message.workflow.thought,
                timestamp: Date.now(),
                metadata: {
                  workflowName: message.workflow.name,
                  agentName: message.agentName,
                  taskId: message.taskId,
                  // 如果thinking瞬间完成，记录0持续时间
                  thinkingDuration: message.streamDone ? 0 : undefined,
                },
              }

              tempMessage.steps?.push(newStep)
            }
          } else if (message.type === 'text' && message.text !== undefined) {
            // 瀑布流逻辑：只有当最后一个step就是text类型时，才更新它
            // 如果最后一个step是tool/thinking等其他类型，则创建新的text step
            const lastStep = tempMessage.steps?.[tempMessage.steps.length - 1]
            const isCurrentRoundText = lastStep?.type === 'text'

            if (isCurrentRoundText) {
              // 同一轮text流式更新：更新最后一个text step
              lastStep.content = message.text
              lastStep.timestamp = Date.now()
            } else {
              // 新一轮text输出：创建新的text step
              tempMessage.steps?.push({
                type: 'text',
                content: message.text,
                timestamp: Date.now(),
              })
            }
            streamingContent.value = message.text

            // 如果文本流完成，更新消息状态
            if (message.streamDone) {
              tempMessage.status = 'complete'
              tempMessage.content = message.text
            }
          }
        }

        // 使用回调工厂
        const callback = createSidebarCallback(handleStreamMessage)

        ekoInstance.value = await createTerminalEko({
          callback,
          debug: true,
        })
      }
    } catch (err) {
      // 创建fallback实例
      try {
        ekoInstance.value = await createTerminalEko({ debug: true })
      } catch {
        // 完全失败，保持null
      }
    }
  }

  // 从会话状态恢复 AI 状态
  const restoreFromSessionState = (): void => {
    if (!sessionStore.initialized) return

    const aiState = sessionStore.aiState
    if (aiState) {
      isVisible.value = aiState.visible
      sidebarWidth.value = aiState.width
      chatMode.value = aiState.mode as 'chat' | 'agent'
      currentConversationId.value = aiState.conversationId || null
    }
  }

  // 将当前状态保存到会话系统
  const saveToSessionState = (): void => {
    if (!sessionStore.initialized) return

    // 更新会话状态中的 AI 状态
    sessionStore.updateAiState({
      visible: isVisible.value,
      width: sidebarWidth.value,
      mode: chatMode.value,
      conversationId: currentConversationId.value || undefined,
    })
  }

  // 使用lodash防抖保存函数，避免频繁保存
  const debouncedSave = debounce(() => {
    if (isInitialized.value) {
      saveToSessionState()
    }
  }, 300)

  // 监听状态变化并自动保存（防抖）
  watch([isVisible, sidebarWidth, chatMode, currentConversationId], debouncedSave)

  // 初始化方法
  const initialize = async (): Promise<void> => {
    if (isInitialized.value) return

    try {
      // 等待会话Store初始化
      if (!sessionStore.initialized) {
        await sessionStore.initialize()
      }

      // 从会话状态恢复
      restoreFromSessionState()

      // 如果恢复了当前会话ID，尝试加载会话
      if (currentConversationId.value) {
        try {
          await switchToConversation(currentConversationId.value)
        } catch (err) {
          currentConversationId.value = null
        }
      }

      // 加载会话列表
      await refreshConversations()

      isInitialized.value = true
    } catch (err) {
      handleErrorWithMessage(err, 'AI聊天初始化失败')
    }
  }

  return {
    // 状态
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

    // 计算属性
    hasMessages,
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
    truncateAndResend,
    clearError,
    initializeEko,
    initialize,
    restoreFromSessionState,
    saveToSessionState,
  }
})
