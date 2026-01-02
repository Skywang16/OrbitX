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
    return channelApi.createStream<TaskProgressPayload>(
      'agent_execute_task',
      { params },
      {
        cancelCommand: 'agent_cancel_task',
        shouldClose: (event: TaskProgressPayload) => {
          return event.type === 'task_completed' || event.type === 'task_cancelled' || event.type === 'task_error'
        },
      }
    )
  }

  /**
   * 恢复任务已移除（不再支持）
   */
}

export const agentChannelApi = new AgentChannelApi()
