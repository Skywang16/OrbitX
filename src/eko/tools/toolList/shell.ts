/**
 * Shell命令执行工具
 */

import { ModifiableTool, type ToolExecutionContext } from '../modifiable-tool'
import type { ToolResult } from '../../types'
import { TerminalError, ValidationError } from '../tool-error'
import { terminalApi } from '@/api'
import { useTerminalStore } from '@/stores/Terminal'
import { TerminalAgent } from '../../agent/terminal-agent'
export interface ShellParams {
  command: string
  workingDirectory?: string
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
    super('shell', '执行命令：在终端中运行指定的命令', {
      type: 'object',
      properties: {
        command: {
          type: 'string',
          description: '要执行的命令',
        },
        workingDirectory: {
          type: 'string',
          description: '工作目录（可选）',
        },
      },
      required: ['command'],
    })
  }

  protected async executeImpl(context: ToolExecutionContext): Promise<ToolResult> {
    const { command, workingDirectory } = context.parameters as unknown as ShellParams

    // 验证命令
    this.validateCommand(command)

    // 获取终端实例
    const targetTerminalId = await this.getOrCreateAgentTerminal()

    try {
      // 构建命令
      const finalCommand = workingDirectory ? `cd "${workingDirectory}" && ${command}` : command

      // 使用事件驱动的方式等待命令完成
      const result = await this.executeCommandWithCallback(targetTerminalId, finalCommand, 30000)
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

      // 绑定 cleanOutput 方法
      const cleanOutputFn = this.cleanOutput.bind(this)

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
        // 彻底清理ANSI转义序列
        let cleanOutput = output
          .replace(/\u001B\[[0-9;?]*[a-zA-Z]/g, '') // 标准ANSI序列
          .replace(/\u001B\[[?][0-9]*[a-zA-Z]/g, '') // ?开头的序列
          .replace(/\u001B\[K/g, '') // 清除行序列
          .replace(/\u001B\[[0-9]*[mK]/g, '') // m和K结尾的序列
          .replace(/\r/g, '') // 回车符
          .replace(/\n+/g, ' ') // 换行符转空格
          .replace(/\s+/g, ' ') // 多个空格合并
          .trim()

        // 检测提示符：包含 @ 和 % 的提示符，或以常见提示符结尾
        const hasUserHostPrompt = cleanOutput.includes('@') && cleanOutput.includes(' % ')
        const hasSimplePrompt = /[%$#>]\s*$/.test(cleanOutput)

        return hasUserHostPrompt || hasSimplePrompt
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
              reject(new TerminalError(`命令执行失败，退出码: ${exitCode}`))
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
    const terminals = await terminalApi.listTerminals()
    if (terminals.length === 0) {
      throw new TerminalError('没有可用的终端')
    }
    return terminals[0]
  }
}

// 导出工具实例
export const shellTool = new ShellTool()
