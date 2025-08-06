/**
 * 终端管理相关的API接口
 */

import { invoke } from '@/utils/request'
import { handleError } from '../../utils/errorHandler'
import type {
  BatchTerminalResize,
  CreateTerminalWithShellOptions,
  ShellInfo,
  TerminalCreateOptions,
  TerminalOperationResult,
  TerminalResizeOptions,
  TerminalRetryOptions,
  TerminalStats,
  TerminalWriteOptions,
} from './types'

/**
 * 终端管理API
 * 提供终端的创建、管理、操作等功能
 */
export class TerminalAPI {
  /**
   * 创建新的终端会话
   */
  async createTerminal(options: TerminalCreateOptions): Promise<number> {
    try {
      return await invoke<number>('create_terminal', {
        rows: options.rows,
        cols: options.cols,
        cwd: options.cwd,
      })
    } catch (error) {
      throw new Error(handleError(error, '创建终端失败'))
    }
  }

  /**
   * 使用指定shell创建终端
   */
  async createTerminalWithShell(options: CreateTerminalWithShellOptions): Promise<number> {
    try {
      return await invoke<number>('create_terminal_with_shell', {
        shellName: options.shellName,
        rows: options.rows,
        cols: options.cols,
      })
    } catch (error) {
      throw new Error(handleError(error, '使用指定shell创建终端失败'))
    }
  }

  /**
   * 向终端写入数据
   */
  async writeToTerminal(options: TerminalWriteOptions): Promise<void> {
    try {
      return await invoke<void>('write_to_terminal', { paneId: options.paneId, data: options.data })
    } catch (error) {
      throw new Error(handleError(error, '向终端写入数据失败'))
    }
  }

  /**
   * 调整终端大小
   */
  async resizeTerminal(options: TerminalResizeOptions): Promise<void> {
    try {
      return await invoke<void>('resize_terminal', {
        paneId: options.paneId,
        rows: options.rows,
        cols: options.cols,
      })
    } catch (error) {
      throw new Error(handleError(error, '调整终端大小失败'))
    }
  }

  /**
   * 关闭终端会话
   */
  async closeTerminal(paneId: number): Promise<void> {
    try {
      return await invoke<void>('close_terminal', { paneId })
    } catch (error) {
      throw new Error(handleError(error, '关闭终端失败'))
    }
  }

  /**
   * 获取终端列表
   */
  async listTerminals(): Promise<number[]> {
    try {
      return await invoke<number[]>('list_terminals')
    } catch (error) {
      throw new Error(handleError(error, '获取终端列表失败'))
    }
  }

  /**
   * 获取可用的shell列表
   */
  async getAvailableShells(): Promise<ShellInfo[]> {
    try {
      return await invoke<ShellInfo[]>('get_available_shells')
    } catch (error) {
      throw new Error(handleError(error, '获取可用shell列表失败'))
    }
  }

  /**
   * 获取默认shell信息
   */
  async getDefaultShell(): Promise<ShellInfo> {
    try {
      return await invoke<ShellInfo>('get_default_shell')
    } catch (error) {
      throw new Error(handleError(error, '获取默认shell失败'))
    }
  }

  /**
   * 验证shell路径是否有效
   */
  async validateShellPath(path: string): Promise<boolean> {
    try {
      return await invoke<boolean>('validate_shell_path', { path })
    } catch (error) {
      console.warn('验证shell路径失败:', handleError(error))
      return false
    }
  }

  /**
   * 批量创建终端
   */
  async createMultipleTerminals(terminals: TerminalCreateOptions[]): Promise<number[]> {
    try {
      const results: number[] = []
      for (const terminal of terminals) {
        const paneId = await this.createTerminal(terminal)
        results.push(paneId)
      }
      return results
    } catch (error) {
      throw new Error(handleError(error, '批量创建终端失败'))
    }
  }

  /**
   * 批量关闭终端
   */
  async closeMultipleTerminals(paneIds: number[]): Promise<void[]> {
    try {
      const results: void[] = []
      for (const paneId of paneIds) {
        await this.closeTerminal(paneId)
        results.push()
      }
      return results
    } catch (error) {
      throw new Error(handleError(error, '批量关闭终端失败'))
    }
  }

  /**
   * 向多个终端写入相同数据
   */
  async writeToMultipleTerminals(paneIds: number[], data: string): Promise<void[]> {
    try {
      const results: void[] = []
      for (const paneId of paneIds) {
        await this.writeToTerminal({ paneId, data })
        results.push()
      }
      return results
    } catch (error) {
      throw new Error(handleError(error, '向多个终端写入数据失败'))
    }
  }

  /**
   * 批量调整终端大小
   */
  async resizeMultipleTerminals(terminals: BatchTerminalResize[]): Promise<void[]> {
    try {
      const results: void[] = []
      for (const terminal of terminals) {
        await this.resizeTerminal(terminal)
        results.push()
      }
      return results
    } catch (error) {
      throw new Error(handleError(error, '批量调整终端大小失败'))
    }
  }

  /**
   * 带重试的终端创建
   */
  async createTerminalWithRetry(options: TerminalCreateOptions, retryOptions?: TerminalRetryOptions): Promise<number> {
    const maxRetries = retryOptions?.retries || 3
    const retryDelay = retryOptions?.retryDelay || 1000

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      try {
        return await this.createTerminal(options)
      } catch (error) {
        if (attempt === maxRetries) {
          throw new Error(handleError(error, '创建终端失败（重试后）'))
        }
        await new Promise(resolve => setTimeout(resolve, retryDelay))
      }
    }
    throw new Error('创建终端失败（重试后）')
  }

  /**
   * 安全的终端写入（带错误处理）
   */
  async safeWriteToTerminal(options: TerminalWriteOptions): Promise<TerminalOperationResult> {
    try {
      await this.writeToTerminal(options)
      return { success: true }
    } catch (error) {
      return {
        success: false,
        error: handleError(error, '终端写入失败'),
      }
    }
  }

  /**
   * 检查终端是否存在
   */
  async terminalExists(paneId: number): Promise<boolean> {
    try {
      const terminals = await this.listTerminals()
      return terminals.includes(paneId)
    } catch (error) {
      console.warn('检查终端存在性失败:', handleError(error))
      return false
    }
  }

  /**
   * 获取终端统计信息
   */
  async getTerminalStats(): Promise<TerminalStats> {
    try {
      const terminals = await this.listTerminals()
      return {
        total: terminals.length,
        active: terminals.length, // 假设所有列出的终端都是活跃的
        ids: terminals,
      }
    } catch (error) {
      console.warn('获取终端统计失败:', handleError(error))
      return {
        total: 0,
        active: 0,
        ids: [],
      }
    }
  }

  /**
   * 获取终端缓冲区内容
   */
  async getTerminalBuffer(paneId: number): Promise<string> {
    try {
      return await invoke<string>('get_terminal_buffer', { paneId })
    } catch (error) {
      console.warn('获取终端缓冲区失败:', handleError(error))
      return ''
    }
  }

  /**
   * 设置终端缓冲区内容
   */
  async setTerminalBuffer(paneId: number, content: string): Promise<void> {
    try {
      return await invoke<void>('set_terminal_buffer', { paneId, content })
    } catch (error) {
      throw new Error(handleError(error, '设置终端缓冲区失败'))
    }
  }
}

/**
 * 终端API实例
 */
export const terminalAPI = new TerminalAPI()

/**
 * 便捷的终端操作函数
 */
export const terminal = {
  // 基本操作
  create: (options: TerminalCreateOptions) => terminalAPI.createTerminal(options),
  createWithShell: (options: CreateTerminalWithShellOptions) => terminalAPI.createTerminalWithShell(options),
  write: (options: TerminalWriteOptions) => terminalAPI.writeToTerminal(options),
  resize: (options: TerminalResizeOptions) => terminalAPI.resizeTerminal(options),
  close: (paneId: number) => terminalAPI.closeTerminal(paneId),
  list: () => terminalAPI.listTerminals(),

  // Shell相关
  getShells: () => terminalAPI.getAvailableShells(),
  getDefaultShell: () => terminalAPI.getDefaultShell(),
  validateShell: (path: string) => terminalAPI.validateShellPath(path),

  // 批量操作
  createMultiple: (terminals: TerminalCreateOptions[]) => terminalAPI.createMultipleTerminals(terminals),
  closeMultiple: (paneIds: number[]) => terminalAPI.closeMultipleTerminals(paneIds),
  writeToMultiple: (paneIds: number[], data: string) => terminalAPI.writeToMultipleTerminals(paneIds, data),
  resizeMultiple: (terminals: BatchTerminalResize[]) => terminalAPI.resizeMultipleTerminals(terminals),

  // 高级功能
  createWithRetry: (options: TerminalCreateOptions, retryOptions?: TerminalRetryOptions) =>
    terminalAPI.createTerminalWithRetry(options, retryOptions),
  safeWrite: (options: TerminalWriteOptions) => terminalAPI.safeWriteToTerminal(options),
  exists: (paneId: number) => terminalAPI.terminalExists(paneId),
  getStats: () => terminalAPI.getTerminalStats(),

  // 缓冲区操作
  getBuffer: (paneId: number) => terminalAPI.getTerminalBuffer(paneId),
  setBuffer: (paneId: number, content: string) => terminalAPI.setTerminalBuffer(paneId, content),
}

// 类型定义现在统一从 @/types 导入，不在此处重复导出
