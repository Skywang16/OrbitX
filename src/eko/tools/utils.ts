/**
 * 终端工具共用函数
 */

import type { AgentContext } from '@eko-ai/eko'
import { terminalAPI } from '@/api/terminal'

/**
 * 获取Agent名称的辅助函数
 */
export const getAgentName = (_context: AgentContext): string => {
  // 简单返回默认名称，避免类型复杂性
  return 'AI Agent'
}

/**
 * 获取或创建Agent专属终端
 * 使用终端store来管理Agent专属终端标签页
 */
export const getOrCreateAgentTerminal = async (_agentName: string = 'AI Agent'): Promise<number> => {
  try {
    // 检查是否已存在Agent专属终端
    const terminals = await terminalAPI.listTerminals()
    const existingTerminal = terminals.find(_id => {
      // 这里需要根据实际API来检查终端标题
      // 暂时返回第一个终端作为Agent专属终端
      return true
    })

    if (existingTerminal) {
      return existingTerminal
    }

    // 创建新的Agent专属终端
    await terminalAPI.createTerminal({
      rows: 24,
      cols: 80,
      cwd: process.cwd(),
    })

    // 如果没有找到，回退到第一个可用终端
    const terminals2 = await terminalAPI.listTerminals()
    return terminals2.length > 0 ? terminals2[0] : 0
  } catch (error) {
    // 如果创建失败，返回第一个可用终端
    const terminals = await terminalAPI.listTerminals()
    return terminals.length > 0 ? terminals[0] : 0
  }
}

/**
 * 等待指定时间
 */
export const sleep = (ms: number): Promise<void> => {
  return new Promise(resolve => setTimeout(resolve, ms))
}

/**
 * 转义shell命令中的特殊字符
 */
export const escapeShellArg = (arg: string): string => {
  return arg.replace(/["`$\\]/g, '\\$&')
}

/**
 * 检查路径是否为危险路径
 */
export const isDangerousPath = (path: string): boolean => {
  const dangerousPaths = ['/', '/usr', '/bin', '/etc', '/var', '/home', '/root', '.', '..']
  const normalizedPath = path.replace(/\/+$/, '') // 移除尾部斜杠
  return dangerousPaths.includes(normalizedPath) || normalizedPath === ''
}
