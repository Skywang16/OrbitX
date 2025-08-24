/**
 * AI聊天功能的状态管理 - 完全重构版本
 *
 * 使用新的会话上下文管理系统，不再向后兼容
 */

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

// 流式消息类型定义（基于Eko源码）
interface StreamMessage {
  type: 'tool_use' | 'tool_result' | 'workflow' | 'text' | 'thinking'
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
}

// 工具函数
const generateSessionTitle = (content: string): string => {
  const title = content.trim().slice(0, 20)
  if (title.length === 0) return '新对话'
  return title.length < content.trim().length ? title + '...' : title
}

// 检测工具执行结果是否包含错误
const isToolResultError = (toolResult: any): boolean => {
  return toolResult?.isError === true
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

  // 防抖保存函数（在store顶层定义，避免重复创建）
  const debouncedSaveSteps = debounce(async (messageId: number, steps: any[]) => {
    try {
      await aiApi.updateMessageSteps(messageId, steps)
    } catch {
      // 静默失败
    }
  }, 100)

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
      // 加载AI设置
      const aiSettingsStore = useAISettingsStore()
      if (!aiSettingsStore.hasModels && !aiSettingsStore.isLoading) {
        await aiSettingsStore.loadSettings()
      }

      // 加载会话列表
      await refreshConversations()
    }
  }

  const setSidebarWidth = (width: number) => {
    sidebarWidth.value = Math.max(300, Math.min(800, width))
  }

  // 辅助函数：查找空会话（messageCount为0的会话）
  const findEmptyConversation = (): Conversation | null => {
    return conversations.value.find(conv => conv.messageCount === 0) || null
  }

  // 会话管理方法
  const createConversation = async (title?: string): Promise<void> => {
    try {
      // 如果有正在进行的对话，先中断
      stopCurrentConversation()

      // 检查是否已经存在空会话
      const existingEmptyConversation = findEmptyConversation()
      if (existingEmptyConversation) {
        // 如果存在空会话，直接切换到该会话
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
    // 如果有正在进行的对话，先中断
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

    let tempAIMessage: Message | null = null

    try {
      isLoading.value = true
      error.value = null

      // 1. 立即保存用户消息（不等待Eko初始化）
      const userMessageId = await aiApi.saveMessage(currentConversationId.value, 'user', content)

      // 2. 立即更新UI显示用户消息（添加到当前消息列表而不是重新加载）
      const userMessage: Message = {
        id: userMessageId,
        conversationId: currentConversationId.value,
        role: 'user',
        content,
        createdAt: new Date(),
      }
      messageList.value.push(userMessage)

      // 3. 确保Eko实例可用
      if (!ekoInstance.value) {
        await initializeEko()
      }

      // 4. 设置模式（基于Eko源码，setMode是同步的且不会失败）
      ekoInstance.value?.setMode(chatMode.value)

      // 5. 获取当前终端的工作目录
      const terminalStore = useTerminalStore()
      const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)
      const currentWorkingDirectory = activeTerminal?.cwd

      // 6. 获取后端构建的完整prompt（包含上下文和环境信息）
      // 传递用户消息ID，确保上下文构建时包含刚保存的用户消息
      const fullPrompt = await aiApi.buildPromptWithContext(
        currentConversationId.value,
        content,
        userMessageId, // 传递用户消息ID作为上下文边界
        currentWorkingDirectory
      )

      // 7. 立即创建AI消息记录到数据库，获取真实ID用于实时保存steps
      const messageId = await aiApi.saveMessage(currentConversationId.value, 'assistant', '正在生成回复...')

      // 创建AI消息对象，使用真实的数据库ID
      tempAIMessage = {
        id: messageId,
        conversationId: currentConversationId.value,
        role: 'assistant',
        createdAt: new Date(),
        steps: [],
        status: 'streaming',
      }

      // 添加消息到列表
      messageList.value.push(tempAIMessage)

      // 8. 设置取消函数
      cancelFunction.value = () => {
        if (ekoInstance.value) {
          ekoInstance.value.abort()
        }
      }

      // 9. 通过eko处理消息（流式输出通过回调处理）
      streamingContent.value = ''
      const response = await ekoInstance.value!.run(fullPrompt)

      // 10. 更新AI回复内容和状态
      if (response.success && response.result) {
        // 更新消息的内容和状态
        tempAIMessage.content = response.result
        tempAIMessage.status = 'complete'
        tempAIMessage.duration = Date.now() - tempAIMessage.createdAt.getTime()

        // 更新消息的最终内容和状态
        if (tempAIMessage.content) {
          await aiApi.updateMessageContent(tempAIMessage.id, tempAIMessage.content)
        }
        await aiApi.updateMessageStatus(tempAIMessage.id, tempAIMessage.status, tempAIMessage.duration)
      } else {
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
        if (tempAIMessage.steps) {
          try {
            await aiApi.updateMessageSteps(tempAIMessage.id, tempAIMessage.steps)
          } catch {
            // 静默失败
          }
        }
      }

      // 11. 刷新会话列表以更新预览（不重新加载消息，保持步骤信息）
      await refreshConversations()
    } catch (err) {
      // 修复UI状态同步问题：确保在异常情况下也更新消息状态
      if (tempAIMessage) {
        tempAIMessage.status = 'error'
        tempAIMessage.duration = Date.now() - tempAIMessage.createdAt.getTime()

        // 添加错误步骤，显示具体错误信息
        tempAIMessage.steps = tempAIMessage.steps || []
        tempAIMessage.steps.push({
          type: 'error',
          content: `AI任务执行失败: ${err instanceof Error ? err.message : '未知错误'}`,
          timestamp: Date.now(),
          metadata: {
            errorType: 'ExecutionError',
            errorDetails: err instanceof Error ? err.stack : String(err),
          },
        })

        // 尝试更新数据库中的消息状态
        try {
          await aiApi.updateMessageStatus(tempAIMessage.id, tempAIMessage.status, tempAIMessage.duration)
          await aiApi.updateMessageSteps(tempAIMessage.id, tempAIMessage.steps)
        } catch {
          // 静默失败，避免二次错误
        }
      }

      error.value = handleErrorWithMessage(err, '发送消息失败')
      throw err
    } finally {
      isLoading.value = false
      cancelFunction.value = null
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
      await aiApi.truncateConversation(currentConversationId.value, truncateAfterMessageId)

      // 2. 发送新消息（复用sendMessage逻辑）
      await sendMessage(newContent)
    } catch (err) {
      error.value = handleErrorWithMessage(err, '截断重问失败')
      throw err
    } finally {
      isLoading.value = false
    }
  }

  // 中断当前正在进行的对话
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

  // 清空错误
  const clearError = (): void => {
    error.value = null
  }

  // 初始化Eko实例（带流式回调）
  const initializeEko = async (): Promise<void> => {
    try {
      if (!ekoInstance.value) {
        // 简化的流式消息处理
        const handleStreamMessage = async (message: StreamMessage) => {
          const tempMessage = messageList.value[messageList.value.length - 1]
          if (!tempMessage || tempMessage.role !== 'assistant') return

          // 确保steps数组存在
          tempMessage.steps = tempMessage.steps || []

          // 统一的步骤更新函数
          const updateOrCreateStep = (stepData: { type: string; content: string; streamId?: string }) => {
            let targetStep: any = null

            if (stepData.type === 'thinking') {
              // thinking类型：如果有streamId就精确匹配，否则查找最后一个thinking步骤
              if (stepData.streamId) {
                targetStep = tempMessage.steps?.find(
                  step => step.type === 'thinking' && step.metadata?.streamId === stepData.streamId
                )
              } else {
                // 使用兼容的方式查找最后一个thinking步骤
                const thinkingSteps = tempMessage.steps?.filter(step => step.type === 'thinking') || []
                targetStep = thinkingSteps[thinkingSteps.length - 1] || null
              }
            } else {
              // 其他类型：必须有streamId才能匹配
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
                // 检查工具执行结果是否包含错误
                const hasError = isToolResultError(message.toolResult)
                toolStep.toolExecution.status = hasError ? 'error' : 'completed'
                toolStep.toolExecution.endTime = Date.now()
                toolStep.toolExecution.result = message.toolResult

                // 如果有错误，记录错误信息
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

              streamingContent.value = message.text || ''
              // 注意：不在这里设置 status 为 'complete'
              // 状态更新由 sendMessage 方法的最终处理来完成
              if (message.streamDone) {
                tempMessage.content = message.text
              }
              break

            case 'error':
              // 处理错误消息，立即更新UI状态
              console.error('🚨 Eko错误:', (message as any).error)

              // 立即更新UI中的消息状态
              tempMessage.status = 'error'
              tempMessage.duration = Date.now() - tempMessage.createdAt.getTime()

              // 添加错误步骤到UI（数据库保存由其他地方处理）
              tempMessage.steps = tempMessage.steps || []
              tempMessage.steps.push({
                type: 'error',
                content: `AI任务执行失败: ${(message as any).error}`,
                timestamp: Date.now(),
                metadata: {
                  errorType: 'LLMError',
                  errorDetails: String((message as any).error),
                },
              })

              // 清除流式内容
              streamingContent.value = ''
              break
          }

          // 直接保存，去掉队列
          debouncedSaveSteps(tempMessage.id, tempMessage.steps)
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
      ekoInstance.value = await createTerminalEko({ debug: true })
    }
  }

  // 从会话状态恢复 AI 状态
  const restoreFromSessionState = (): void => {
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

    // 等待会话Store初始化
    await sessionStore.initialize()

    // 从会话状态恢复
    restoreFromSessionState()

    // 如果恢复了当前会话ID，尝试加载会话
    if (currentConversationId.value) {
      try {
        await switchToConversation(currentConversationId.value)
      } catch {
        currentConversationId.value = null
      }
    }

    // 加载会话列表
    await refreshConversations()

    isInitialized.value = true
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
    stopCurrentConversation,
    clearError,
    initializeEko,
    initialize,
    restoreFromSessionState,
    saveToSessionState,
  }
})
