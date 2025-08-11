/**
 * Shell命令执行工具
 */

import { ModifiableTool, type ToolExecutionContext } from './modifiable-tool'
import type { ToolResult } from '../types'
import { TerminalError, ValidationError } from './tool-error'
import { terminalAPI } from '@/api/terminal'
import { useTerminalStore } from '@/stores/Terminal'
import { TerminalAgent } from '../agent/terminal-agent'
export interface ShellParams {
  command: string
  workingDirectory?: string
  timeout?: number
  terminalId?: number
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
    const {
      command,
      workingDirectory,
      terminalId,
      environment,
      timeout = 30000,
    } = context.parameters as unknown as ShellParams

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

      // 使用事件驱动的方式等待命令完成
      const result = await this.executeCommandWithCallback(targetTerminalId, finalCommand, timeout)
      return {
        content: [
          {
            type: 'text',
            text: result || '(无输出)',
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

  /**
   * 基于事件驱动的命令执行
   */
  private async executeCommandWithCallback(terminalId: number, command: string, timeout: number): Promise<string> {
    const terminalStore = useTerminalStore()

    // 找到对应的终端会话
    const terminalSession = terminalStore.terminals.find(t => t.backendId === terminalId)
    if (!terminalSession) {
      throw new TerminalError('找不到对应的终端会话')
    }

    return new Promise<string>((resolve, reject) => {
      let outputBuffer = ''
      let timeoutId: NodeJS.Timeout
      let isCompleted = false

      // 设置超时
      timeoutId = setTimeout(() => {
        if (!isCompleted) {
          isCompleted = true
          cleanup()
          reject(new TerminalError(`命令执行超时 (${timeout}ms): ${command}`))
        }
      }, timeout)

      // 命令完成检测逻辑
      const detectCommandCompletion = (output: string): boolean => {
        // 检测常见的shell提示符
        const promptPatterns = [
          /[#%>]\s*$/, // 基本提示符 (移除了$的转义)
          /.*[@#%>:]\s*$/, // 复杂提示符 (移除了$的转义)
          /\w+@\w+.*[#]\s*$/, // user@hostname# 格式 (移除了$的转义)
          /.*\$\s*$/, // $ 提示符 (单独处理)
        ]

        return promptPatterns.some(pattern => pattern.test(output.trim()))
      }

      // 清理函数
      const cleanup = () => {
        if (timeoutId) {
          clearTimeout(timeoutId)
        }
        terminalStore.unregisterTerminalCallbacks(terminalSession.id, callbacks)
      }

      // 终端输出监听回调
      const callbacks = {
        onOutput: (data: string) => {
          console.log(`🔧 Shell Tool - 收到终端输出:`, JSON.stringify(data))
          outputBuffer += data
          console.log(`🔧 Shell Tool - 当前缓冲区:`, JSON.stringify(outputBuffer))

          // 检测命令是否完成（出现新的提示符）
          const isComplete = detectCommandCompletion(data)
          console.log(`🔧 Shell Tool - 命令完成检测:`, isComplete)

          if (isComplete) {
            if (!isCompleted) {
              console.log(`🔧 Shell Tool - 命令执行完成，开始清理输出`)
              isCompleted = true
              cleanup()

              // 清理输出并返回
              const cleanOutput = this.cleanOutput(outputBuffer, command)
              console.log(`🔧 Shell Tool - 清理后的输出:`, JSON.stringify(cleanOutput))
              resolve(cleanOutput)
            }
          }
        },
        onExit: (exitCode: number | null) => {
          if (!isCompleted) {
            isCompleted = true
            cleanup()

            if (exitCode === 0) {
              const cleanOutput = this.cleanOutput(outputBuffer, command)
              resolve(cleanOutput)
            } else {
              reject(new TerminalError(`命令执行失败，退出码: ${exitCode}`))
            }
          }
        },
      }

      // 注册监听器
      console.log(`🔧 Shell Tool - 注册终端监听器, terminalSession.id: ${terminalSession.id}`)
      terminalStore.registerTerminalCallbacks(terminalSession.id, callbacks)

      // 执行命令
      console.log(`🔧 Shell Tool - 执行命令: ${command}, terminalId: ${terminalId}`)
      terminalAPI
        .writeToTerminal({
          paneId: terminalId,
          data: `${command}\n`,
        })
        .then(() => {
          console.log(`🔧 Shell Tool - 命令写入成功`)
        })
        .catch(error => {
          console.error(`🔧 Shell Tool - 命令写入失败:`, error)
          if (!isCompleted) {
            isCompleted = true
            cleanup()
            reject(new TerminalError(`写入命令失败: ${error.message}`))
          }
        })
    })
  }

  /**
   * 获取或创建Agent专属终端
   */
  private async getOrCreateAgentTerminal(): Promise<number> {
    // 尝试从当前活跃的Agent实例获取专属终端
    const currentAgent = TerminalAgent.getCurrentInstance()
    if (currentAgent) {
      const agentTerminalId = currentAgent.getTerminalIdForTools()
      if (agentTerminalId) {
        return agentTerminalId
      }
      // 如果Agent存在但没有终端，让Agent创建一个
      return await currentAgent.ensureAgentTerminal()
    }

    // 降级方案：如果没有Agent实例，使用任何可用的终端
    const terminals = await terminalAPI.listTerminals()
    if (terminals.length === 0) {
      throw new TerminalError('没有可用的终端')
    }
    return terminals[0]
  }
}

// 导出工具实例
export const shellTool = new ShellTool()
