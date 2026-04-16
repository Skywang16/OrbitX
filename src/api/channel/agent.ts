import type { AgentRunEvent, ExecuteRunParams } from '@/api/agent/types'
import { channelApi } from './index'

/**
 * Agent-specific Channel API
 */
class AgentChannelApi {
  /**
   * Create agent run stream
   */
  createRunStream = (params: ExecuteRunParams): ReadableStream<AgentRunEvent> => {
    // The backend may emit nested sub-agent events on the same stream.
    // Only close this stream when the root agent run ends.
    let rootTaskId: string | null = null
    return channelApi.createStream<AgentRunEvent>(
      'agent_execute_run',
      { params },
      {
        cancelCommand: 'agent_cancel_run',
        shouldClose: (event: AgentRunEvent) => {
          if (event.type === 'agent_run_created') {
            rootTaskId = event.runId
            return false
          }
          if (!rootTaskId) return false
          if (
            event.type === 'agent_run_completed' ||
            event.type === 'agent_run_cancelled' ||
            event.type === 'agent_run_error'
          ) {
            return event.runId === rootTaskId
          }
          return false
        },
      }
    )
  }

  /**
   * Resume run removed (no longer supported)
   */
}

export const agentChannelApi = new AgentChannelApi()
