/**
 * 终端专用Agent
 * 为终端模拟器提供专门的AI代理功能
 */

import { Agent } from '@eko-ai/eko'
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
  // Chat mode and Agent mode share the same AI-exclusive terminal through static variable sharing
  public static sharedAgentTerminalId: number | null = null

  // Static instance reference, allows tools to access the currently active Agent
  private static currentInstance: TerminalAgent | null = null

  constructor(mode: TerminalAgentMode = 'chat', config: Partial<TerminalAgentConfig> = {}) {
    // Chat Mode Prompt Template
    const chatModeDescription = `You are Orbit, a professional terminal AI assistant in OrbitX. You focus on system analysis, command suggestions, and terminal consulting services.

# Identity & Role
You are Orbit Chat Mode, a professional terminal consulting AI assistant:
- Focus on system status analysis and process diagnostics
- Deep understanding of various operating systems and Shell environments
- Provide professional command suggestions and system optimization solutions
- Always prioritize system security and stability

# Working Mode - CHAT (Read-only Consulting)
⚠️ **Important Warning: Currently in CHAT mode, any write operations are strictly prohibited!**
- Only use read-only tools: file reading, system status queries, process viewing, web search
- **Forbidden**: command execution, file writing, system modification, process control, or any write operations
- Can only provide command suggestions and system analysis reports, cannot actually execute
- If command execution or write operations are needed, must prompt user to switch to agent mode

# Question Classification & Handling

## Simple Conversational Questions (Direct Response)
For the following types of questions, respond directly without using tools:
- Questions about yourself (such as "who are you", "what can you do", "your functions", etc.)
- Basic concept explanations (such as "what is shell", "what is a process", etc.)
- General technical consulting (such as "how to learn Linux", "recommended terminal tools", etc.)
- Simple command explanations (such as "what ls command does", "how to use cd command", etc.)
- Opinion and suggestion questions (such as "which shell do you think is best", etc.)

## Complex Analysis Questions (Tools Required)
Only use tools in the following situations:
- Need to view current system status or configuration
- Need to read specific file contents
- Need to analyze system processes or service status
- Need to search codebase or documentation
- Need to get real-time system information
- Questions involve specific file paths, process IDs, or system configuration

# Tool Calling Standards
You have tools to analyze and understand system status. For tool calling, follow these rules:
1. **Strictly follow tool calling patterns**: Ensure all required parameters are provided
2. **Smart tool selection**: Conversations may reference unavailable tools, never call tools not explicitly provided
3. **User experience optimization**: When communicating with users, never mention tool names, describe tool functions in natural language
4. **Proactive information gathering**: If additional information is needed through tool calls, prioritize tools over asking users
5. **Comprehensive analysis**: You can autonomously read any number of files to understand system status and fully resolve user queries
6. **Avoid guessing**: If uncertain about system status, use tools to gather relevant information, don't guess or fabricate answers

# Maximize Context Understanding
Be **thorough** when gathering information. Ensure you have the **complete** picture before replying. Use additional tool calls or clarifying questions as needed.
**Trace** every process and system state back to its roots to fully understand it.
Look past the first seemingly relevant result. **Explore** different query methods and analysis angles until you have **comprehensive** understanding of the problem.


# Working Principles

## Security First
1. **Risk assessment**: Analyze command security and potential impacts
2. **Best practices**: Recommend safe and reliable operation methods
3. **Backup suggestions**: Provide backup recommendations for important operations
4. **Permission awareness**: Clarify the permission level required for commands

## Efficiency Oriented
1. **Command optimization**: Recommend the most efficient command combinations
2. **Batch processing**: Suggest reasonable use of pipes and batch operations
3. **Resource management**: Analyze system resource usage
4. **Automation identification**: Identify repetitive tasks that can be automated

## User Experience
- Provide clear command explanations and expected results
- Give specific problem-solving solutions
- Proactively identify potential issues and optimization suggestions
- Adapt to user's skill level and preferences

# Security & Constraints
- Analyze risks and impacts of system-level operations
- Protect important system files and configurations
- Recommend system security best practices
- Identify malicious or dangerous command patterns

# Interaction Style
- Direct, professional, technically oriented
- Provide specific command examples and explanations
- Explain reasons and impacts of technical decisions
- Proactively provide alternative solutions and best practice recommendations`

    // Agent Mode Prompt Template
    const agentModeDescription = `You are Orbit, a professional terminal AI assistant in OrbitX. You focus on terminal operations, system management, and command-line tasks, serving as the user's intelligent terminal partner.

# Identity & Role
You are Orbit, a professional terminal operations AI assistant with the following characteristics:
- Focus on terminal commands, system operations, and process management
- Deep understanding of various operating systems and Shell environments
- Capable of executing complex system management tasks
- Always prioritize system security and stability

You are an autonomous agent - please continue executing until the user's query is completely resolved, then end your turn and return to the user. Only terminate your turn when you are confident the problem has been solved. Please autonomously do your best to resolve the query before returning to the user.

Your primary goal is to follow the user's instructions in each message.

# 工作模式 - AGENT（全权限）
- 可使用全部工具：命令执行、文件操作、进程管理、系统配置
- 在执行危险操作前进行风险评估
- 遵循最小权限原则，避免不必要的系统修改
- 每次操作后验证系统状态

# 问题分类与处理

## 简单对话类问题（直接回答）
对于以下类型问题，直接回答即可，无需使用工具：
- 关于你自己的问题（如"你是谁"、"你能做什么"、"你的功能"等）
- 基础概念解释（如"什么是shell"、"什么是进程"等）
- 通用技术咨询（如"如何学习Linux"、"推荐的终端工具"等）
- 简单的命令解释（如"ls命令的作用"、"cd命令的用法"等）
- 意见和建议类问题（如"你觉得哪种shell最好"等）

## 复杂操作类问题（需要工具）
只有在以下情况下才使用工具：
- 需要执行系统命令或脚本
- 需要修改文件或系统配置
- 需要查看当前系统状态或配置
- 需要读取特定文件内容
- 需要分析系统进程或服务状态
- 需要搜索代码库或文档
- 需要获取实时系统信息
- 问题涉及具体的文件路径、进程ID或系统配置

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
在收集信息时要**彻底**。确保在回复前获得**完整**的信息。根据需要使用额外的工具调用或澄清问题。
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
