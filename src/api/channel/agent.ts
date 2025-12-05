import { channelApi } from './index'
import type { TaskProgressPayload } from '@/api/agent/types'

/**
 * Agent 专用 Channel API
 */
class AgentChannelApi {
  /**
   * 创建 Agent 任务执行流
   */
  createTaskStream = (params: {
    conversationId: number
    userPrompt: string
    chatMode: 'chat' | 'agent'
    modelId: string
    images?: Array<{ type: 'image'; dataUrl: string; mimeType: string }>
    configOverrides?: Record<string, unknown>
    restoreTaskId?: string
  }): ReadableStream<TaskProgressPayload> => {
    return channelApi.createStream<TaskProgressPayload>(
      'agent_execute_task',
      { params },
      {
        cancelCommand: 'agent_cancel_task',
        shouldClose: (event: TaskProgressPayload) => {
          return (
            event.type === 'TaskCompleted' ||
            event.type === 'TaskCancelled' ||
            (event.type === 'TaskError' && !event.payload.isRecoverable)
          )
        },
      }
    )
  }

  /**
   * 创建 Agent 任务恢复流
   */
  createResumeStream = (taskId: string): ReadableStream<TaskProgressPayload> => {
    return channelApi.createStream<TaskProgressPayload>(
      'agent_resume_task',
      { task_id: taskId },
      {
        cancelCommand: 'agent_cancel_task',
        shouldClose: (event: TaskProgressPayload) => {
          return (
            event.type === 'TaskCompleted' ||
            event.type === 'TaskCancelled' ||
            (event.type === 'TaskError' && !event.payload.isRecoverable)
          )
        },
      }
    )
  }
}

export const agentChannelApi = new AgentChannelApi()
