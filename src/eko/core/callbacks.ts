/**
 * Eko回调系统实现 - 简化版本
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
    onMessage: async message => {
      console.log('[Eko Message]', message)
    },
    onHumanConfirm: async (_, prompt) => {
      console.log('[确认]', prompt, '-> 自动确认')
      return true
    },
    onHumanInput: async (_, prompt) => {
      console.log('[输入]', prompt, '-> 使用默认值')
      return ''
    },
    onHumanSelect: async (_, prompt, options) => {
      console.log('[选择]', prompt, '选项:', options, '-> 选择第一个')
      return [options?.[0] || '']
    },
    onHumanHelp: async (_, helpType, prompt) => {
      console.log('[帮助]', helpType, prompt, '-> 自动处理')
      return true
    },
    onCommandConfirm: async (_, command) => {
      const safe = !isDangerousCommand(command)
      console.log('[命令确认]', command, '->', safe ? '允许' : '拒绝')
      return safe
    },
    onFileSelect: async (_, prompt, directory) => {
      const file = smartFileSelect(prompt, directory)
      console.log('[文件选择]', prompt, '目录:', directory, '-> 选择:', file)
      return file
    },
    onPathInput: async (_, prompt, defaultPath) => {
      const path = defaultPath || './default-path'
      console.log('[路径输入]', prompt, '-> 使用:', path)
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
    onMessage: onMessage || (async () => {}),
    onHumanConfirm: async () => true,
    onHumanInput: async () => '',
    onHumanSelect: async (_, __, options) => [options?.[0] || ''],
    onHumanHelp: async () => true,
    onCommandConfirm: async (_, command) => !isDangerousCommand(command),
    onFileSelect: async (_, prompt, directory) => smartFileSelect(prompt, directory),
    onPathInput: async (_, __, defaultPath) => defaultPath || './',
  }
}
