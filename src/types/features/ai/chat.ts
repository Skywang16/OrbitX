/**
 * AI聊天相关类型定义 - 全新重构版本
 *
 * 基于会话上下文管理系统的类型定义
 */

// ===== 会话管理类型 =====

/**
 * 会话信息
 */
export interface Conversation {
  id: number
  title: string
  messageCount: number
  lastMessagePreview?: string
  createdAt: Date
  updatedAt: Date
}

/**
 * 消息信息
 */
export interface Message {
  id: number
  conversationId: number
  role: 'user' | 'assistant' | 'system'
  content: string
  createdAt: Date
}

/**
 * AI配置
 */
export interface AIConfig {
  maxContextTokens: number // 上下文最大token
  modelName: string // 使用的模型名称
  enableSemanticCompression: boolean // 是否启用语义压缩
}

/**
 * 上下文统计信息
 */
export interface ContextStats {
  conversationId: number
  totalMessages: number
  summaryGenerated: boolean
  lastSummaryAt?: Date
}

// ===== 聊天状态类型 =====

export type ChatStatus = 'idle' | 'loading' | 'streaming' | 'error'

export interface ChatInputState {
  value: string
  isComposing: boolean
  placeholder: string
  disabled: boolean
}

/**
 * 会话状态（重构版本）
 */
export interface ConversationState {
  currentConversationId: number | null
  conversations: Conversation[]
  messages: Message[]
  isLoading: boolean
  error: string | null
}

/**
 * 消息发送请求
 */
export interface SendMessageRequest {
  conversationId: number
  content: string
  modelId?: string
}

/**
 * 截断重问请求
 */
export interface TruncateAndResendRequest {
  conversationId: number
  truncateAfterMessageId: number
  newContent: string
  modelId?: string
}

// ===== 聊天配置类型 =====

export interface ChatSidebarConfig {
  width: number
  minWidth: number
  maxWidth: number
  defaultWidth: number
  resizable: boolean
  collapsible: boolean
}
