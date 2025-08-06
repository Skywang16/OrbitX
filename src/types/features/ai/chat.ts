/**
 * AI聊天相关类型定义
 */

// ===== 聊天消息类型 =====
export interface ChatMessage {
  id: string
  messageType: 'user' | 'assistant' | 'system'
  content: string
  timestamp: Date
  metadata?: {
    model?: string
    tokensUsed?: number
    // Agent消息相关字段
    isAgentMessage?: boolean
    messageData?: any
    [key: string]: any
  }
  // 新增：Agent事件相关字段
  agentEventType?: string
  agentEventData?: Record<string, unknown>
  isStreaming?: boolean
}

export interface ChatSession {
  id: string
  title: string
  messages: ChatMessage[]
  createdAt: Date
  updatedAt: Date
  modelId?: string
}

// ===== 聊天状态类型 =====

export type ChatStatus = 'idle' | 'loading' | 'streaming' | 'error'

export interface ChatInputState {
  value: string
  isComposing: boolean
  placeholder: string
  disabled: boolean
}

export interface ChatSessionState {
  id: string | null
  title: string
  messageCount: number
  lastActivity: Date | null
  isActive: boolean
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
