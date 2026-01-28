import { channelApi } from './index'
import type { ExecuteTaskParams, TaskProgressPayload } from '@/api/agent/types'

/**
 * Agent 专用 Channel API
 */
class AgentChannelApi {
  /**
   * 创建 Agent 任务执行流
   */
  createTaskStream = (params: ExecuteTaskParams): ReadableStream<TaskProgressPayload> => {
    // The backend may emit task_* events for subtasks on the same event stream.
    // Only close this stream when the *root* task (the one created by agent_execute_task) ends.
    let rootTaskId: string | null = null
    return channelApi.createStream<TaskProgressPayload>(
      'agent_execute_task',
      { params },
      {
        cancelCommand: 'agent_cancel_task',
        shouldClose: (event: TaskProgressPayload) => {
          if (event.type === 'task_created') {
            rootTaskId = event.taskId
            return false
          }
          if (!rootTaskId) return false
          if (event.type === 'task_completed' || event.type === 'task_cancelled' || event.type === 'task_error') {
            return event.taskId === rootTaskId
          }
          return false
        },
      }
    )
  }

  /**
   * 恢复任务已移除（不再支持）
   */
}

export const agentChannelApi = new AgentChannelApi()
