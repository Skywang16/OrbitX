/**
 * Eko回调系统实现
 * 只保留核心功能，移除冗余代码
 */

import type { TerminalCallback, StreamCallbackMessage } from '../types'
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
        if (onMessage) {
          await onMessage(message)
        }
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
