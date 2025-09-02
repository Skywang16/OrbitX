/**
 * 终端专用Agent
 * 为终端模拟器提供专门的AI代理功能
 */

import { Agent } from '@/eko-core'
import type { TerminalAgentConfig } from '../types'
import { getToolsForMode } from '../tools'
import { terminalApi } from '@/api'
import { useTerminalStore } from '@/stores/Terminal'

// Define mode types
export type TerminalAgentMode = 'chat' | 'agent'

/**
 * Unified Terminal Agent class
 * Distinguishes between chat mode and agent mode through mode parameters and description concatenation
 */
export class TerminalAgent extends Agent {
  private config: TerminalAgentConfig
  private mode: TerminalAgentMode
  // Shared AI-exclusive terminal across mode switches
  public static sharedAgentTerminalId: number | null = null

  // Static instance reference, allows tools to access the currently active Agent
  private static currentInstance: TerminalAgent | null = null

  constructor(mode: TerminalAgentMode = 'chat', config: Partial<TerminalAgentConfig> = {}) {
    // 根据模式设置默认配置
    const defaultConfig: TerminalAgentConfig = {
      name: 'Orbit',
      description: '', // 将通过组件系统动态生成
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

    // 根据模式选择工具
    const tools = getToolsForMode(mode)

    // 调用父类构造函数
    super({
      name: finalConfig.name,
      description: finalConfig.description,
      tools: tools,
      llms: ['default'], // 使用默认模型
    })

    this.config = finalConfig
    this.mode = mode

    // 设置为当前活跃实例
    TerminalAgent.currentInstance = this

    // 生成模式专用的提示词
    this.generateModeDescription()
  }

  /**
   * 动态生成模式专用的提示词描述
   */
  private generateModeDescription(): void {
    if (this.mode === 'chat') {
      this.description = `You are ${this.config.name}, a skilled DevOps engineer and terminal expert.

CHAT MODE (Read-Only): Analyze files and provide command suggestions
- Use read_file, read_directory, orbit_search, web_fetch
- DO NOT execute commands or modify files
- Provide detailed analysis and step-by-step command suggestions
- For execution needs, tell user to switch to agent mode`
    } else {
      this.description = `You are ${this.config.name}, a skilled DevOps engineer and terminal expert.

AGENT MODE (Full Authority): Execute commands and complete tasks autonomously  
- Use all tools: shell commands, file operations, system modifications
- Work methodically: analyze → plan → execute → verify → complete
- Continue until task fully resolved, then return to user
- Implement safety checks before destructive operations`
    }
  }

  /**
   * 获取当前模式
   */
  getMode(): TerminalAgentMode {
    return this.mode
  }

  /**
   * 设置模式并更新工具配置
   */
  setMode(mode: TerminalAgentMode): void {
    if (this.mode === mode) {
      return // 模式未改变，无需更新
    }

    this.mode = mode

    // 更新工具配置
    const tools = getToolsForMode(mode)
    this.tools = tools

    // 重新生成提示词
    this.generateModeDescription()
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
  }

  /**
   * 获取当前活跃的Agent实例（供工具使用）
   */
  static getCurrentInstance(): TerminalAgent | null {
    return TerminalAgent.currentInstance
  }

  /**
   * 获取Agent专属终端ID（共享终端）
   */
  getAgentTerminalId(): number | null {
    return TerminalAgent.sharedAgentTerminalId
  }

  /**
   * 获取工具使用的终端ID（共享终端）
   */
  getTerminalIdForTools(): number | null {
    return TerminalAgent.sharedAgentTerminalId
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
    mode: string
    toolsCount: number
    safeMode: boolean
    defaultTerminalId?: number
    defaultWorkingDirectory?: string
    allowedCommandsCount: number
    blockedCommandsCount: number
    agentTerminalId?: number | null
  } {
    return {
      name: this.name,
      description: this.description,
      mode: this.mode,
      toolsCount: this.tools.length,
      safeMode: this.config.safeMode || false,
      defaultTerminalId: this.config.defaultTerminalId,
      defaultWorkingDirectory: this.config.defaultWorkingDirectory,
      allowedCommandsCount: this.config.allowedCommands?.length || 0,
      blockedCommandsCount: this.config.blockedCommands?.length || 0,
      agentTerminalId: TerminalAgent.sharedAgentTerminalId,
    }
  }

  /**
   * 创建或获取Agent专属终端（共享终端）
   */
  async ensureAgentTerminal(): Promise<number> {
    try {
      // 如果已经有共享终端，检查是否还存在
      if (TerminalAgent.sharedAgentTerminalId !== null) {
        const terminals = await terminalApi.listTerminals()
        if (terminals.includes(TerminalAgent.sharedAgentTerminalId)) {
          // 激活现有的共享终端
          await this.activateAgentTerminal(TerminalAgent.sharedAgentTerminalId)
          return TerminalAgent.sharedAgentTerminalId
        } else {
          // 终端已被关闭，清空引用
          TerminalAgent.sharedAgentTerminalId = null
        }
      }

      const terminalStore = useTerminalStore()

      // Use Terminal Store's createAgentTerminal method
      const agentTerminalSessionId = await terminalStore.createAgentTerminal(this.config.name)

      // Get corresponding backend terminal ID
      const agentSession = terminalStore.terminals.find(t => t.id === agentTerminalSessionId)
      if (!agentSession || !agentSession.backendId) {
        throw new Error('Unable to get Agent terminal backend ID')
      }

      TerminalAgent.sharedAgentTerminalId = agentSession.backendId

      // Set terminal identifier and welcome message
      await this.initializeAgentTerminal(TerminalAgent.sharedAgentTerminalId)

      return TerminalAgent.sharedAgentTerminalId
    } catch (error) {
      throw new Error(
        `Unable to create AI assistant dedicated terminal: ${error instanceof Error ? error.message : String(error)}`
      )
    }
  }

  /**
   * 初始化Agent专属终端（仅在首次创建时调用）
   */
  private async initializeAgentTerminal(terminalId: number): Promise<void> {
    try {
      // 保持Agent终端干净，不输出欢迎信息
      // 只设置工作目录（如果配置了）
      if (this.config.defaultWorkingDirectory) {
        await terminalApi.writeToTerminal({
          paneId: terminalId,
          data: `cd "${this.config.defaultWorkingDirectory}"\n`,
        })
      }
    } catch (error) {
      console.warn('初始化Agent终端失败:', error)
    }
  }

  /**
   * 激活Agent专属终端（静默激活，不输出额外信息）
   */
  private async activateAgentTerminal(terminalId: number): Promise<void> {
    try {
      const terminalStore = useTerminalStore()

      // 找到对应的会话并激活（静默）
      const agentSession = terminalStore.terminals.find(t => t.backendId === terminalId)
      if (agentSession) {
        terminalStore.setActiveTerminal(agentSession.id)
      }
    } catch (error) {
      console.warn('激活Agent终端失败:', error)
    }
  }

  /**
   * 清理Agent专属终端（共享终端）
   */
  async cleanupAgentTerminal(): Promise<void> {
    if (TerminalAgent.sharedAgentTerminalId !== null) {
      try {
        // 通过Terminal Store关闭终端
        const terminalStore = useTerminalStore()
        // 找到对应的会话并关闭
        const agentSession = terminalStore.terminals.find(t => t.backendId === TerminalAgent.sharedAgentTerminalId)
        if (agentSession) {
          await terminalStore.closeTerminal(agentSession.id)
        } else {
          // 降级到直接关闭后端终端
          await terminalApi.closeTerminal(TerminalAgent.sharedAgentTerminalId)
        }
      } catch (error) {
        // 清理失败不影响程序运行
      } finally {
        TerminalAgent.sharedAgentTerminalId = null
      }
    }
  }

  /**
   * 获取Agent专属终端的会话信息（共享终端）
   */
  async getAgentTerminalSession() {
    if (!TerminalAgent.sharedAgentTerminalId) {
      return null
    }

    try {
      const terminalStore = useTerminalStore()
      return terminalStore.terminals.find(t => t.backendId === TerminalAgent.sharedAgentTerminalId) || null
    } catch (error) {
      return null
    }
  }
}

/**
 * 创建终端Agent实例
 * @param mode - 模式：'chat'（只读）或 'agent'（全权限）
 * @param config - 配置选项
 */
export const createTerminalAgent = (
  mode: TerminalAgentMode = 'chat',
  config?: Partial<TerminalAgentConfig>
): TerminalAgent => {
  return new TerminalAgent(mode, config)
}
