/**
 * Eko回调系统实现
 * 只保留核心功能，移除冗余代码
 */

import type { TerminalCallback, StreamCallbackMessage } from '../types'
import type { AgentContext } from '@/eko-core'

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
