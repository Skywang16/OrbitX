/**
 * Agent API type definitions
 *
 * Defines all interface types for the agent runtime.
 */

import type { AgentRunEvent as DomainAgentRunEvent } from '@/types'

/**
 * Agent run execution parameters
 */
export interface ExecuteRunParams {
  /** Workspace path */
  workspacePath: string
  /** Thread ID */
  threadId: number
  /** User prompt */
  userPrompt: string
  /** Model ID - required! */
  modelId: string
  /** Single request override agent type (will not persist to session) */
  agentType?: string
  /** Command ID for slash commands (e.g., "code-review", "skill-creator") */
  commandId?: string
  /** Image attachments (optional) */
  images?: Array<{ type: 'image'; dataUrl: string; mimeType: string }>
}

/**
 * Agent run summary information
 */
export interface AgentRunSummary {
  /** Task ID */
  runId: string
  /** Thread ID */
  threadId: number
  /** Task status */
  status: AgentRunStatus
  /** Current iteration count */
  currentIteration: number
  /** Error count */
  errorCount: number
  /** Creation time */
  createdAt: string
  /** Update time */
  updatedAt: string
  /** User prompt */
  userPrompt?: string
  /** Completion time */
  completedAt?: string
}

/**
 * Task status
 */
export type AgentRunStatus =
  | 'created' // Created
  | 'running' // Running
  | 'paused' // Paused
  | 'completed' // Completed
  | 'error' // Error
  | 'cancelled' // Cancelled

/**
 * Agent run event payload
 */
export type AgentRunEvent = DomainAgentRunEvent

// ===== Streaming interface types =====

/**
 * Task progress stream interface
 *
 * Provides chainable event listening API
 */
export interface AgentRunStream {
  /**
   * Listen to progress events
   * @param callback Progress callback function
   * @returns Stream object (supports chaining)
   */
  onProgress(callback: (event: AgentRunEvent) => void): AgentRunStream

  /**
   * Listen to error events
   * @param callback Error callback function
   * @returns Stream object (supports chaining)
   */
  onError(callback: (error: Error) => void): AgentRunStream

  /**
   * Listen to stream close event
   * @param callback Close callback function
   * @returns Stream object (supports chaining)
   */
  onClose(callback: () => void): AgentRunStream

  /**
   * Manually close stream
   */
  close(): void

  /**
   * Whether stream is closed
   */
  readonly isClosed: boolean
}

// ===== Control command types =====

/**
 * Task control command
 */
export type RunControlCommand = CancelCommand

/**
 * Cancel command
 */
export interface CancelCommand {
  type: 'cancel'
  reason?: string
}

// ===== Query filter types =====

/**
 * Agent run list filter conditions
 */
export interface RunListFilter {
  /** Thread ID filter */
  threadId?: number
  /** Status filter */
  status?: AgentRunStatus | string
  /** Pagination offset */
  offset?: number
  /** Pagination limit */
  limit?: number
}

// ===== Command system types =====

export interface CommandSummary {
  name: string
  description?: string
  agent?: string
  model?: string
  delegated: boolean
}

export interface CommandRenderResult {
  name: string
  agent?: string
  model?: string
  delegated: boolean
  prompt: string
}

// ===== Skill system types =====

export type SkillSource = 'global' | 'workspace'

export interface SkillSummary {
  name: string
  description: string
  license?: string
  metadata: Record<string, string>
  /** Skill source: 'global' | 'workspace' */
  source: SkillSource
  /** Skill directory path */
  skillDir: string
}

export interface SkillValidationResult {
  valid: boolean
  errors: string[]
  warnings: string[]
}

// ===== Utility types =====

/**
 * Event type guard function
 */
export const isAgentRunEvent = (event: unknown): event is AgentRunEvent => {
  if (!event || typeof event !== 'object') {
    return false
  }

  const candidate = event as { type?: unknown }
  return typeof candidate.type === 'string'
}

/**
 * Determine if it is a terminal event
 */
export const isTerminalEvent = (event: AgentRunEvent): boolean => {
  return (
    event.type === 'agent_run_completed' || event.type === 'agent_run_cancelled' || event.type === 'agent_run_error'
  )
}

/**
 * Get run ID of event
 */
export const getEventTaskId = (event: AgentRunEvent): string => {
  return 'runId' in event && typeof event.runId === 'string' ? event.runId : ''
}

/**
 * Determine if it is an error event
 */
export const isErrorEvent = (event: AgentRunEvent): boolean => {
  return event.type === 'agent_run_error'
}

/**
 * File context status
 */
export interface FileContextStatus {
  workspacePath: string
  fileCount: number
  files: string[]
}
