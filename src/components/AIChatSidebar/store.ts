/**
 * AI聊天功能的状态管理
 */

import { ai } from '@/api/ai'
import { useAISettingsStore } from '@/components/settings/components/AI'
import { AI_SESSION_CONFIG } from '@/constants/ai'
import { handleErrorWithMessage } from '@/utils/errorHandler'
import { createStorage } from '@/utils/storage'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { ChatMessage, ChatSession } from './types'

// 聊天历史管理类
class ChatHistoryManager {
  private readonly SESSIONS_KEY = AI_SESSION_CONFIG.STORAGE_KEY
  private readonly storage = createStorage<ChatSession[]>(this.SESSIONS_KEY)

  generateId(): string {
    return `session_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
  }

  save(sessionId: string, message: ChatMessage): void {
    try {
      const sessions = this.loadAllSessions()
      let session = sessions.find(s => s.id === sessionId)

      if (!session) {
        session = {
          id: sessionId,
          title: this.generateSessionTitle(message.content),
          messages: [],
          createdAt: new Date(),
          updatedAt: new Date(),
        }
        sessions.push(session)
      }

      session.messages.push(message)
      session.updatedAt = new Date()
      this.saveAllSessions(sessions)
    } catch (error) {
      console.error('保存聊天消息失败:', error)
    }
  }

  saveAll(sessionId: string, messages: ChatMessage[]): void {
    try {
      const sessions = this.loadAllSessions()
      let session = sessions.find(s => s.id === sessionId)

      if (!session) {
        const firstMessage = messages[0]
        session = {
          id: sessionId,
          title: firstMessage ? this.generateSessionTitle(firstMessage.content) : '新对话',
          messages: [],
          createdAt: new Date(),
          updatedAt: new Date(),
        }
        sessions.push(session)
      }

      session.messages = messages
      session.updatedAt = new Date()
      this.saveAllSessions(sessions)
    } catch (error) {
      console.error('保存聊天会话失败:', error)
    }
  }

  load(sessionId: string): ChatMessage[] {
    try {
      const sessions = this.loadAllSessions()
      const session = sessions.find(s => s.id === sessionId)
      return session ? session.messages : []
    } catch (error) {
      console.error('加载聊天历史失败:', error)
      return []
    }
  }

  loadSessions(): ChatSession[] {
    try {
      return this.loadAllSessions()
    } catch (error) {
      console.error('加载会话列表失败:', error)
      return []
    }
  }

  delete(sessionId: string): void {
    try {
      const sessions = this.loadAllSessions()
      const filteredSessions = sessions.filter(s => s.id !== sessionId)
      this.saveAllSessions(filteredSessions)
    } catch (error) {
      console.error('删除会话失败:', error)
    }
  }

  clear(): void {
    try {
      this.storage.remove()
    } catch (error) {
      console.error('清空聊天历史失败:', error)
    }
  }

  loadAllSessions(): ChatSession[] {
    try {
      const sessions = this.storage.load() || []
      return sessions.map(session => ({
        ...session,
        createdAt: new Date(session.createdAt),
        updatedAt: new Date(session.updatedAt),
        messages: session.messages.map(msg => ({
          ...msg,
          timestamp: new Date(msg.timestamp),
        })),
      }))
    } catch (error) {
      console.error('加载会话数据失败:', error)
      return []
    }
  }

  private saveAllSessions(sessions: ChatSession[]): void {
    try {
      const maxSessions = AI_SESSION_CONFIG.MAX_SESSIONS
      const sortedSessions = sessions
        .sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime())
        .slice(0, maxSessions)

      this.storage.save(sortedSessions)
    } catch (error) {
      console.error('保存会话数据失败:', error)
    }
  }

  private generateSessionTitle(content: string): string {
    const title = content.trim().substring(0, AI_SESSION_CONFIG.TITLE_MAX_LENGTH)
    return title.length < content.trim().length ? title + '...' : title
  }
}

const chatHistory = new ChatHistoryManager()

export const useAIChatStore = defineStore('ai-chat', () => {
  // 状态
  const isVisible = ref(false)
  const sidebarWidth = ref(250)
  const currentSessionId = ref<string | null>(null)
  const messages = ref<ChatMessage[]>([])
  const streamingContent = ref('')
  const isLoading = ref(false)
  const isStreaming = ref(false)
  const error = ref<string | null>(null)
  const sessions = ref<ChatSession[]>([])
  const cancelFunction = ref<(() => void) | null>(null)

  // 计算属性
  const hasMessages = computed(() => messages.value.length > 0)
  const canSendMessage = computed(() => {
    const aiSettingsStore = useAISettingsStore()
    return !isLoading.value && !isStreaming.value && aiSettingsStore.hasModels
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

      if (!currentSessionId.value) {
        createNewSession()
      }
    }
  }

  const showSidebar = async () => {
    isVisible.value = true

    // 确保AI设置已加载
    const aiSettingsStore = useAISettingsStore()
    if (!aiSettingsStore.hasModels && !aiSettingsStore.isLoading) {
      try {
        await aiSettingsStore.loadSettings()
      } catch (_error) {
        // 静默处理加载失败，不影响用户体验
      }
    }

    if (!currentSessionId.value) {
      createNewSession()
    }
  }

  const hideSidebar = () => {
    isVisible.value = false
    saveCurrentSession()
  }

  const createNewSession = () => {
    saveCurrentSession()
    currentSessionId.value = chatHistory.generateId()
    messages.value = []
    error.value = null
  }

  const loadSession = (sessionId: string) => {
    saveCurrentSession()
    currentSessionId.value = sessionId
    messages.value = chatHistory.load(sessionId)
    error.value = null
  }

  const deleteSession = (sessionId: string) => {
    chatHistory.delete(sessionId)
    loadSessions()

    if (currentSessionId.value === sessionId) {
      createNewSession()
    }
  }

  const loadSessions = () => {
    sessions.value = chatHistory.loadSessions()
  }

  const saveCurrentSession = () => {
    if (currentSessionId.value && messages.value.length > 0) {
      chatHistory.saveAll(currentSessionId.value, messages.value)
    }
  }

  const generateMessageId = () => {
    return `msg_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
  }

  const sendMessage = async (content: string) => {
    if (!canSendMessage.value || !currentSessionId.value) {
      return
    }

    const userMessage: ChatMessage = {
      id: generateMessageId(),
      messageType: 'user',
      content: content.trim(),
      timestamp: new Date(),
    }

    messages.value.push(userMessage)
    chatHistory.save(currentSessionId.value, userMessage)

    const aiMessage: ChatMessage = {
      id: generateMessageId(),
      messageType: 'assistant',
      content: '',
      timestamp: new Date(),
    }

    messages.value.push(aiMessage)

    // 获取消息在数组中的索引，用于后续更新
    const messageIndex = messages.value.length - 1

    try {
      isLoading.value = true
      isStreaming.value = true
      error.value = null
      streamingContent.value = '' // 重置流式内容

      const aiSettingsStore = useAISettingsStore()

      // 设置超时机制，防止流式响应卡住
      const streamTimeout = setTimeout(() => {
        if (isStreaming.value) {
          isStreaming.value = false
          if (currentSessionId.value) {
            chatHistory.save(currentSessionId.value, aiMessage)
          }
        }
      }, 30000) // 30秒超时

      const { cancel } = await ai.streamMessageCancellable(
        content,
        (chunk: { content?: string; isComplete?: boolean; metadata?: unknown }) => {
          // 检查是否是流式响应开始信号
          if (chunk.metadata && typeof chunk.metadata === 'object' && 'stream_started' in chunk.metadata) {
            return
          }

          if (chunk.content) {
            // 累积流式内容
            streamingContent.value += chunk.content

            // 强制更新消息数组，确保Vue响应式更新
            const currentMessage = messages.value[messageIndex]
            messages.value.splice(messageIndex, 1, {
              ...currentMessage,
              content: streamingContent.value,
            })
          }

          if (chunk.isComplete) {
            clearTimeout(streamTimeout)
            isStreaming.value = false
            if (currentSessionId.value) {
              chatHistory.save(currentSessionId.value, messages.value[messageIndex])
            }
          }
        },
        aiSettingsStore.defaultModel?.id
      )

      // 保存取消函数
      cancelFunction.value = cancel

      // 清除超时器
      clearTimeout(streamTimeout)

      // 确保流式状态被重置（防止后端没有发送完成标志）
      if (isStreaming.value) {
        isStreaming.value = false
        if (currentSessionId.value && messageIndex < messages.value.length) {
          chatHistory.save(currentSessionId.value, messages.value[messageIndex])
        }
      }
    } catch (err) {
      error.value = handleErrorWithMessage(err, '发送消息失败')
      // 移除失败的AI消息
      if (messageIndex < messages.value.length) {
        messages.value.splice(messageIndex, 1)
      }
      // 确保在错误时重置流式状态
      isStreaming.value = false
    } finally {
      isLoading.value = false
      // 最终确保流式状态被重置
      isStreaming.value = false
      cancelFunction.value = null
    }
  }

  const clearCurrentSession = () => {
    messages.value = []
    if (currentSessionId.value) {
      chatHistory.delete(currentSessionId.value)
      createNewSession()
    }
  }

  const setSidebarWidth = (width: number) => {
    sidebarWidth.value = Math.max(100, Math.min(800, width))
  }

  const stopStreaming = () => {
    if (isStreaming.value && cancelFunction.value) {
      cancelFunction.value()
      cancelFunction.value = null
      isStreaming.value = false
      isLoading.value = false
    }
  }

  // 初始化
  const initialize = () => {
    loadSessions()
  }

  return {
    // 状态
    isVisible,
    sidebarWidth,
    currentSessionId,
    messages,
    streamingContent,
    isLoading,
    isStreaming,
    error,
    sessions,

    // 计算属性
    hasMessages,
    canSendMessage,

    // 操作方法
    toggleSidebar,
    showSidebar,
    hideSidebar,
    createNewSession,
    loadSession,
    deleteSession,
    loadSessions,
    saveCurrentSession,
    sendMessage,
    stopStreaming,
    clearCurrentSession,
    setSidebarWidth,
    initialize,
  }
})
