/**
 * Shell命令执行工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '@eko-ai/eko/types'
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
      `在当前终端中执行Shell命令。适用于系统操作、构建部署、版本控制等场景。包含安全检查，会阻止危险命令。注意：代码搜索请使用orbit_search工具，文件内容查找请使用orbit_search或read_file工具。`,
      {
        type: 'object',
        properties: {
          command: {
            type: 'string',
            description: '要执行的命令。示例："ls -la"、"npm install"、"git status"',
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
      throw new ToolError(`命令执行失败: ${error instanceof Error ? error.message : String(error)}`)
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
      throw new ToolError('找不到对应的终端会话')
    }

    return new Promise<string>((resolve, reject) => {
      let outputBuffer = ''
      let timeoutId: NodeJS.Timeout
      let isCompleted = false

      // 绑定 cleanOutput 方法
      const cleanOutputFn = this.cleanOutput.bind(this)

      // 设置超时
      timeoutId = setTimeout(() => {
        if (!isCompleted) {
          isCompleted = true
          cleanup()
          reject(new ToolError(`命令执行超时 (${timeout}ms): ${command}`))
        }
      }, timeout)

      // 命令完成检测逻辑
      const detectCommandCompletion = (output: string): boolean => {
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
          outputBuffer += data

          // 检测命令是否完成（出现新的提示符）
          // 同时检测当前数据块和整个缓冲区
          const isCompleteInData = detectCommandCompletion(data)
          const isCompleteInBuffer = detectCommandCompletion(outputBuffer)
          const isComplete = isCompleteInData || isCompleteInBuffer

          if (isComplete && !isCompleted) {
            isCompleted = true
            cleanup()

            // 清理输出并返回
            const cleanOutput = cleanOutputFn(outputBuffer, command)
            resolve(cleanOutput)
          }
        },
        onExit: (exitCode: number | null) => {
          if (!isCompleted) {
            isCompleted = true
            cleanup()

            if (exitCode === 0) {
              const cleanOutput = cleanOutputFn(outputBuffer, command)
              resolve(cleanOutput)
            } else {
              reject(new ToolError(`命令执行失败，退出码: ${exitCode}`))
            }
          }
        },
      }

      // 注册监听器
      terminalStore.registerTerminalCallbacks(terminalSession.id, callbacks)

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
            reject(new ToolError(`写入命令失败: ${error.message}`))
          }
        })
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
