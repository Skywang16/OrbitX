/**
 * AI聊天功能的状态管理 - 完全重构版本
 *
 * 使用新的会话上下文管理系统，不再向后兼容
 */

import { conversations as conversationAPI } from '@/api/ai'
import { useAISettingsStore } from '@/components/settings/components/AI'
import { useSessionStore } from '@/stores/session'
import { handleErrorWithMessage } from '@/utils/errorHandler'
import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import type { ChatMode } from './types'
import { createTerminalEko, createSidebarCallback, type TerminalEko } from '@/eko'
import type { Conversation, Message } from '@/types/features/ai/chat'

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
      const conversationId = await conversationAPI.create(title)
      const newConversation = await conversationAPI.get(conversationId)
      conversations.value.unshift(newConversation)
      currentConversationId.value = newConversation.id
      messageList.value = []
    } catch (err) {
      error.value = handleErrorWithMessage(err, '创建会话失败')
    } finally {
      isLoading.value = false
    }
  }

  const loadConversation = async (conversationId: number): Promise<void> => {
    try {
      isLoading.value = true
      currentConversationId.value = conversationId

      // 使用新的API获取压缩上下文作为消息历史
      const loadedMessages = await conversationAPI.getCompressedContext(conversationId)
      messageList.value = loadedMessages
    } catch (err) {
      error.value = handleErrorWithMessage(err, '加载会话失败')
    } finally {
      isLoading.value = false
    }
  }

  const deleteConversation = async (conversationId: number): Promise<void> => {
    try {
      await conversationAPI.delete(conversationId)
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
      conversations.value = await conversationAPI.getList()
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
      await conversationAPI.saveMessage(currentConversationId.value, 'user', content)

      // 2. 立即更新UI显示用户消息
      await loadConversation(currentConversationId.value)

      // 3. 确保Eko实例可用（如果未初始化则自动初始化）
      if (!ekoInstance.value) {
        console.log('Eko实例未初始化，正在自动初始化...')
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
      const contextMessages = await conversationAPI.getCompressedContext(currentConversationId.value)

      // 6. 构建完整的prompt（包含上下文，不重复当前用户消息）
      const fullPrompt =
        contextMessages.length > 0
          ? contextMessages.map(msg => `${msg.role}: ${msg.content}`).join('\n')
          : `user: ${content}`

      // 7. 创建临时AI消息（使用新的数据结构）
      const tempAIMessage: Message = {
        id: Date.now(),
        conversationId: currentConversationId.value,
        role: 'assistant' as const,
        createdAt: new Date(),
        steps: [],
        status: 'streaming',
      }

      // 添加临时消息到列表
      messageList.value.push(tempAIMessage)

      // 8. 通过eko处理消息（流式输出通过回调处理）
      streamingContent.value = ''
      const response = await ekoInstance.value.run(fullPrompt)

      // 9. 保存AI回复到数据库
      if (response.success && response.result) {
        // 更新占位消息内容
        tempAIMessage.content = response.result
        await conversationAPI.saveMessage(currentConversationId.value, 'assistant', response.result)
      }

      // 10. 重新加载消息
      await loadConversation(currentConversationId.value)

      // 11. 刷新会话列表以更新预览
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
      await conversationAPI.truncateConversation(currentConversationId.value, truncateAfterMessageId)

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
        const handleStreamMessage = async (message: any) => {
          const tempMessage = messageList.value[messageList.value.length - 1]
          if (!tempMessage || tempMessage.role !== 'assistant') return

          if (message.type === 'tool_use') {
            // 详细打印工具调用消息结构
            console.log('🔧 Tool Use Message:', JSON.stringify(message, null, 2))

            // 处理工具调用 - 创建工具步骤
            tempMessage.steps?.push({
              type: 'tool_use',
              content: '正在调用工具...',
              timestamp: Date.now(),
              metadata: {
                toolName: message.toolName || '工具调用',
                toolCommand: message.params?.command || JSON.stringify(message.params || {}),
                status: 'running',
                originalMessage: message, // 保存原始消息用于调试
              },
            })
          } else if (message.type === 'tool_result') {
            // 详细打印工具结果消息结构
            console.log('✅ Tool Result Message:', JSON.stringify(message, null, 2))

            // 更新现有工具步骤的结果
            let toolStep = tempMessage.steps?.find(step => step.type === 'tool_use')
            if (toolStep) {
              toolStep.content = '工具执行完成'
              toolStep.metadata = {
                ...toolStep.metadata,
                status: 'completed',
                toolResult: message,
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
                },
              }

              // 如果thinking瞬间完成，记录0持续时间
              if (message.streamDone) {
                newStep.metadata = {
                  ...newStep.metadata,
                  thinkingDuration: 0,
                }
              }

              tempMessage.steps?.push(newStep)
            }
          } else if (message.type === 'text' && !message.streamDone) {
            // 处理文本步骤
            let textStep = tempMessage.steps?.find(step => step.type === 'text')
            if (textStep) {
              textStep.content = message.text
              textStep.timestamp = Date.now()
            } else {
              tempMessage.steps?.push({
                type: 'text',
                content: message.text,
                timestamp: Date.now(),
              })
            }
            streamingContent.value = message.text
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

  // 从会话状态恢复 OrbitX 状态
  const restoreFromSessionState = (): void => {
    if (!sessionStore.initialized) return

    const orbitxState = sessionStore.sessionState.uiState.orbitxChat
    if (orbitxState) {
      isVisible.value = orbitxState.isVisible
      sidebarWidth.value = orbitxState.sidebarWidth
      chatMode.value = orbitxState.chatMode
      currentConversationId.value = orbitxState.currentConversationId
    }
  }

  // 将当前状态保存到会话系统
  const saveToSessionState = (): void => {
    if (!sessionStore.initialized) return

    // 更新会话状态中的 OrbitX 状态
    sessionStore.sessionState.uiState.orbitxChat = {
      isVisible: isVisible.value,
      sidebarWidth: sidebarWidth.value,
      chatMode: chatMode.value,
      currentConversationId: currentConversationId.value,
    }

    // 触发会话状态保存
    sessionStore.saveSessionState().catch(() => {
      /* ignore: 后台保存失败不打扰用户 */
    })
  }

  // 监听状态变化并自动保存
  watch([isVisible, sidebarWidth, chatMode, currentConversationId], () => {
    if (isInitialized.value) {
      saveToSessionState()
    }
  })

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
          await loadConversation(currentConversationId.value)
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
