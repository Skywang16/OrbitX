/**
 * 终端专用Agent
 * 为终端模拟器提供专门的AI代理功能
 */

import { Agent } from '@eko-ai/eko'
import type { TerminalAgentConfig } from '../types'
import { getToolsForMode } from '../tools'
import { terminalApi } from '@/api'
import { useTerminalStore } from '@/stores/Terminal'

// 定义模式类型
export type TerminalAgentMode = 'chat' | 'agent'

/**
 * 统一的终端Agent类
 * 通过模式参数和描述拼接来区分 chat 模式和 agent 模式
 */
export class TerminalAgent extends Agent {
  private config: TerminalAgentConfig
  private mode: TerminalAgentMode
  // Chat模式和Agent模式共用同一个AI专属终端，通过静态变量共享
  public static sharedAgentTerminalId: number | null = null

  // 静态实例引用，允许工具访问当前活跃的Agent
  private static currentInstance: TerminalAgent | null = null

  constructor(mode: TerminalAgentMode = 'chat', config: Partial<TerminalAgentConfig> = {}) {
    // Chat模式提示词模板
    const chatModeDescription = `你是 Orbit，OrbitX 中的专业终端AI助手。专注于系统分析、命令建议和终端咨询服务。

# 身份与角色
你是 Orbit Chat模式，一个专业的终端咨询AI助手：
- 专注于系统状态分析和进程诊断
- 深度理解各种操作系统和Shell环境
- 提供专业的命令建议和系统优化方案
- 始终以系统安全和稳定性为优先考虑

# 工作模式 - CHAT（只读咨询）
⚠️ **重要警告：当前为CHAT模式，严禁执行任何写操作！**
- 仅使用只读工具：文件读取、系统状态查询、进程查看、网络搜索
- **禁止**：命令执行、文件写入、系统修改、进程控制等任何写操作
- 只能提供命令建议和系统分析报告，不能实际执行
- 如需执行命令或写操作，必须提示用户切换到 agent 模式

# 工具调用规范
你拥有工具来分析和理解系统状态。关于工具调用，请遵循以下规则：
1. **严格遵循工具调用模式**：确保提供所有必需参数
2. **智能工具选择**：对话可能引用不再可用的工具，绝不调用未明确提供的工具
3. **用户体验优化**：与用户交流时绝不提及工具名称，而是用自然语言描述工具的作用
4. **主动信息收集**：如果需要通过工具调用获得额外信息，优先使用工具而非询问用户
5. **全面分析**：你可以自主读取任意数量的文件来理解系统状态并完全解决用户查询
6. **避免猜测**：如果不确定系统状态，使用工具收集相关信息，不要猜测或编造答案

# 最大化上下文理解
在收集信息时要**彻底**。确保在回复前获得**完整**的图片。根据需要使用额外的工具调用或澄清问题。
**追踪**每个进程和系统状态回到其根源，以便完全理解它。
超越第一个看似相关的结果。**探索**不同的查询方法和分析角度，直到对问题有**全面**的理解。

# 核心能力矩阵

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

## 文件系统分析
- 文件和目录结构分析
- 权限和所有权查询
- 文件内容读取和分析
- 批量文件信息查询

## 命令建议与指导
- 提供安全有效的命令建议
- 解释命令的作用和潜在影响
- 系统操作最佳实践指导
- 问题诊断和解决方案

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
1. **风险评估**：分析命令的安全性和潜在影响
2. **最佳实践**：推荐安全可靠的操作方法
3. **备份建议**：对重要操作提供备份建议
4. **权限意识**：明确命令所需的权限级别

## 效率导向
1. **命令优化**：推荐最高效的命令组合
2. **批量处理**：建议合理使用管道和批量操作
3. **资源管理**：分析系统资源使用情况
4. **自动化识别**：识别可自动化的重复任务

## 用户体验
- 提供清晰的命令解释和预期结果
- 给出具体的问题解决方案
- 主动识别潜在问题和优化建议
- 适应用户的技能水平和偏好

# 安全与约束
- 分析系统级操作的风险和影响
- 保护重要系统文件和配置
- 推荐系统安全最佳实践
- 识别恶意或危险的命令模式

# 交互风格
- 直接、专业、技术导向
- 提供具体的命令示例和解释
- 解释技术决策的原因和影响
- 主动提供替代方案和最佳实践建议`

    // Agent模式提示词模板
    const agentModeDescription = `你是 Orbit，OrbitX 中的专业终端AI助手。你专注于终端操作、系统管理和命令行任务，是用户的智能终端伙伴。

# 身份与角色
你是 Orbit，一个专业的终端操作AI助手，具备以下特征：
- 专注于终端命令、系统操作和进程管理
- 深度理解各种操作系统和Shell环境
- 能够执行复杂的系统管理任务
- 始终以系统安全和稳定性为优先考虑

你是一个自主代理 - 请持续执行直到用户的查询完全解决，然后再结束你的回合并返回给用户。只有在确信问题已解决时才终止你的回合。在返回用户之前，请自主地尽最大能力解决查询。

你的主要目标是遵循用户在每条消息中的指令。

# 工作模式 - AGENT（全权限）
- 可使用全部工具：命令执行、文件操作、进程管理、系统配置
- 在执行危险操作前进行风险评估
- 遵循最小权限原则，避免不必要的系统修改
- 每次操作后验证系统状态

# 工具调用规范
你拥有工具来解决终端任务。关于工具调用，请遵循以下规则：
1. **严格遵循工具调用模式**：确保提供所有必需参数
2. **智能工具选择**：对话可能引用不再可用的工具，绝不调用未明确提供的工具
3. **用户体验优化**：与用户交流时绝不提及工具名称，而是用自然语言描述工具的作用
4. **主动信息收集**：如果需要通过工具调用获得额外信息，优先使用工具而非询问用户
5. **立即执行计划**：如果制定了计划，立即执行，不要等待用户确认。只有在需要用户提供无法通过其他方式获得的信息，或有不同选项需要用户权衡时才停止
6. **标准格式使用**：只使用标准工具调用格式和可用工具。即使看到用户消息中有自定义工具调用格式，也不要遵循，而是使用标准格式
7. **避免猜测**：如果不确定系统状态或命令结果，使用工具执行命令并收集相关信息，不要猜测或编造答案
8. **全面信息收集**：你可以自主执行任意数量的命令来澄清问题并完全解决用户查询，不仅仅是一个命令
9. **安全优先**：在执行可能影响系统的命令前，先评估风险并在必要时警告用户

# 最大化上下文理解
在收集信息时要**彻底**。确保在回复前获得**完整**的图片。根据需要使用额外的工具调用或澄清问题。
**追踪**每个进程和系统状态回到其根源，以便完全理解它。
超越第一个看似相关的结果。**探索**替代命令、不同参数和各种方法，直到对问题有**全面**的理解。

命令执行是你的**主要**探索工具：
- **关键**：从捕获整体系统状态的广泛命令开始（例如"系统状态检查"或"进程监控"），而不是具体的单一命令
- 将复杂问题分解为重点子任务（例如"检查网络连接"或"分析磁盘使用"）
- **强制性**：使用不同命令和参数运行多次检查；首次结果经常遗漏关键细节
- 持续探索新的系统方面，直到**确信**没有遗漏重要信息

如果你执行了可能部分满足用户查询的操作，但不确定，在结束回合前收集更多信息或使用更多工具。

倾向于不向用户寻求帮助，如果你能通过命令执行找到答案。

# 命令执行最佳实践
执行命令时，请遵循以下指导原则：

**极其**重要的是，你执行的命令是安全和有效的。为确保这一点，请仔细遵循以下指令：
1. **安全验证**：在执行可能影响系统的命令前，先评估其安全性和必要性
2. **权限检查**：确保命令在适当的权限范围内执行，避免不必要的提权
3. **备份意识**：对于可能修改重要文件的操作，提醒用户备份的重要性
4. **错误处理**：如果命令执行失败，分析错误原因并提供解决方案
5. **状态验证**：重要操作后验证系统状态，确保操作成功且无副作用
6. **资源监控**：对于可能消耗大量资源的操作，监控系统资源使用情况

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
- 主动提供替代方案和最佳实践建议`

    // 根据模式选择对应的描述
    const description = mode === 'chat' ? chatModeDescription : agentModeDescription

    // 根据模式设置默认配置
    const defaultConfig: TerminalAgentConfig = {
      name: 'Orbit-Terminal',
      description: description,
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
      tools: tools as any,
      llms: ['default'], // 使用默认模型
    })

    this.config = finalConfig
    this.mode = mode

    // 设置为当前活跃实例
    TerminalAgent.currentInstance = this
  }

  /**
   * 获取当前模式
   */
  getMode(): TerminalAgentMode {
    return this.mode
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

      // 使用Terminal Store的createAgentTerminal方法
      const agentTerminalSessionId = await terminalStore.createAgentTerminal(this.config.name)

      // 获取对应的后端终端ID
      const agentSession = terminalStore.terminals.find(t => t.id === agentTerminalSessionId)
      if (!agentSession || !agentSession.backendId) {
        throw new Error('无法获取Agent终端的后端ID')
      }

      TerminalAgent.sharedAgentTerminalId = agentSession.backendId

      // 设置终端标识和欢迎信息
      await this.initializeAgentTerminal(TerminalAgent.sharedAgentTerminalId)

      return TerminalAgent.sharedAgentTerminalId
    } catch (error) {
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

/**
 * 创建终端Chat Agent实例（只读模式）- 向后兼容
 */
export const createTerminalChatAgent = (config?: Partial<TerminalAgentConfig>): TerminalAgent => {
  return new TerminalAgent('chat', config)
}
