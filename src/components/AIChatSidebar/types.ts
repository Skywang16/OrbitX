/**
 * AI聊天侧边栏组件相关的类型定义
 */

// 重新导出通用AI类型
export type { AIModelConfig, AIProvider, ChatMessage, ChatSession, StreamCallback, StreamChunk } from '@/types'

// 导入需要在本文件中使用的类型
import type { ChatSession } from '@/types'

// 聊天消息类型扩展
export type MessageType = 'user' | 'assistant' | 'system'

// 聊天状态
export type ChatStatus = 'idle' | 'loading' | 'streaming' | 'error'

// 聊天模式
export type ChatMode = 'chat' | 'agent'

// Agent消息类型
export interface AgentThinkingMessage {
  type: 'thinking'
  stage: string
  content: string
  timestamp: string
}

export interface AgentToolUseMessage {
  type: 'tool_use'
  toolName: string
  params: Record<string, unknown>
  timestamp: string
}

export interface AgentToolResultMessage {
  type: 'tool_result'
  toolName: string
  result: unknown
  timestamp: string
}

export interface AgentTextMessage {
  type: 'text'
  content: string
  timestamp: string
}

export interface AgentWorkflowMessage {
  type: 'workflow'
  stage: string
  content: string
  timestamp: string
  workflow?: Record<string, unknown>
  step?: Record<string, unknown>
}

export type AgentMessageData =
  | AgentThinkingMessage
  | AgentToolUseMessage
  | AgentToolResultMessage
  | AgentTextMessage
  | AgentWorkflowMessage

// 聊天输入状态
export interface ChatInputState {
  value: string
  isComposing: boolean
  placeholder: string
  disabled: boolean
}

// 聊天会话状态
export interface ChatSessionState {
  id: string | null
  title: string
  messageCount: number
  lastActivity: Date | null
  isActive: boolean
}

// 聊天侧边栏配置
export interface ChatSidebarConfig {
  width: number
  minWidth: number
  maxWidth: number
  defaultWidth: number
  resizable: boolean
  collapsible: boolean
}

// 聊天消息渲染选项
export interface MessageRenderOptions {
  showTimestamp: boolean
  showAvatar: boolean
  enableMarkdown: boolean
  enableCodeHighlight: boolean
  enableCopyCode: boolean
  maxContentLength?: number
}

// 聊天会话管理选项
export interface SessionManagerOptions {
  maxSessions: number
  autoSave: boolean
  autoTitle: boolean
  titleMaxLength: number
  storageKey: string
}

// 聊天流式响应配置
export interface StreamingConfig {
  enabled: boolean
  timeout: number
  chunkDelay: number
  showTypingIndicator: boolean
}

// 聊天错误信息
export interface ChatError {
  type: 'network' | 'auth' | 'model' | 'validation' | 'unknown'
  message: string
  code?: string
  details?: Record<string, unknown>
  timestamp: Date
}

// 聊天统计信息
export interface ChatStats {
  totalSessions: number
  totalMessages: number
  totalTokensUsed: number
  averageResponseTime: number
  lastActivity: Date | null
}

// 聊天导出数据
export interface ChatExportData {
  version: string
  sessions: ChatSession[]
  exportedAt: Date
  metadata: {
    totalSessions: number
    totalMessages: number
    dateRange: {
      from: Date
      to: Date
    }
  }
}

// 聊天搜索选项
export interface ChatSearchOptions {
  query: string
  sessionId?: string
  messageType?: MessageType
  dateRange?: {
    from: Date
    to: Date
  }
  caseSensitive: boolean
  useRegex: boolean
}

// 聊天搜索结果
export interface ChatSearchResult {
  sessionId: string
  sessionTitle: string
  messageId: string
  messageContent: string
  messageType: MessageType
  timestamp: Date
  matchedText: string
  context: {
    before: string
    after: string
  }
}

// 聊天快捷操作
export interface ChatQuickAction {
  id: string
  label: string
  icon?: string
  shortcut?: string
  action: () => void | Promise<void>
  visible: boolean
  disabled: boolean
}

// 聊天主题配置
export interface ChatThemeConfig {
  userMessageBg: string
  assistantMessageBg: string
  systemMessageBg: string
  textColor: string
  timestampColor: string
  borderColor: string
  scrollbarColor: string
}

// 聊天可调整大小的配置
export interface ResizableConfig {
  enabled: boolean
  direction: 'horizontal' | 'vertical'
  minSize: number
  maxSize: number
  defaultSize: number
  step: number
  handles: string[]
}
