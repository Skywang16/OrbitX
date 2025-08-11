/**
 * 终端专用Agent
 * 为终端模拟器提供专门的AI代理功能
 */

import { Agent } from '@eko-ai/eko'
import type { TerminalAgentConfig } from '../types'
import { getToolsForMode } from '../tools'
import { terminalAPI } from '@/api/terminal'
import { useTerminalStore } from '@/stores/Terminal'
/**
 * 终端Agent类
 * 继承自Eko的Agent基类，专门为终端操作优化
 */
export class TerminalAgent extends Agent {
  private config: TerminalAgentConfig
  private agentTerminalId: number | null = null
  private baseDescription: string

  // 静态实例引用，允许工具访问当前活跃的Agent
  private static currentInstance: TerminalAgent | null = null

  constructor(config: Partial<TerminalAgentConfig> = {}) {
    // 默认配置
    const defaultConfig: TerminalAgentConfig = {
      name: 'Orbit',
      description: `你是 Orbit，OrbitX 中的 AI 助手伙伴。你有两个工作模式：chat 与 agent，它们都基于系统的 agent 框架。

📌 模式说明
- chat 模式：只能使用只读工具（如读取文件、网页获取/搜索），严禁任何写入、命令执行或会改变系统/数据状态的操作
- agent 模式：可以使用全部工具，包含写入、命令执行等能力；在危险操作前需再次确认

你是专属于OrbitX终端应用的AI智能助手，为用户提供强大的终端操作支持。

🤖 **身份说明**
- 你是 Orbit，OrbitX 的专属AI助手，负责帮助用户完成各种终端任务
- 你不是eko，不是通用AI，而是专门为OrbitX应用定制的智能助手
- 你深度集成在用户的OrbitX环境中，了解用户的工作流程和习惯

💻 **你的工作环境**
- 你运行在用户的OrbitX终端应用中
- 你可以直接访问用户的文件系统、执行命令、管理进程
- 你是OrbitX用户的得力助手，就像一个非常聪明的命令行伙伴

🛠️ **核心能力**
- 执行shell命令和系统操作
- 文件和目录管理（读取、写入、创建、删除）
- 批量文件处理和内容分析  
- 网络请求和数据获取
- 会话记忆和上下文管理
- 命令解释和错误诊断
- 自动化脚本编写和执行

💡 **交互风格**
- 友好、专业、高效
- 主动理解用户意图，提供最佳解决方案
- 在执行危险操作前会提醒用户
- 提供清晰的操作步骤和结果说明

🔒 **安全保护**
- 智能识别危险命令并提醒用户
- 支持安全模式防止误操作
- 保护用户数据和系统安全
`,
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
      tools: getToolsForMode('chat') as any, // 初始化为chat模式的只读工具
      llms: ['default'], // 使用默认模型
    })

    this.config = finalConfig
    this.baseDescription = finalConfig.description

    // 设置为当前活跃实例
    TerminalAgent.currentInstance = this
  }

  /**
   * 获取Agent配置
   */
  getConfig(): TerminalAgentConfig {
    return { ...this.config }
  }

  /**
   * 切换工作模式并更新工具/提示词
   */
  setMode(mode: 'chat' | 'agent'): void {
    // 更新工具权限
    this.tools = getToolsForMode(mode) as any

    // 根据模式强化描述中的权限提醒
    const modeNotice =
      mode === 'chat'
        ? `\n\n🔐 当前模式：chat（只读）\n- 仅可使用读取类工具（读取文件/网络）\n- 禁止写入、执行命令、修改系统/数据\n- 如需执行，请用户切换到 agent 模式`
        : `\n\n🛠️ 当前模式：agent（全权限）\n- 可使用全部工具（含写入/命令执行）\n- 危险操作前需给出风险提示并征得确认`

    this.description = `${this.baseDescription}${modeNotice}`
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
    agentTerminalId?: number | null
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
      agentTerminalId: this.agentTerminalId,
    }
  }

  /**
   * 创建或获取Agent专属终端
   */
  async ensureAgentTerminal(): Promise<number> {
    try {
      // 如果已经有专属终端，检查是否还存在
      if (this.agentTerminalId !== null) {
        const terminals = await terminalAPI.listTerminals()
        if (terminals.includes(this.agentTerminalId)) {
          // 激活现有的Agent终端
          await this.activateAgentTerminal(this.agentTerminalId)
          return this.agentTerminalId
        } else {
          // 终端已被关闭，清空引用
          this.agentTerminalId = null
        }
      }

      const terminalStore = useTerminalStore()

      // 使用Terminal Store的createAgentTerminal方法
      const agentTerminalSessionId = await terminalStore.createAgentTerminal(this.config.name)

      // 获取对应的后端终端ID
      const agentSession = terminalStore.terminals.find(t => t.id === agentTerminalSessionId)
      if (!agentSession || !agentSession.backendId) {
        throw new Error('无法获取Agent终端的后端ID')
      }

      this.agentTerminalId = agentSession.backendId

      // 设置终端标识和欢迎信息
      await this.initializeAgentTerminal(this.agentTerminalId)

      return this.agentTerminalId
    } catch (error) {
      console.error('创建Agent专属终端失败:', error)
      throw new Error(`无法创建AI助手专属终端: ${error instanceof Error ? error.message : String(error)}`)
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
        await terminalAPI.writeToTerminal({
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
      const { useTerminalStore } = await import('@/stores/Terminal')
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
   * 获取Agent专属终端ID
   */
  getAgentTerminalId(): number | null {
    return this.agentTerminalId
  }

  /**
   * 清理Agent专属终端
   */
  async cleanupAgentTerminal(): Promise<void> {
    if (this.agentTerminalId !== null) {
      try {
        // 通过Terminal Store关闭终端
        const { useTerminalStore } = await import('@/stores/Terminal')
        const terminalStore = useTerminalStore()
        // 找到对应的会话并关闭
        const agentSession = terminalStore.terminals.find(t => t.backendId === this.agentTerminalId)
        if (agentSession) {
          await terminalStore.closeTerminal(agentSession.id)
        } else {
          // 降级到直接关闭后端终端
          await terminalAPI.closeTerminal(this.agentTerminalId)
        }
      } catch (error) {
        console.warn('关闭Agent专属终端失败:', error)
      } finally {
        this.agentTerminalId = null
      }
    }
  }

  /**
   * 获取Agent专属终端的会话信息
   */
  async getAgentTerminalSession() {
    if (!this.agentTerminalId) {
      return null
    }

    try {
      const { useTerminalStore } = await import('@/stores/Terminal')
      const terminalStore = useTerminalStore()
      return terminalStore.terminals.find(t => t.backendId === this.agentTerminalId) || null
    } catch (error) {
      console.warn('获取Agent终端会话信息失败:', error)
      return null
    }
  }

  /**
   * 确保Agent工具能够访问专属终端
   */
  getTerminalIdForTools(): number | null {
    return this.agentTerminalId
  }

  /**
   * 获取当前活跃的Agent实例（供工具使用）
   */
  static getCurrentInstance(): TerminalAgent | null {
    return TerminalAgent.currentInstance
  }
}

/**
 * 创建默认的终端Agent实例
 */
export const createTerminalAgent = (config?: Partial<TerminalAgentConfig>): TerminalAgent => {
  return new TerminalAgent(config)
}

/**
 * 创建安全模式的终端Agent
 */
export const createSafeTerminalAgent = (config?: Partial<TerminalAgentConfig>): TerminalAgent => {
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
export const createDeveloperTerminalAgent = (config?: Partial<TerminalAgentConfig>): TerminalAgent => {
  return new TerminalAgent({
    ...config,
    safeMode: false,
    description: `你是 Orbit，OrbitX 终端应用的高级开发者模式AI助手，专为资深开发者提供无限制的终端操作支持。

🤖 **身份说明**
- 你是 Orbit，OrbitX 的专属AI助手（开发者模式）
- 你不是eko，不是通用AI，而是专门为OrbitX应用的高级用户定制的智能助手
- 你拥有更高权限，可以执行系统级操作，是开发者的超级终端伙伴

💻 **开发者环境**
- 你运行在用户的OrbitX终端应用中，拥有高级权限
- 你可以执行任何命令，包括系统级操作
- 你是OrbitX开发者的得力助手，理解复杂的开发需求

🛠️ **开发者专属能力**
- 无限制的shell命令执行
- 系统级文件操作和权限管理
- 高级网络和系统诊断
- 复杂的自动化脚本编写和执行
- 开发环境配置和部署操作
- Git操作和代码管理
- 服务器管理和运维操作

💡 **开发者交互模式**
- 直接、高效、专业
- 理解开发者的专业术语和需求
- 提供深度技术支持和解决方案
- 快速执行复杂操作

⚠️ **权限提醒**
- 开发者模式下安全限制已解除
- 你会执行用户要求的任何命令
- 用户需要自行承担操作风险
- 建议重要操作前做好备份
`,
  })
}
