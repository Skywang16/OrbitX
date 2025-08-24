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

  // 根据提示内容推断文件类型
  if (prompt.includes('package') || prompt.includes('依赖')) {
    return `${baseDir}package.json`
  }

  if (prompt.includes('config') || prompt.includes('配置')) {
    return `${baseDir}vite.config.ts`
  }

  if (prompt.includes('readme') || prompt.includes('文档')) {
    return `${baseDir}README.md`
  }

  // 默认返回package.json
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
 * 创建回调（带调试信息）
 */
export const createCallback = (): TerminalCallback => {
  return {
    onMessage: async (_message: StreamCallbackMessage, _agentContext?: AgentContext): Promise<void> => {
      // 静默处理消息
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

/**
 * 创建侧边栏专用回调
 * @param onMessage 自定义消息处理函数
 */
export const createSidebarCallback = (onMessage?: (message: StreamMessage) => Promise<void>): TerminalCallback => {
  return {
    onMessage: async (message: StreamCallbackMessage, _agentContext?: AgentContext): Promise<void> => {
      console.log('🔥 [DEBUG] createSidebarCallback收到消息:', message.type, message)
      if (onMessage) {
        // Convert StreamCallbackMessage to StreamMessage for backward compatibility
        const streamMessage: StreamMessage = {
          type:
            message.type === 'agent_start'
              ? 'workflow'
              : (message.type as 'tool_use' | 'tool_result' | 'workflow' | 'text' | 'thinking'),
          toolName: (message as Record<string, unknown>).toolName as string,
          params: (message as Record<string, unknown>).params as Record<string, unknown>,
          toolResult: (message as Record<string, unknown>).toolResult,
          thought: (message as Record<string, unknown>).thought as string,
          text: (message as Record<string, unknown>).text as string,
          streamId: (message as Record<string, unknown>).streamId as string,
          streamDone: (message as Record<string, unknown>).streamDone as boolean,
          workflow: (message as Record<string, unknown>).workflow as { thought?: string },
        }
        await onMessage(streamMessage)
      }
    },
    onHumanConfirm: async (): Promise<boolean> => true,
    onHumanInput: async (): Promise<string> => '',
    onHumanSelect: async (
      _agentContext: AgentContext,
      _prompt: string,
      options: readonly string[]
    ): Promise<string[]> => [options?.[0] ?? ''],
    onHumanHelp: async (): Promise<boolean> => true,
    onCommandConfirm: async (_agentContext: AgentContext, command: string): Promise<boolean> =>
      !isDangerousCommand(command),
    onFileSelect: async (_agentContext: AgentContext, prompt: string, directory?: string): Promise<string> =>
      smartFileSelect(prompt, directory),
    onPathInput: async (_agentContext: AgentContext, __: string, defaultPath?: string): Promise<string> =>
      defaultPath || './',
  }
}
