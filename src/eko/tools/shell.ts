/**
 * Shell命令执行工具
 */

import { ModifiableTool, type ToolExecutionContext } from './modifiable-tool'
import type { ToolResult } from '../types'
import { TerminalError, ValidationError } from './tool-error'
import { terminalAPI } from '@/api/terminal'
import { useTerminalStore } from '@/stores/Terminal'
export interface ShellParams {
  command: string
  workingDirectory?: string
  timeout?: number
  terminalId?: number
  interactive?: boolean
  environment?: Record<string, string>
}

/**
 * Shell命令执行工具
 */
export class ShellTool extends ModifiableTool {
  private readonly dangerousCommands = [
    'rm -rf /',
    'sudo rm -rf',
    'format',
    'fdisk',
    'mkfs',
    'dd if=/dev/',
    'shutdown',
    'reboot',
    'halt',
    'poweroff',
  ]

  constructor() {
    super(
      'shell',
      '🔧 执行Shell命令：运行任意终端命令，支持工作目录切换、环境变量设置。用于npm install、git操作、系统命令等',
      {
        type: 'object',
        properties: {
          command: {
            type: 'string',
            description: '要执行的Shell命令',
          },
          workingDirectory: {
            type: 'string',
            description: '工作目录，可选',
          },
          timeout: {
            type: 'number',
            description: '命令超时时间（毫秒），默认30秒',
            default: 30000,
            minimum: 1000,
            maximum: 300000,
          },
          terminalId: {
            type: 'number',
            description: '指定终端ID，可选',
          },
          interactive: {
            type: 'boolean',
            description: '是否为交互式命令，默认false',
            default: false,
          },
          environment: {
            type: 'object',
            description: '环境变量设置',
            additionalProperties: { type: 'string' },
          },
        },
        required: ['command'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { command, workingDirectory, terminalId, environment } = context.parameters as unknown as ShellParams

    // 验证命令
    this.validateCommand(command)

    // 获取终端实例
    let targetTerminalId: number
    if (terminalId) {
      targetTerminalId = terminalId
    } else {
      targetTerminalId = await this.getOrCreateAgentTerminal()
    }

    try {
      // 构建命令
      const commandParts: string[] = []

      if (workingDirectory) {
        commandParts.push(`cd "${workingDirectory}"`)
      }

      if (environment && Object.keys(environment).length > 0) {
        const envVars = Object.entries(environment)
          .map(([key, value]) => `export ${key}="${value}"`)
          .join('; ')
        commandParts.push(envVars)
      }

      commandParts.push(command)
      const finalCommand = commandParts.length > 1 ? commandParts.join(' && ') : command

      // 执行命令
      await terminalAPI.writeToTerminal({
        paneId: targetTerminalId,
        data: `${finalCommand}\n`,
      })

      // 等待执行完成
      await this.sleep(500)

      // 获取输出
      const output = await terminalAPI.getTerminalBuffer(targetTerminalId)
      const cleanOutput = this.cleanOutput(output, finalCommand)

      return {
        content: [
          {
            type: 'text',
            text: cleanOutput || '(无输出)',
          },
        ],
      }
    } catch (error) {
      throw new TerminalError(`命令执行失败: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private validateCommand(command: string): void {
    if (!command || command.trim() === '') {
      throw new ValidationError('命令不能为空')
    }

    // 检查危险命令
    const lowerCommand = command.toLowerCase()
    for (const dangerous of this.dangerousCommands) {
      if (lowerCommand.includes(dangerous)) {
        throw new ValidationError(`检测到危险命令，已阻止执行: ${command}`)
      }
    }

    // 检查命令长度
    if (command.length > 1000) {
      throw new ValidationError('命令过长，请分解为多个较短的命令')
    }
  }

  private cleanOutput(output: string, command: string): string {
    if (!output) return ''

    const lines = output.split('\n')
    const cleanLines: string[] = []
    let foundCommand = false

    for (const line of lines) {
      const trimmed = line.trim()

      // 跳过空行
      if (!trimmed) continue

      // 跳过提示符
      if (trimmed.match(/^[$#%>]\s*$/) || trimmed.match(/.*[@#$%>:]\s*$/)) {
        continue
      }

      // 跳过包含命令的行
      if (trimmed.includes(command) && !foundCommand) {
        foundCommand = true
        continue
      }

      // 跳过Agent欢迎信息
      if (trimmed.includes('🤖') || trimmed.includes('专属终端')) {
        continue
      }

      cleanLines.push(trimmed)
    }

    return cleanLines.join('\n') || '(无输出)'
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms))
  }

  /**
   * 获取或创建Agent专属终端
   */
  private async getOrCreateAgentTerminal(): Promise<number> {
    try {
      // 首先尝试从上下文中获取Agent实例
      const agentTerminalId = await this.getAgentTerminalFromContext()
      if (agentTerminalId) {
        return agentTerminalId
      }

      const terminalStore = useTerminalStore()

      // 查找现有的Agent终端
      const agentTerminal = terminalStore.terminals.find(terminal => terminal.title === 'OrbitX')

      if (agentTerminal && agentTerminal.backendId) {
        // 激活现有的Agent终端
        terminalStore.setActiveTerminal(agentTerminal.id)
        return agentTerminal.backendId
      }

      // 创建新的Agent终端
      const agentTerminalSessionId = await terminalStore.createAgentTerminal('OrbitX')
      const newAgentTerminal = terminalStore.terminals.find(t => t.id === agentTerminalSessionId)
      if (!newAgentTerminal || !newAgentTerminal.backendId) {
        throw new TerminalError('无法创建或获取Agent专属终端')
      }

      return newAgentTerminal.backendId
    } catch (error) {
      // 降级到使用任何可用的终端
      console.warn('无法获取Agent专属终端，使用普通终端:', error)
      const terminals = await terminalAPI.listTerminals()
      if (terminals.length === 0) {
        throw new TerminalError('没有可用的终端')
      }
      return terminals[0]
    }
  }

  /**
   * 尝试从Agent上下文中获取专属终端ID
   */
  private async getAgentTerminalFromContext(): Promise<number | null> {
    // 这里可以通过某种方式获取当前Agent实例
    // 由于架构限制，暂时返回null
    // 在未来可以考虑通过context传递Agent实例
    return null
  }
}

// 导出工具实例
export const shellTool = new ShellTool()
