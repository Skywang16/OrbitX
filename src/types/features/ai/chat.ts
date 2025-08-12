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
 * AI输出的单个步骤/阶段
 */
export interface AIOutputStep {
  type: 'thinking' | 'workflow' | 'text' | 'tool_use' | 'tool_result' | 'error'
  content: string
  timestamp: number
  metadata?: {
    // 思考阶段的元数据
    thinkingDuration?: number

    // 工具调用的元数据
    toolCallId?: string // 工具调用唯一标识
    toolName?: string
    toolCommand?: string
    toolParams?: Record<string, any>
    toolResult?: any
    status?: 'running' | 'completed' | 'error'
    completedAt?: number // 完成时间戳
    originalMessage?: any // 用于调试

    // 工作流的元数据
    workflowName?: string
    agentName?: string
    taskId?: string

    // 错误信息
    errorType?: string
    errorDetails?: string
  }
}

/**
 * 消息信息 - 扩展版本支持完整AI对话数据
 */
export interface Message {
  id: number
  conversationId: number
  role: 'user' | 'assistant' | 'system'
  createdAt: Date

  // AI消息的扩展字段
  steps?: AIOutputStep[] // AI输出的所有步骤
  status?: 'pending' | 'streaming' | 'complete' | 'error' // 消息状态
  duration?: number // 总耗时（毫秒）

  // 兼容字段（用户消息需要，AI消息从steps中获取）
  content?: string // 用户消息内容
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
