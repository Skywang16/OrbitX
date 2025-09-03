/**
 * Shell Integration 统一服务
 * 
 * 提供类似VSCode的Shell Integration API，让任何组件都能使用
 */

import { ref, reactive } from 'vue'
import { terminalApi } from '@/api'

export interface CommandResult {
  commandId: string
  exitCode: number
  isSuccess: boolean
  output: string
  duration: number
}

export interface ShellIntegrationEvent {
  type: 'command-started' | 'command-finished' | 'cwd-changed'
  terminalId: string
  data: any
}

export type ShellIntegrationListener = (event: ShellIntegrationEvent) => void

/**
 * 全局Shell Integration服务
 */
class ShellIntegrationService {
  private listeners = new Set<ShellIntegrationListener>()
  private terminalStates = reactive(new Map<string, any>())
  private currentCommands = reactive(new Map<string, any>())

  /**
   * 注册事件监听器
   */
  onEvent(listener: ShellIntegrationListener): () => void {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  /**
   * 触发事件
   */
  private emit(event: ShellIntegrationEvent) {
    this.listeners.forEach(listener => {
      try {
        listener(event)
      } catch (error) {
        console.error('Shell Integration listener error:', error)
      }
    })
  }

  /**
   * 执行命令并等待结果
   */
  async executeCommand(terminalId: string, command: string, timeout = 30000): Promise<CommandResult> {
    return new Promise((resolve, reject) => {
      const commandId = `cmd_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
      const startTime = Date.now()
      let outputBuffer = ''

      // 注册临时监听器
      const cleanup = this.onEvent((event) => {
        if (event.terminalId !== terminalId) return

        if (event.type === 'command-finished' && event.data.commandId === commandId) {
          cleanup()
          resolve({
            commandId,
            exitCode: event.data.exitCode,
            isSuccess: event.data.isSuccess,
            output: outputBuffer,
            duration: Date.now() - startTime
          })
        }
      })

      // 设置超时
      const timeoutId = setTimeout(() => {
        cleanup()
        reject(new Error(`Command execution timeout: ${command}`))
      }, timeout)

      // 执行命令
      this.writeToTerminal(terminalId, command + '\n')
        .catch(error => {
          cleanup()
          clearTimeout(timeoutId)
          reject(error)
        })
    })
  }

  /**
   * 获取终端的当前工作目录
   */
  getCurrentWorkingDirectory(terminalId: string): string | null {
    return this.terminalStates.get(terminalId)?.cwd || null
  }

  /**
   * 获取终端的命令历史
   */
  getCommandHistory(terminalId: string): any[] {
    return this.terminalStates.get(terminalId)?.commandHistory || []
  }

  /**
   * 处理OSC序列（由Terminal组件调用）
   */
  processOSCSequence(terminalId: string, command: string, payload: string) {
    switch (command) {
      case 'B':
        // 命令开始
        this.handleCommandStart(terminalId)
        break
      case 'C':
        // 命令执行开始
        this.handleCommandExecute(terminalId)
        break
      case 'D':
        // 命令结束
        this.handleCommandFinish(terminalId, payload)
        break
    }
  }

  private handleCommandStart(terminalId: string) {
    const commandId = `cmd_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`
    this.currentCommands.set(terminalId, { commandId, startTime: Date.now() })
    
    this.emit({
      type: 'command-started',
      terminalId,
      data: { commandId }
    })
  }

  private handleCommandExecute(terminalId: string) {
    // 如果没有当前命令，创建一个
    if (!this.currentCommands.has(terminalId)) {
      this.handleCommandStart(terminalId)
    }
  }

  private handleCommandFinish(terminalId: string, payload: string) {
    const currentCommand = this.currentCommands.get(terminalId)
    if (!currentCommand) return

    const exitCode = payload ? parseInt(payload, 10) : 0
    const isSuccess = exitCode === 0

    this.emit({
      type: 'command-finished',
      terminalId,
      data: {
        commandId: currentCommand.commandId,
        exitCode,
        isSuccess,
        duration: Date.now() - currentCommand.startTime
      }
    })

    this.currentCommands.delete(terminalId)
  }

  private async writeToTerminal(terminalId: string, data: string) {
    // 这里需要通过Terminal Store获取backendId
    // 简化实现，实际需要更完善的终端ID映射
    throw new Error('Not implemented: need terminal ID to backend ID mapping')
  }
}

// 导出单例
export const shellIntegrationService = new ShellIntegrationService()

/**
 * Vue Composable 包装器
 */
export function useShellIntegrationAPI(terminalId: string) {
  return {
    executeCommand: (command: string, timeout?: number) => 
      shellIntegrationService.executeCommand(terminalId, command, timeout),
    
    getCurrentWorkingDirectory: () => 
      shellIntegrationService.getCurrentWorkingDirectory(terminalId),
    
    getCommandHistory: () => 
      shellIntegrationService.getCommandHistory(terminalId),
    
    onEvent: (listener: ShellIntegrationListener) => 
      shellIntegrationService.onEvent(listener)
  }
}
