/**
 * Agent API - Frontend interface wrapper for backend Agent system
 *
 * Provides agent run execution, state management, and real-time progress monitoring.
 */

import { agentChannelApi } from '@/api/channel/agent'
import { invoke } from '@/utils/request'
import type {
  AgentRunEvent,
  AgentRunStream,
  AgentRunSummary,
  CommandRenderResult,
  CommandSummary,
  ExecuteRunParams,
  RunListFilter,
  SkillSummary,
  SkillValidationResult,
} from './types'

/**
 * Agent API main class
 *
 * Wraps backend agent runtime commands with a type-safe frontend interface.
 */
export class AgentApi {
  /**
   * Execute an agent run
   * @param userPrompt User input
   * @param threadId Thread ID
   * @param modelId Model ID
   * @param images Image attachments (optional)
   * @returns Returns agent run stream
   */
  executeRun = async (params: ExecuteRunParams): Promise<AgentRunStream> => {
    const stream = agentChannelApi.createRunStream(params)
    return this.createProgressStreamFromReadableStream(stream)
  }

  /**
   * Cancel agent run
   * @param runId Task ID
   * @param reason Cancellation reason
   */
  cancelRun = async (runId: string, reason?: string): Promise<void> => {
    await invoke('agent_cancel_run', { runId, reason })
  }

  confirmTool = async (requestId: string, decision: 'allow_once' | 'allow_always' | 'deny'): Promise<void> => {
    await invoke('agent_tool_confirm', {
      params: { requestId, decision },
    })
  }

  /**
   * List active agent runs
   * @param filters Filter conditions
   * @returns Agent run summary list
   */
  listRuns = async (filters?: RunListFilter): Promise<AgentRunSummary[]> => {
    return await invoke<AgentRunSummary[]>('agent_list_runs', {
      threadId: filters?.threadId,
      statusFilter: filters?.status,
    })
  }

  listCommands = async (workspacePath: string): Promise<CommandSummary[]> => {
    return await invoke<CommandSummary[]>('agent_list_commands', {
      params: { workspacePath },
    })
  }

  renderCommand = async (workspacePath: string, name: string, input: string): Promise<CommandRenderResult> => {
    return await invoke<CommandRenderResult>('agent_render_command', {
      params: { workspacePath, name, input },
    })
  }

  listSkills = async (workspacePath: string): Promise<SkillSummary[]> => {
    return await invoke<SkillSummary[]>('agent_list_skills', {
      params: { workspacePath },
    })
  }

  validateSkill = async (skillPath: string): Promise<SkillValidationResult> => {
    return await invoke<SkillValidationResult>('agent_validate_skill', {
      skillPath,
    })
  }

  /**
   * Get run details
   * @param runId Task ID
   * @returns Agent run detailed information
   */
  getRun = async (runId: string): Promise<AgentRunSummary> => {
    const tasks = await this.listRuns()
    const task = tasks.find(t => t.runId === runId)

    if (!task) {
      throw new Error(`Agent run ${runId} not found`)
    }

    return task
  }

  sendCommand = async (runId: string, command: { type: 'cancel'; reason?: string }): Promise<void> => {
    await this.cancelRun(runId, command.reason)
  }

  /**
   * Create agent run stream from ReadableStream
   * @private
   * @param stream ReadableStream
   * @returns AgentRunStream
   */
  private createProgressStreamFromReadableStream(stream: ReadableStream<AgentRunEvent>): AgentRunStream {
    let isClosed = false
    const callbacks: Array<(event: AgentRunEvent) => void> = []
    const errorCallbacks: Array<(error: Error) => void> = []
    const closeCallbacks: Array<() => void> = []
    let reader: ReadableStreamDefaultReader<AgentRunEvent> | null = null
    // Used to temporarily store events when there are no subscribers yet, to avoid losing early root-run events.
    const pendingEvents: AgentRunEvent[] = []

    const startReading = async () => {
      try {
        reader = stream.getReader()

        while (!isClosed) {
          const { done, value } = await reader.read()

          if (done || isClosed) {
            closeStream()
            break
          }

          // Print Channel output content (using warn to comply with no-console rule)
          console.warn('[Channel output]', {
            type: value.type,
            data: value,
            timestamp: new Date().toISOString(),
          })

          if (callbacks.length === 0) {
            // No subscribers yet, temporarily store event
            pendingEvents.push(value)
          } else {
            // Notify all listeners
            callbacks.forEach(callback => {
              try {
                callback(value)
              } catch (error) {
                console.error('[AgentApi] Progress callback error:', error)
              }
            })
          }
        }
      } catch (error) {
        if (!isClosed) {
          errorCallbacks.forEach(callback => {
            try {
              callback(error as Error)
            } catch (err) {
              console.error('[AgentApi] Error callback error:', err)
            }
          })
          closeStream()
        }
      }
    }

    const closeStream = () => {
      if (isClosed) return
      isClosed = true

      if (reader) {
        reader.cancel().catch(console.error)
        reader = null
      }

      closeCallbacks.forEach(callback => {
        try {
          callback()
        } catch (error) {
          console.error('[AgentApi] Close callback error:', error)
        }
      })

      // Clear callback arrays
      callbacks.length = 0
      errorCallbacks.length = 0
      closeCallbacks.length = 0
    }

    startReading()

    // Create stream object
    const taskProgressStream: AgentRunStream = {
      onProgress: callback => {
        if (!isClosed) {
          callbacks.push(callback)
          // When first subscriber appears, immediately replay pendingEvents to current subscriber
          if (pendingEvents.length > 0) {
            try {
              for (const ev of pendingEvents.splice(0, pendingEvents.length)) {
                callback(ev)
              }
            } catch (error) {
              console.error('[AgentApi] Replay stored event error:', error)
            }
          }
        }
        return taskProgressStream
      },

      onError: callback => {
        if (!isClosed) {
          errorCallbacks.push(callback)
        }
        return taskProgressStream
      },

      onClose: callback => {
        if (!isClosed) {
          closeCallbacks.push(callback)
        } else {
          callback()
        }
        return taskProgressStream
      },

      close: () => {
        closeStream()
      },

      get isClosed() {
        return isClosed
      },
    }

    return taskProgressStream
  }
}

/**
 * Agent API singleton instance
 */
export const agentApi = new AgentApi()

/**
 * Export types
 */
export * from './types'

/**
 * Frontend extension types
 */
export interface AgentRunState extends AgentRunSummary {
  /** Whether listening to progress */
  isListening?: boolean
  /** Last update time */
  lastUpdated?: Date
  /** Progress stream reference */
  progressStream?: AgentRunStream
  /** Recent progress events */
  recentEvents?: AgentRunEvent[]
  /** Error information */
  error?: string
}

/**
 * Default export
 */
export default agentApi
