export type MessageRole = 'user' | 'assistant'
export type MessageStatus = 'streaming' | 'completed' | 'cancelled' | 'error'
export type ToolStatus = 'pending' | 'running' | 'completed' | 'cancelled' | 'error'

export interface TokenUsage {
  inputTokens: number
  outputTokens: number
  cacheReadTokens?: number
  cacheWriteTokens?: number
}

export interface ContextUsage {
  tokensUsed: number
  contextWindow: number
}

export interface Message {
  id: number
  sessionId: number
  role: MessageRole
  agentType: string
  parentMessageId?: number
  status: MessageStatus
  blocks: Block[]
  isSummary: boolean
  modelId?: string
  providerId?: string
  createdAt: string
  finishedAt?: string
  durationMs?: number
  tokenUsage?: TokenUsage
  contextUsage?: ContextUsage
}

export type Block =
  | { type: 'user_text'; content: string }
  | { type: 'user_image'; dataUrl: string; mimeType: string; fileName?: string; fileSize?: number }
  | { type: 'thinking'; id: string; content: string; isStreaming: boolean }
  | { type: 'text'; id: string; content: string; isStreaming: boolean }
  | {
      type: 'tool'
      id: string
      callId: string
      name: string
      status: ToolStatus
      input: unknown
      output?: ToolOutput
      compactedAt?: string
      startedAt: string
      finishedAt?: string
      durationMs?: number
    }
  | { type: 'agent_switch'; fromAgent: string; toAgent: string; reason?: string }
  | {
      type: 'subtask'
      childSessionId: number
      agentType: string
      description: string
      status: 'pending' | 'running' | 'completed' | 'error'
      summary?: string
    }
  | { type: 'error'; code: string; message: string; details?: string }

export interface ToolOutput {
  content: unknown
  title?: string
  metadata?: unknown
  cancelReason?: string
}

export type TaskEvent =
  | { type: 'task_created'; taskId: string; sessionId: number; workspacePath: string }
  | { type: 'message_created'; message: Message }
  | { type: 'block_appended'; messageId: number; block: Block }
  | { type: 'block_updated'; messageId: number; blockId: string; block: Block }
  | {
      type: 'tool_confirmation_requested'
      taskId: string
      requestId: string
      workspacePath: string
      toolName: string
      summary: string
    }
  | {
      type: 'message_finished'
      messageId: number
      status: MessageStatus
      finishedAt: string
      durationMs: number
      tokenUsage?: TokenUsage
      contextUsage?: ContextUsage
    }
  | { type: 'task_completed'; taskId: string }
  | { type: 'task_error'; taskId: string; error: { code: string; message: string; details?: string } }
  | { type: 'task_cancelled'; taskId: string }
