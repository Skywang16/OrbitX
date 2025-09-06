/**
 * Shell命令执行工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@/eko-core/types'
import { ValidationError, ToolError } from '../tool-error'
import { terminalApi } from '@/api'
import { useTerminalStore } from '@/stores/Terminal'

import stripAnsi from 'strip-ansi'
export interface ShellParams {
  command: string
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
      `Execute Shell commands in the current terminal. Suitable for system operations, build deployment, version control, and other scenarios. Includes security checks that will block dangerous commands. Note: For code search, use orbit_search or grep_search tools; for file content lookup, use orbit_search, grep_search, or read_file tools.`,
      {
        type: 'object',
        properties: {
          command: {
            type: 'string',
            description: 'Command to execute. Examples: "ls -la", "npm install", "git status"',
          },
        },
        required: ['command'],
      }
    )
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { command } = context.parameters as unknown as ShellParams

    // 验证命令
    this.validateCommand(command)

    // 获取当前活跃的终端ID
    const targetTerminalId = await this.getActiveTerminal()

    try {
      // 使用事件驱动的方式等待命令完成
      const result = await this.executeCommandWithCallback(targetTerminalId, command, 30000)
      return {
        content: [
          {
            type: 'text',
            text: result || '(无输出)',
          },
        ],
      }
    } catch (error) {
      throw new ToolError(`Command execution failed: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  private validateCommand(command: string): void {
    if (!command || command.trim() === '') {
      throw new ValidationError('命令不能为空')
    }

    const lowerCommand = command.toLowerCase().trim()

    // 检查危险命令
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

    // 先清理ANSI序列
    const cleanedOutput = stripAnsi(output)
    const lines = cleanedOutput.split('\n')
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

      cleanLines.push(trimmed)
    }

    return cleanLines.join('\n') || '(无输出)'
  }

  /**
   * 基于Shell Integration的命令执行 - 使用OSC 133序列检测命令完成
   */
  private async executeCommandWithCallback(terminalId: number, command: string, timeout: number): Promise<string> {
    const terminalStore = useTerminalStore()

    // 找到对应的终端会话
    const terminalSession = terminalStore.terminals.find(t => t.backendId === terminalId)
    if (!terminalSession) {
      throw new ToolError('找不到对应的终端会话')
    }

    return new Promise<string>((resolve, reject) => {
      let outputBuffer = ''
      let timeoutId: NodeJS.Timeout
      let isCompleted = false

      // 绑定 cleanOutput 方法
      const cleanOutputFn = this.cleanOutput.bind(this)

      // Shell Integration状态跟踪
      let shellIntegrationActive = false

      // 增强的命令完成检测 - 优先使用Shell Integration，回退到正则检测
      const detectCommandCompletion = (output: string): boolean => {
        // 如果Shell Integration激活，依赖其状态
        if (shellIntegrationActive) {
          return false // Shell Integration会通过回调通知完成
        }

        // 回退到传统的正则检测（兼容性）
        if (!output || output.trim() === '') return false

        // 去除 ANSI 转义序列与回车符
        const cleanOutput = stripAnsi(output).replace(/\r/g, '')

        // 按行分割，检查最后几行
        const lines = cleanOutput.split('\n').filter(line => line.trim() !== '')
        if (lines.length === 0) return false

        const lastLine = lines[lines.length - 1].trim()

        // 检测各种提示符模式
        const promptPatterns = [
          /.*[@#$%>]\s*$/, // 通用提示符结尾
          /.*\$\s*$/, // bash提示符
          /.*%\s*$/, // zsh提示符
          /.*#\s*$/, // root提示符
          /.*>\s*$/, // cmd提示符
          /.*@.*:\s*[~/].*[$%#>]\s*$/, // 完整的用户@主机:路径$ 格式
        ]

        return promptPatterns.some(pattern => pattern.test(lastLine))
      }

      // 设置超时 - 智能回退机制
      timeoutId = setTimeout(() => {
        if (!isCompleted) {
          // 如果Shell Integration激活但没有完成，尝试传统检测作为回退
          if (shellIntegrationActive && outputBuffer) {
            const isComplete = detectCommandCompletion(outputBuffer)
            if (isComplete) {
              isCompleted = true
              cleanup()
              const cleanOutput = cleanOutputFn(outputBuffer, command)
              resolve(cleanOutput)
              return
            }
          }

          isCompleted = true
          cleanup()
          reject(new ToolError(`命令执行超时 (${timeout}ms): ${command}`))
        }
      }, timeout)

      // 订阅命令事件
      const unsubscribe = terminalStore.subscribeToCommandEvents((terminalId, event, data) => {
        // 只处理当前终端的事件
        if (terminalId !== terminalSession.id) return

        if (event === 'started') {
          if (process.env.NODE_ENV === 'development') {
            console.warn(`Shell Tool: Command started via event - ${data?.commandId}`)
          }
        } else if (event === 'finished') {
          if (process.env.NODE_ENV === 'development') {
            console.warn(
              `Shell Tool: Command finished via event - exitCode=${data?.exitCode}, isSuccess=${data?.isSuccess}`
            )
          }

          if (!isCompleted) {
            isCompleted = true
            cleanup()

            if (data?.isSuccess) {
              const cleanOutput = cleanOutputFn(outputBuffer, command)
              resolve(cleanOutput)
            } else {
              reject(new ToolError(`Command execution failed with exit code: ${data?.exitCode}`))
            }
          }
        }
      })

      // 清理函数
      const cleanup = () => {
        if (timeoutId) {
          clearTimeout(timeoutId)
        }
        terminalStore.unregisterTerminalCallbacks(terminalSession.id, callbacks)
        unsubscribe() // 取消事件订阅
      }

      // 终端输出监听回调
      const callbacks = {
        onOutput: (data: string) => {
          outputBuffer += data

          // Shell Integration通过事件系统处理，这里不需要处理OSC序列

          // 检测OSC 133序列以激活Shell Integration
          if (data.includes('\x1b]133;')) {
            shellIntegrationActive = true
            if (process.env.NODE_ENV === 'development') {
              console.warn('Shell Tool: Shell Integration mode activated')
            }
          }

          // 只有在Shell Integration未激活时才使用传统检测
          if (!shellIntegrationActive) {
            const isComplete = detectCommandCompletion(data) || detectCommandCompletion(outputBuffer)

            if (isComplete && !isCompleted) {
              isCompleted = true
              cleanup()

              if (process.env.NODE_ENV === 'development') {
                console.warn('Shell Tool: Command completed via traditional detection')
              }

              // 清理输出并返回
              const cleanOutput = cleanOutputFn(outputBuffer, command)
              resolve(cleanOutput)
            }
          }
        },
        onExit: (exitCode: number | null) => {
          // 如果Shell Integration已经处理了命令完成，不要重复处理
          if (!isCompleted && !shellIntegrationActive) {
            isCompleted = true
            cleanup()

            if (process.env.NODE_ENV === 'development') {
              console.warn(`Shell Tool: Command completed via onExit with code: ${exitCode}`)
            }

            if (exitCode === 0) {
              const cleanOutput = cleanOutputFn(outputBuffer, command)
              resolve(cleanOutput)
            } else {
              reject(new ToolError(`Command execution failed with exit code: ${exitCode}`))
            }
          }
        },
      }

      // 注册监听器
      terminalStore.registerTerminalCallbacks(terminalSession.id, callbacks)

      // 确保Shell Integration准备就绪，然后执行命令
      setTimeout(() => {
        // 执行命令
        terminalApi
          .writeToTerminal({
            paneId: terminalId,
            data: `${command}\n`,
          })
          .catch(error => {
            if (!isCompleted) {
              isCompleted = true
              cleanup()
              reject(new ToolError(`Failed to write command: ${error.message}`))
            }
          })
      }, 100)
    })
  }

  /**
   * 获取当前活跃的终端
   */
  private async getActiveTerminal(): Promise<number> {
    const terminalStore = useTerminalStore()

    // 获取当前活跃的终端
    const activeTerminal = terminalStore.terminals.find(t => t.id === terminalStore.activeTerminalId)

    if (!activeTerminal || !activeTerminal.backendId) {
      throw new ToolError('没有可用的活跃终端')
    }

    return activeTerminal.backendId
  }
}

// 导出工具实例
export const shellTool = new ShellTool()
