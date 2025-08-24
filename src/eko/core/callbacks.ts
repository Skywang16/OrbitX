/**
 * Eko回调系统实现
 * 只保留核心功能，移除冗余代码
 */

import type { TerminalCallback } from '../types'

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
    onMessage: async _message => {
      // 静默处理消息
    },
    onHumanConfirm: async (_, _prompt) => {
      return true
    },
    onHumanInput: async (_, _prompt) => {
      return ''
    },
    onHumanSelect: async (_, _prompt, options) => {
      return [options?.[0] || '']
    },
    onHumanHelp: async (_, _helpType, _prompt) => {
      return true
    },
    onCommandConfirm: async (_, command) => {
      const safe = !isDangerousCommand(command)
      return safe
    },
    onFileSelect: async (_, prompt, directory) => {
      const file = smartFileSelect(prompt, directory)
      return file
    },
    onPathInput: async (_, _prompt, defaultPath) => {
      const path = defaultPath || './default-path'
      return path
    },
  }
}

/**
 * 创建侧边栏专用回调
 * @param onMessage 自定义消息处理函数
 */
export const createSidebarCallback = (onMessage?: (message: any) => Promise<void>): TerminalCallback => {
  return {
    onMessage: async (message: any) => {
      console.log('🔥 [DEBUG] createSidebarCallback收到消息:', message.type, message)
      if (onMessage) {
        await onMessage(message)
      }
    },
    onHumanConfirm: async () => true,
    onHumanInput: async () => '',
    onHumanSelect: async (_, __, options) => [options?.[0] || ''],
    onHumanHelp: async () => true,
    onCommandConfirm: async (_, command) => !isDangerousCommand(command),
    onFileSelect: async (_, prompt, directory) => smartFileSelect(prompt, directory),
    onPathInput: async (_, __, defaultPath) => defaultPath || './',
  }
}
