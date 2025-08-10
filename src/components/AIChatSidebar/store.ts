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
import { createDebugTerminalEko, type TerminalEko } from '@/eko'
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
  const messages = ref<Message[]>([])
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
  const hasMessages = computed(() => messages.value.length > 0)
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
          // 静默处理加载失败，不影响用户体验
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
      messages.value = []
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
      messages.value = await conversationAPI.getCompressedContext(conversationId)
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
        messages.value = []
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

      // 确保eko实例存在
      if (!ekoInstance.value) {
        await initializeEko()
      }

      if (!ekoInstance.value) {
        throw new Error('Eko实例初始化失败')
      }

      // 1. 根据模式设置只读/全权限工具
      try {
        ekoInstance.value.setMode(chatMode.value)
      } catch (_) {
        // 忽略
      }

      // 2. 保存用户消息
      await conversationAPI.saveMessage(currentConversationId.value, 'user', content)

      // 3. 获取压缩上下文
      const contextMessages = await conversationAPI.getCompressedContext(currentConversationId.value)

      // 4. 构建完整的prompt（包含上下文，不重复当前用户消息）
      const fullPrompt =
        contextMessages.length > 0
          ? contextMessages.map(msg => `${msg.role}: ${msg.content}`).join('\n')
          : `user: ${content}`

      // 5. 通过eko处理消息（传递完整上下文）
      const response = await ekoInstance.value.run(fullPrompt)

      // 6. 保存AI回复
      if (response.success && response.result) {
        await conversationAPI.saveMessage(currentConversationId.value, 'assistant', response.result)
      }

      // 7. 重新加载当前会话的消息
      await loadConversation(currentConversationId.value)

      // 8. 刷新会话列表以更新预览
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

  // 初始化Eko实例（保持原有功能）
  const initializeEko = async (): Promise<void> => {
    try {
      if (!ekoInstance.value) {
        ekoInstance.value = await createDebugTerminalEko()
      }
    } catch (err) {
      // 静默处理错误
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
    sessionStore.saveSessionState().catch(err => {
      console.warn('保存 OrbitX 状态失败:', err)
    })
  }

  // 监听状态变化并自动保存
  watch(
    [isVisible, sidebarWidth, chatMode, currentConversationId],
    () => {
      if (isInitialized.value) {
        saveToSessionState()
      }
    },
    { deep: true }
  )

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
      console.warn('OrbitX Store 初始化失败:', err)
    }
  }

  return {
    // 状态
    isVisible,
    sidebarWidth,
    currentConversationId,
    messages,
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
