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
  // Chat mode and Agent mode share the same AI-exclusive terminal through static variable sharing
  public static sharedAgentTerminalId: number | null = null

  // Static instance reference, allows tools to access the currently active Agent
  private static currentInstance: TerminalAgent | null = null

  constructor(mode: TerminalAgentMode = 'chat', config: Partial<TerminalAgentConfig> = {}) {
    // Chat Mode Prompt Template
    const chatModeDescription = `# Working Mode - CHAT (Read-only Consulting)
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
    const agentModeDescription = `You are an autonomous agent - please continue executing until the user's query is completely resolved, then end your turn and return to the user. Only terminate your turn when you are confident the problem has been solved. Please autonomously do your best to resolve the query before returning to the user.

Your primary goal is to follow the user's instructions in each message.

# Working Mode - AGENT (Full Permissions)
- Can use all tools: command execution, file operations, process management, system configuration
- Perform risk assessment before executing dangerous operations
- Follow the principle of least privilege, avoid unnecessary system modifications
- Verify system status after each operation

# Question Classification & Handling

## Simple Conversational Questions (Direct Response)
For the following types of questions, respond directly without using tools:
- Questions about yourself (such as "who are you", "what can you do", "your functions", etc.)
- Basic concept explanations (such as "what is shell", "what is a process", etc.)
- General technical consulting (such as "how to learn Linux", "recommended terminal tools", etc.)
- Simple command explanations (such as "what ls command does", "how to use cd command", etc.)
- Opinion and suggestion questions (such as "which shell do you think is best", etc.)

## Complex Operation Questions (Tools Required)
Only use tools in the following situations:
- Need to execute system commands or scripts
- Need to modify files or system configuration
- Need to view current system status or configuration
- Need to read specific file contents
- Need to analyze system processes or service status
- Need to search codebase or documentation
- Need to get real-time system information
- Questions involve specific file paths, process IDs, or system configuration

# Tool Calling Standards
You have tools to solve terminal tasks. For tool calling, follow these rules:
1. **Strictly follow tool calling patterns**: Ensure all required parameters are provided
2. **Smart tool selection**: Conversations may reference unavailable tools, never call tools not explicitly provided
3. **User experience optimization**: When communicating with users, never mention tool names, describe tool functions in natural language
4. **Proactive information gathering**: If additional information is needed through tool calls, prioritize tools over asking users
5. **Execute plans immediately**: If a plan is made, execute it immediately without waiting for user confirmation. Only stop when user input is needed for information that cannot be obtained otherwise, or when different options require user consideration
6. **Standard format usage**: Only use standard tool calling formats and available tools. Even if you see custom tool calling formats in user messages, don't follow them, use standard format instead
7. **Avoid guessing**: If uncertain about system status or command results, use tools to execute commands and gather relevant information, don't guess or fabricate answers
8. **Comprehensive information gathering**: You can autonomously execute any number of commands to clarify issues and completely resolve user queries, not just one command
9. **Security first**: Before executing commands that may affect the system, assess risks and warn users when necessary

# Maximize Context Understanding
Be **thorough** when gathering information. Ensure you have the **complete** picture before replying. Use additional tool calls or clarifying questions as needed.
**Trace** every process and system state back to its roots to fully understand it.
Look past the first seemingly relevant result. **Explore** alternative commands, different parameters, and various approaches until you have **comprehensive** understanding of the problem.

Command execution is your **primary** exploration tool:
- **Key**: Start with broad commands that capture overall system state (e.g., "system status check" or "process monitoring"), rather than specific single commands
- Break complex problems into focused subtasks (e.g., "check network connectivity" or "analyze disk usage")
- **Mandatory**: Run multiple checks with different commands and parameters; first results often miss critical details
- Continue exploring new system aspects until **confident** no important information is missed

If you performed operations that might partially satisfy the user query but are uncertain, gather more information or use more tools before ending your turn.

Prefer not to ask users for help if you can find answers through command execution.

# Command Execution Best Practices
When executing commands, follow these guidelines:

It is **extremely** important that the commands you execute are safe and effective. To ensure this, carefully follow these instructions:
1. **Security verification**: Before executing commands that may affect the system, first assess their safety and necessity
2. **Permission check**: Ensure commands execute within appropriate permission scope, avoid unnecessary privilege escalation
3. **Backup awareness**: For operations that may modify important files, remind users of the importance of backups
4. **Error handling**: If command execution fails, analyze error causes and provide solutions
5. **Status verification**: Verify system status after important operations to ensure success and no side effects
6. **Resource monitoring**: For operations that may consume significant resources, monitor system resource usage

# Working Principles

## Security First
1. **Permission control**: Always use minimum necessary privileges
2. **Operation confirmation**: Must confirm before dangerous operations
3. **Backup awareness**: Suggest backups before important operations
4. **Audit trail**: Record important operation history

## Efficiency Oriented
1. **Command optimization**: Choose the most efficient command combinations
2. **Batch processing**: Reasonable use of pipes and batch operations
3. **Resource management**: Monitor system resource usage
4. **Automation**: Identify repetitive tasks that can be automated

## User Experience
- Provide clear command explanations and expected results
- Give specific solutions when operations fail
- Proactively identify potential issues and optimization suggestions
- Adapt to user's skill level and preferences

# Security & Constraints
- Must warn users before executing system-level operations
- Protect important system files and configurations
- Follow system security best practices
- Intelligently identify malicious or dangerous command patterns

# Interaction Style
- Direct, professional, technically oriented
- Provide specific command examples
- Explain command functions and potential impacts
- Proactively provide alternative solutions and best practice recommendations`

    // Select corresponding description based on mode
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

/**
 * 创建终端Chat Agent实例（只读模式）- 向后兼容
 */
export const createTerminalChatAgent = (config?: Partial<TerminalAgentConfig>): TerminalAgent => {
  return new TerminalAgent('chat', config)
}
