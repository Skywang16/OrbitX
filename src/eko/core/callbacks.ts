/**
 * Eko回调系统实现
 * 只保留核心功能，移除冗余代码
 */

import type { TerminalCallback, StreamMessage, StreamCallbackMessage } from '../types'
import type { AgentContext } from '@eko-ai/eko'

/**
 * 智能文件选择 - 根据提示内容推断合适的文件
 */
const smartFileSelect = (prompt: string, directory?: string): string => {
  const baseDir = directory || './'

  // Infer file type based on prompt content
  if (prompt.includes('package') || prompt.includes('dependency') || prompt.includes('依赖')) {
    return `${baseDir}package.json`
  }

  if (prompt.includes('config') || prompt.includes('configuration') || prompt.includes('配置')) {
    return `${baseDir}vite.config.ts`
  }

  if (prompt.includes('readme') || prompt.includes('documentation') || prompt.includes('文档')) {
    return `${baseDir}README.md`
  }

  // Default to package.json
  return `${baseDir}package.json`
}

/**
 * 危险命令检测
 */
const isDangerousCommand = (command: string): boolean => {
  const dangerousCommands = ['rm -rf', 'sudo rm', 'format', 'del /f', 'shutdown', 'reboot']
  return dangerousCommands.some(dangerous => command.toLowerCase().includes(dangerous))
}

/**
 * 创建回调函数
 * @param onMessage 自定义消息处理函数，如果不提供则只输出基础日志
 */
export const createSidebarCallback = (onMessage?: (message: StreamMessage) => Promise<void>): TerminalCallback => {
  return {
    onMessage: async (message: StreamCallbackMessage, _agentContext?: AgentContext): Promise<void> => {
      if (onMessage) {
        // Convert StreamCallbackMessage to StreamMessage for backward compatibility
        const streamMessage: StreamMessage = {
          type: message.type as StreamMessage['type'],
          toolName: (message as Record<string, unknown>).toolName as string,
          params: (message as Record<string, unknown>).params as Record<string, unknown>,
          toolResult: (message as Record<string, unknown>).toolResult,
          thought: (message as Record<string, unknown>).thought as string,
          text: (message as Record<string, unknown>).text as string,
          streamId: (message as Record<string, unknown>).streamId as string,
          streamDone: (message as Record<string, unknown>).streamDone as boolean,
          workflow: (message as Record<string, unknown>).workflow as { thought?: string },
          // 新增字段支持
          agentName: (message as Record<string, unknown>).agentName as string,
          agentResult: (message as Record<string, unknown>).agentResult,
          toolStreaming: (message as Record<string, unknown>).toolStreaming as StreamMessage['toolStreaming'],
          fileData: (message as Record<string, unknown>).fileData as StreamMessage['fileData'],
          error: (message as Record<string, unknown>).error as StreamMessage['error'],
          finish: (message as Record<string, unknown>).finish as StreamMessage['finish'],
        }

        await onMessage(streamMessage)
      }

      // 基础日志输出（无论是否有自定义处理函数都会输出）
      switch (message.type) {
        case 'agent_start':
          console.warn('🚀 [EKO] Agent开始执行:', message)
          break
        case 'agent_result':
          console.warn('✅ [EKO] Agent执行结果:', message)
          break
        case 'tool_streaming':
          console.warn('📡 [EKO] 工具参数流式输出:', message)
          break
        case 'tool_running':
          console.warn('⚙️ [EKO] 工具执行中:', message)
          break
        case 'file':
          console.warn('📁 [EKO] 文件输出:', message)
          break
        case 'error':
          console.warn('❌ [EKO] 错误信息:', message)
          break
        case 'finish':
          console.warn('🏁 [EKO] 完成信息:', message)
          break
        default:
          // 对于其他回调类型，保持静默
          break
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
    onCommandConfirm: async (_agentContext: AgentContext, command: string): Promise<boolean> => {
      const safe = !isDangerousCommand(command)
      return safe
    },
    onFileSelect: async (_agentContext: AgentContext, prompt: string, directory?: string): Promise<string> => {
      const file = smartFileSelect(prompt, directory)
      return file
    },
    onPathInput: async (_agentContext: AgentContext, _prompt: string, defaultPath?: string): Promise<string> => {
      const path = defaultPath || './default-path'
      return path
    },
  }
}
