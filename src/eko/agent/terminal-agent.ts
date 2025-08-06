/**
 * 终端专用Agent
 * 为终端模拟器提供专门的AI代理功能
 */

import { Agent } from '@eko-ai/eko'
import type { TerminalAgentConfig } from '../types'
import { terminalTools } from '../tools/terminal-tools'

/**
 * 终端Agent类
 * 继承自Eko的Agent基类，专门为终端操作优化
 */
export class TerminalAgent extends Agent {
  private config: TerminalAgentConfig

  constructor(config: Partial<TerminalAgentConfig> = {}) {
    // 默认配置
    const defaultConfig: TerminalAgentConfig = {
      name: 'Terminal',
      description: '终端操作专家，能够执行命令、管理文件、操作目录等终端相关任务',
      defaultTerminalId: undefined,
      defaultWorkingDirectory: undefined,
      safeMode: true,
      allowedCommands: [],
      blockedCommands: [
        'rm -rf /',
        'sudo rm -rf',
        'format',
        'del /f /s /q',
        'shutdown',
        'reboot',
        'halt',
        'poweroff',
        'init 0',
        'init 6',
      ],
    }

    // 合并配置
    const finalConfig = { ...defaultConfig, ...config }

    // 调用父类构造函数
    super({
      name: finalConfig.name,
      description: finalConfig.description,
      tools: terminalTools,
      llms: ['default'], // 使用默认模型
    })

    this.config = finalConfig
  }

  /**
   * 获取Agent配置
   */
  getConfig(): TerminalAgentConfig {
    return { ...this.config }
  }

  /**
   * 更新Agent配置
   */
  updateConfig(updates: Partial<TerminalAgentConfig>): void {
    this.config = { ...this.config, ...updates }

    // 更新描述
    if (updates.description) {
      this.description = updates.description
    }
  }

  /**
   * 检查命令是否安全
   */
  isCommandSafe(command: string): boolean {
    if (!this.config.safeMode) {
      return true
    }

    const lowerCommand = command.toLowerCase().trim()

    // 检查黑名单
    for (const blocked of this.config.blockedCommands || []) {
      if (lowerCommand.includes(blocked.toLowerCase())) {
        return false
      }
    }

    // 如果有白名单，检查是否在白名单中
    if (this.config.allowedCommands && this.config.allowedCommands.length > 0) {
      return this.config.allowedCommands.some(allowed => lowerCommand.startsWith(allowed.toLowerCase()))
    }

    return true
  }

  /**
   * 设置默认终端ID
   */
  setDefaultTerminalId(terminalId: number): void {
    this.config.defaultTerminalId = terminalId
  }

  /**
   * 获取默认终端ID
   */
  getDefaultTerminalId(): number | undefined {
    return this.config.defaultTerminalId
  }

  /**
   * 设置默认工作目录
   */
  setDefaultWorkingDirectory(directory: string): void {
    this.config.defaultWorkingDirectory = directory
  }

  /**
   * 获取默认工作目录
   */
  getDefaultWorkingDirectory(): string | undefined {
    return this.config.defaultWorkingDirectory
  }

  /**
   * 启用/禁用安全模式
   */
  setSafeMode(enabled: boolean): void {
    this.config.safeMode = enabled
  }

  /**
   * 检查是否启用安全模式
   */
  isSafeModeEnabled(): boolean {
    return this.config.safeMode || false
  }

  /**
   * 添加允许的命令
   */
  addAllowedCommand(command: string): void {
    if (!this.config.allowedCommands) {
      this.config.allowedCommands = []
    }
    if (!this.config.allowedCommands.includes(command)) {
      this.config.allowedCommands.push(command)
    }
  }

  /**
   * 移除允许的命令
   */
  removeAllowedCommand(command: string): void {
    if (this.config.allowedCommands) {
      this.config.allowedCommands = this.config.allowedCommands.filter(cmd => cmd !== command)
    }
  }

  /**
   * 添加禁止的命令
   */
  addBlockedCommand(command: string): void {
    if (!this.config.blockedCommands) {
      this.config.blockedCommands = []
    }
    if (!this.config.blockedCommands.includes(command)) {
      this.config.blockedCommands.push(command)
    }
  }

  /**
   * 移除禁止的命令
   */
  removeBlockedCommand(command: string): void {
    if (this.config.blockedCommands) {
      this.config.blockedCommands = this.config.blockedCommands.filter(cmd => cmd !== command)
    }
  }

  /**
   * 获取Agent状态信息
   */
  getStatus(): {
    name: string
    description: string
    toolsCount: number
    safeMode: boolean
    defaultTerminalId?: number
    defaultWorkingDirectory?: string
    allowedCommandsCount: number
    blockedCommandsCount: number
  } {
    return {
      name: this.name,
      description: this.description,
      toolsCount: this.tools.length,
      safeMode: this.config.safeMode || false,
      defaultTerminalId: this.config.defaultTerminalId,
      defaultWorkingDirectory: this.config.defaultWorkingDirectory,
      allowedCommandsCount: this.config.allowedCommands?.length || 0,
      blockedCommandsCount: this.config.blockedCommands?.length || 0,
    }
  }
}

/**
 * 创建默认的终端Agent实例
 */
export function createTerminalAgent(config?: Partial<TerminalAgentConfig>): TerminalAgent {
  return new TerminalAgent(config)
}

/**
 * 创建安全模式的终端Agent
 */
export function createSafeTerminalAgent(config?: Partial<TerminalAgentConfig>): TerminalAgent {
  return new TerminalAgent({
    ...config,
    safeMode: true,
    blockedCommands: [
      'rm -rf /',
      'sudo rm -rf',
      'format',
      'del /f /s /q',
      'shutdown',
      'reboot',
      'halt',
      'poweroff',
      'init 0',
      'init 6',
      'dd if=',
      'mkfs',
      'fdisk',
      'parted',
    ],
  })
}

/**
 * 创建开发者模式的终端Agent（较少限制）
 */
export function createDeveloperTerminalAgent(config?: Partial<TerminalAgentConfig>): TerminalAgent {
  return new TerminalAgent({
    ...config,
    safeMode: false,
    description: '开发者终端操作专家，具有更高权限，能够执行系统级命令和操作',
  })
}
