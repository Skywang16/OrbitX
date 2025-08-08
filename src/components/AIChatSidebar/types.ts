/**
 * AI聊天侧边栏组件相关的类型定义 - 完全重构版本
 *
 * 使用新的会话管理系统类型，不再向后兼容
 */

// 重新导出新的AI类型
export type { AIModelConfig, AIProvider, StreamCallback, StreamChunk } from '@/types'
export type { Conversation, Message } from '@/types/features/ai/chat'

// 聊天消息类型扩展
export type MessageType = 'user' | 'assistant' | 'system'

// 聊天状态
export type ChatStatus = 'idle' | 'loading' | 'streaming' | 'error'

// 聊天模式
export type ChatMode = 'chat' | 'agent'

// Agent消息类型
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

export type AgentMessageData = AgentTextMessage | AgentWorkflowMessage
