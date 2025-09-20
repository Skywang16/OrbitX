/**
 * Eko回调系统实现
 * 只保留核心功能，移除冗余代码
 */

import type { TerminalCallback, StreamCallbackMessage } from '../types'
import { taskPersistence } from '@/eko-core/persistence/task_persistence'
import type { AgentContext } from '@/eko-core'

/**
 * 创建回调函数
 * @param onMessage 自定义消息处理函数，如果不提供则只输出基础日志
 */
export const createSidebarCallback = (
  onMessage?: (message: StreamCallbackMessage) => Promise<void>
): TerminalCallback => {
  return {
    onMessage: async (message: StreamCallbackMessage): Promise<void> => {
      // 添加错误处理，避免回调错误中断执行流程
      try {
        // 最小元数据维护：状态变更与 spawn（仅写入任务索引/元数据，不记录 UI 事件流）
        try {
          if (message?.taskId && message.type === 'task_status') {
            await taskPersistence.updateMetadataPartial(message.taskId, {
              status: message.status,
              updatedAt: Date.now(),
            })
          } else if (message?.taskId && message.type === 'task_spawn') {
            await taskPersistence.updateMetadataPartial(message.taskId, {
              parentTaskId: message.parentTaskId,
              rootTaskId: message.rootTaskId,
              name: message.task?.name,
              createdAt: Date.now(),
            })
          }
        } catch (_) {
          // 忽略元数据持久化错误
        }

        if (onMessage) {
          await onMessage(message)
        }

        // UI 事件不再写入新表，结束时无需刷新
      } catch (error) {
        console.error('回调处理错误:', error)
        // 不要抛出错误，避免中断执行流程
      }
    },
    onHumanConfirm: async (_agentContext: AgentContext, _prompt: string): Promise<boolean> => {
      return true
    },
    onHumanInput: async (_agentContext: AgentContext, _prompt: string): Promise<string> => {
      return ''
    },
    onHumanSelect: async (
      _agentContext: AgentContext,
      _prompt: string,
      options: readonly string[]
    ): Promise<string[]> => {
      return [options?.[0] ?? '']
    },
    onHumanHelp: async (_agentContext: AgentContext, _helpType: string, _prompt: string): Promise<boolean> => {
      return true
    },
  }
}
