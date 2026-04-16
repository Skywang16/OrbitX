import type { SubagentRecord } from './subagent'

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

export interface RetryStatus {
  attempt: number
  maxAttempts: number
  reason: string
  errorMessage: string
}

export interface Message {
  id: number
  threadId: number
  role: MessageRole
  agentType: string
  parentMessageId?: number
  status: MessageStatus
  blocks: Block[]
  isSummary: boolean
  isInternal: boolean
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
  | { type: 'error'; code: string; message: string; details?: string }

export interface ToolOutput {
  content: unknown
  title?: string
  metadata?: unknown
  cancelReason?: string
}

export type AgentRunEvent =
  | { type: 'agent_run_created'; runId: string; threadId: number; workspacePath: string }
  | { type: 'message_created'; runId: string; message: Message }
  | { type: 'subagent_created'; runId: string; subagent: SubagentRecord }
  | { type: 'subagent_updated'; runId: string; subagent: SubagentRecord }
  | { type: 'block_appended'; runId: string; messageId: number; block: Block }
  | { type: 'block_updated'; runId: string; messageId: number; blockId: string; block: Block }
  | {
      type: 'tool_confirmation_requested'
      runId: string
      requestId: string
      workspacePath: string
      toolName: string
      summary: string
    }
  | {
      type: 'message_finished'
      runId: string
      messageId: number
      status: MessageStatus
      finishedAt: string
      durationMs: number
      tokenUsage?: TokenUsage
      contextUsage?: ContextUsage
    }
  | { type: 'agent_run_completed'; runId: string }
  | { type: 'agent_run_error'; runId: string; error: { code: string; message: string; details?: string } }
  | { type: 'agent_run_cancelled'; runId: string }
  | {
      type: 'agent_run_retrying'
      runId: string
      attempt: number
      maxAttempts: number
      reason: string
      errorMessage: string
      retryInMs: number
    }
