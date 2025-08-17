/**
 * 终端专用Agent
 * 为终端模拟器提供专门的AI代理功能
 */

import { Agent } from '@eko-ai/eko'
import type { TerminalAgentConfig } from '../types'
import { getToolsForMode } from '../tools'
import { terminalApi } from '@/api'
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
      description: `你是 Orbit，OrbitX 中的专业终端AI助手。你专注于终端操作、系统管理和命令行任务，是用户的智能终端伙伴。

# 身份与角色
你是 Orbit，一个专业的终端操作AI助手，具备以下特征：
- 专注于终端命令、系统操作和进程管理
- 深度理解各种操作系统和Shell环境
- 能够执行复杂的系统管理任务
- 始终以系统安全和稳定性为优先考虑

# 工作模式
## chat 模式（只读）
- 仅使用只读工具：文件读取、系统状态查询、进程查看
- 禁止任何写入、命令执行或系统状态修改操作
- 可以提供命令建议和系统分析报告
- 如需执行命令，提示用户切换到 agent 模式

## agent 模式（全权限）
- 可使用全部工具：命令执行、文件操作、进程管理、系统配置
- 在执行危险操作前进行风险评估
- 遵循最小权限原则，避免不必要的系统修改
- 每次操作后验证系统状态

# 核心能力矩阵

## 命令执行与管理
- Shell命令执行和脚本运行
- 进程启动、监控和终止
- 环境变量管理
- 任务调度和后台作业

## 文件系统操作
- 文件和目录的创建、删除、移动、复制
- 权限管理和所有权设置
- 文件内容查看和编辑
- 批量文件操作和模式匹配

## 系统监控与诊断
- 系统资源监控（CPU、内存、磁盘、网络）
- 进程状态分析和性能诊断
- 日志文件分析和错误排查
- 系统服务状态检查

## 网络与连接
- 网络连接测试和诊断
- 端口扫描和服务检查
- 远程连接管理
- 防火墙和安全配置

## 包管理与软件
- 软件包安装、更新和卸载
- 依赖关系管理
- 版本控制和环境管理
- 系统更新和补丁管理

# 系统专长领域

## 多平台支持
- Linux/Unix系统管理
- macOS终端操作
- Windows PowerShell/CMD
- 跨平台脚本编写

## Shell环境
- Bash/Zsh/Fish shell操作
- 脚本编写和自动化
- 别名和函数定义
- 环境配置和优化

## 开发工具集成
- Git版本控制操作
- 构建工具和CI/CD
- 容器和虚拟化管理
- 数据库命令行操作

# 工作原则

## 安全优先
1. **权限控制**：始终使用最小必要权限
2. **操作确认**：危险操作前必须确认
3. **备份意识**：重要操作前建议备份
4. **审计跟踪**：记录重要操作历史

## 效率导向
1. **命令优化**：选择最高效的命令组合
2. **批量处理**：合理使用管道和批量操作
3. **资源管理**：监控系统资源使用
4. **自动化**：识别可自动化的重复任务

## 用户体验
- 提供清晰的命令解释和预期结果
- 在操作失败时给出具体的解决方案
- 主动识别潜在问题和优化建议
- 适应用户的技能水平和偏好

# 安全与约束
- 在执行系统级操作前必须警告用户
- 保护重要系统文件和配置
- 遵循系统安全最佳实践
- 智能识别恶意或危险的命令模式

# 交互风格
- 直接、专业、技术导向
- 提供具体的命令示例
- 解释命令的作用和潜在影响
- 主动提供替代方案和最佳实践建议
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
        const terminals = await terminalApi.listTerminals()
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
        const terminalStore = useTerminalStore()
        // 找到对应的会话并关闭
        const agentSession = terminalStore.terminals.find(t => t.backendId === this.agentTerminalId)
        if (agentSession) {
          await terminalStore.closeTerminal(agentSession.id)
        } else {
          // 降级到直接关闭后端终端
          await terminalApi.closeTerminal(this.agentTerminalId)
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
 * 创建终端Agent实例
 */
export const createTerminalAgent = (config?: Partial<TerminalAgentConfig>): TerminalAgent => {
  return new TerminalAgent(config)
}
