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
import type { ChatMode } from './types'
import { createDebugTerminalEko, type TerminalEko } from '@/eko'

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
        // 新会话添加到数组开头，确保它是最新的
        sessions.unshift(session)
      } else {
        // 更新现有会话的时间戳
        session.updatedAt = new Date()
        // 将更新的会话移到数组开头
        const sessionIndex = sessions.findIndex(s => s.id === sessionId)
        if (sessionIndex > 0) {
          sessions.splice(sessionIndex, 1)
          sessions.unshift(session)
        }
      }

      session.messages.push(message)
      this.saveAllSessions(sessions)
    } catch (error) {
      // 保存聊天消息失败
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
        // 新会话添加到数组开头
        sessions.unshift(session)
      } else {
        // 更新现有会话的时间戳
        session.updatedAt = new Date()
        // 将更新的会话移到数组开头
        const sessionIndex = sessions.findIndex(s => s.id === sessionId)
        if (sessionIndex > 0) {
          sessions.splice(sessionIndex, 1)
          sessions.unshift(session)
        }
      }

      session.messages = messages
      this.saveAllSessions(sessions)
    } catch (error) {
      // 保存聊天会话失败
    }
  }

  load(sessionId: string): ChatMessage[] {
    try {
      const sessions = this.loadAllSessions()
      const session = sessions.find(s => s.id === sessionId)
      return session ? session.messages : []
    } catch (error) {
      // 加载聊天历史失败
      return []
    }
  }

  loadSessions(): ChatSession[] {
    try {
      const sessions = this.loadAllSessions()
      // 按更新时间降序排列，最新的在前面
      return sessions.sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime())
    } catch (error) {
      // 加载会话列表失败
      return []
    }
  }

  delete(sessionId: string): void {
    try {
      const sessions = this.loadAllSessions()
      const filteredSessions = sessions.filter(s => s.id !== sessionId)
      this.saveAllSessions(filteredSessions)
    } catch (error) {
      // 删除会话失败
    }
  }

  clear(): void {
    try {
      this.storage.remove()
    } catch (error) {
      // 清空聊天历史失败
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
      // 加载会话数据失败
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
      // 保存会话数据失败
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
  const sidebarWidth = ref(350)
  const currentSessionId = ref<string | null>(null)
  const messages = ref<ChatMessage[]>([])
  const streamingContent = ref('')
  const isLoading = ref(false)

  const error = ref<string | null>(null)
  const sessions = ref<ChatSession[]>([])
  const cancelFunction = ref<(() => void) | null>(null)

  // 聊天模式相关状态
  const chatMode = ref<ChatMode>('chat')
  const ekoInstance = ref<TerminalEko | null>(null)
  const currentAgentId = ref<string | null>(null)

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

      // 智能选择会话：如果当前没有会话或没有消息内容，则选择第一个历史会话或创建新会话
      if (!currentSessionId.value || !hasMessages.value) {
        loadSessions() // 先加载会话列表
        const firstSession = getFirstSession()
        if (firstSession && !hasMessages.value) {
          // 只有在当前没有消息时才加载历史会话
          loadSession(firstSession.id)
        } else if (!currentSessionId.value) {
          // 如果没有当前会话ID，创建新会话
          createNewSession()
        }
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

    // 初始化Eko框架（如果还未初始化）
    await initializeEkoFramework()

    // 智能选择会话：如果当前没有会话或没有消息内容，则选择第一个历史会话或创建新会话
    if (!currentSessionId.value || !hasMessages.value) {
      loadSessions() // 先加载会话列表
      const firstSession = getFirstSession()
      if (firstSession && !hasMessages.value) {
        // 只有在当前没有消息时才加载历史会话
        loadSession(firstSession.id)
      } else if (!currentSessionId.value) {
        // 如果没有当前会话ID，创建新会话
        createNewSession()
      }
    }
  }

  const hideSidebar = () => {
    isVisible.value = false
    saveCurrentSession()
  }

  const createNewSession = () => {
    saveCurrentSession()
    const newSessionId = chatHistory.generateId()
    currentSessionId.value = newSessionId
    messages.value = []
    error.value = null

    // 创建新会话后立即刷新会话列表，确保新会话出现在列表中
    loadSessions()
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

  // 获取第一个会话（最新的会话）
  const getFirstSession = (): ChatSession | null => {
    const sortedSessions = chatHistory.loadSessions()
    return sortedSessions.length > 0 ? sortedSessions[0] : null
  }

  // 从后端刷新会话列表
  const refreshSessions = async () => {
    try {
      const sessionIds = await ai.getChatSessions()
      const localSessions = chatHistory.loadSessions()
      const refreshedSessions: ChatSession[] = []

      for (const sessionId of sessionIds) {
        const localSession = localSessions.find(s => s.id === sessionId)
        const messages = await ai.getChatHistory(sessionId)

        const convertedMessages: ChatMessage[] = messages.map((msg: any) => ({
          id: msg.id,
          messageType: msg.messageType,
          content: msg.content,
          timestamp: new Date(msg.timestamp),
          metadata: msg.metadata,
        }))

        refreshedSessions.push({
          id: sessionId,
          title: localSession?.title || convertedMessages[0]?.content.substring(0, 30) || '未命名会话',
          messages: convertedMessages,
          createdAt: localSession?.createdAt || new Date(),
          updatedAt:
            convertedMessages.length > 0
              ? new Date(Math.max(...convertedMessages.map(m => m.timestamp.getTime())))
              : new Date(),
        })
      }

      sessions.value = refreshedSessions
      return refreshedSessions
    } catch (error) {
      // 刷新会话列表失败
      loadSessions()
      return sessions.value
    }
  }

  const saveCurrentSession = () => {
    if (currentSessionId.value && messages.value.length > 0) {
      chatHistory.saveAll(currentSessionId.value, messages.value)
    }
  }

  const generateMessageId = () => {
    return `msg_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
  }

  // 处理普通聊天消息
  const handleChatMessage = async (content: string, messageIndex: number, aiSettingsStore: any) => {
    const { cancel } = await ai.streamMessageCancellable(
      content,
      (chunk: { content?: string; isComplete?: boolean; metadata?: unknown }) => {
        // 检查是否是流式响应开始信号
        if (chunk.metadata && typeof chunk.metadata === 'object' && 'stream_started' in chunk.metadata) {
          return
        }

        // 检查是否包含错误信息
        if (chunk.metadata && typeof chunk.metadata === 'object' && 'error' in chunk.metadata) {
          const errorInfo = (chunk.metadata as any).error
          // AI响应错误

          // 显示详细错误信息
          const errorMessage = `${errorInfo.message || '未知错误'}`
          const errorDetails = errorInfo.providerResponse
            ? `\n详细信息: ${JSON.stringify(errorInfo.providerResponse, null, 2)}`
            : ''

          // 直接更新消息内容为错误信息
          messages.value[messageIndex].content = `❌ ${errorMessage}${errorDetails}`

          if (currentSessionId.value) {
            chatHistory.save(currentSessionId.value, messages.value[messageIndex])
          }
          return
        }

        if (chunk.content) {
          // 累积流式内容
          streamingContent.value += chunk.content

          // 直接更新消息内容，避免使用splice
          messages.value[messageIndex].content = streamingContent.value
        }

        if (chunk.isComplete) {
          if (currentSessionId.value) {
            chatHistory.save(currentSessionId.value, messages.value[messageIndex])
          }
        }
      },
      aiSettingsStore.defaultModel?.id
    )

    // 保存取消函数
    cancelFunction.value = cancel
  }

  // 处理Agent消息 - 使用Eko框架
  const handleAgentMessage = async (content: string, messageIndex: number) => {
    try {
      if (!ekoInstance.value || !currentSessionId.value) {
        throw new Error('Eko instance not initialized or session ID is missing')
      }

      console.log('🚀 [Eko] 开始执行任务:', content)

      // 执行任务
      const result = await ekoInstance.value.run(content, {
        timeout: 30000, // 30秒超时
      })

      console.log('✅ [Eko] 任务执行完成:', result)

      // 更新消息内容
      if (result.success && result.result) {
        messages.value[messageIndex].content = result.result
      } else {
        messages.value[messageIndex].content = `❌ 任务执行失败: ${result.error || '未知错误'}`
      }

      // 保存消息
      if (currentSessionId.value) {
        chatHistory.save(currentSessionId.value, messages.value[messageIndex])
      }
    } catch (error) {
      console.error('❌ [Eko] 任务执行失败:', error)

      // 更新消息内容为错误信息
      const errorMessage = error instanceof Error ? error.message : String(error)
      messages.value[messageIndex].content = `❌ 任务执行失败: ${errorMessage}`

      // 保存错误消息
      if (currentSessionId.value) {
        chatHistory.save(currentSessionId.value, messages.value[messageIndex])
      }
    }
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

      error.value = null
      streamingContent.value = '' // 重置流式内容

      const aiSettingsStore = useAISettingsStore()

      // 根据聊天模式选择不同的处理方式

      if (chatMode.value === 'agent' && ekoInstance.value) {
        // Agent模式：使用Eko框架处理
        await handleAgentMessage(content, messageIndex)
      } else {
        // 普通聊天模式：使用原有的AI API
        await handleChatMessage(content, messageIndex, aiSettingsStore)
      }
    } catch (err) {
      error.value = handleErrorWithMessage(err, '发送消息失败')
      // 移除失败的AI消息
      if (messageIndex < messages.value.length) {
        messages.value.splice(messageIndex, 1)
      }
      // 确保在错误时重置状态
    } finally {
      isLoading.value = false
      // 最终确保状态被重置
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
    if (cancelFunction.value) {
      cancelFunction.value()
      cancelFunction.value = null
      isLoading.value = false
    }
  }

  // Eko框架初始化
  const initializeEkoFramework = async () => {
    if (!ekoInstance.value) {
      try {
        console.log('🚀 [Eko] 正在初始化Eko框架...')

        // 创建Eko实例
        const eko = await createDebugTerminalEko({
          agentConfig: {
            name: 'TerminalAssistant',
            description: '终端助手，可以执行命令、管理文件、操作目录等',
            safeMode: true,
            allowedCommands: ['ls', 'pwd', 'cat', 'echo', 'mkdir', 'cd', 'git'],
            blockedCommands: ['rm -rf', 'format', 'shutdown', 'reboot'],
          },
        })

        ekoInstance.value = eko

        console.log('✅ [Eko] Eko框架初始化成功')
        console.log('Agent状态:', eko.getAgent().getStatus())

        // 设置当前Agent ID
        currentAgentId.value = 'terminal-agent'
      } catch (error) {
        console.error('❌ [Eko] Eko框架初始化失败:', error)
      }
    }
  }

  // 切换聊天模式
  const switchChatMode = async (mode: ChatMode) => {
    if (chatMode.value === mode) return

    // 保存当前会话
    saveCurrentSession()

    // 切换模式
    chatMode.value = mode

    // 创建新会话（切换模式时总是开始新对话）
    createNewSession()

    if (mode === 'agent') {
      await initializeEkoFramework()
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

    error,
    sessions,

    // 聊天模式相关状态
    chatMode,
    currentAgentId,

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
    getFirstSession,
    refreshSessions,
    saveCurrentSession,
    sendMessage,
    stopStreaming,
    switchChatMode,
    clearCurrentSession,
    setSidebarWidth,
    initialize,
  }
})
