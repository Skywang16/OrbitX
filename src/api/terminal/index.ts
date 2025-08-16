/**
 * 终端管理 API
 *
 * 提供终端管理的统一接口，包括：
 * - 终端创建和管理
 * - Shell 信息获取
 * - 批量操作
 */

import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'
import type {
  CreateTerminalWithShellOptions,
  ShellInfo,
  TerminalCreateOptions,
  TerminalResizeOptions,
  TerminalWriteOptions,
} from './types'

/**
 * 终端 API 接口类
 */
export class TerminalApi {
  // ===== 基本操作 =====

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

  async writeToTerminal(options: TerminalWriteOptions): Promise<void> {
    try {
      await invoke('write_to_terminal', { paneId: options.paneId, data: options.data })
    } catch (error) {
      throw new Error(handleError(error, '向终端写入数据失败'))
    }
  }

  async resizeTerminal(options: TerminalResizeOptions): Promise<void> {
    try {
      await invoke('resize_terminal', {
        paneId: options.paneId,
        rows: options.rows,
        cols: options.cols,
      })
    } catch (error) {
      throw new Error(handleError(error, '调整终端大小失败'))
    }
  }

  async closeTerminal(paneId: number): Promise<void> {
    try {
      await invoke('close_terminal', { paneId })
    } catch (error) {
      throw new Error(handleError(error, '关闭终端失败'))
    }
  }

  async listTerminals(): Promise<number[]> {
    try {
      return await invoke<number[]>('list_terminals')
    } catch (error) {
      throw new Error(handleError(error, '获取终端列表失败'))
    }
  }

  // ===== Shell 管理 =====

  async getAvailableShells(): Promise<ShellInfo[]> {
    try {
      return await invoke<ShellInfo[]>('get_available_shells')
    } catch (error) {
      throw new Error(handleError(error, '获取可用shell列表失败'))
    }
  }

  async getDefaultShell(): Promise<ShellInfo> {
    try {
      return await invoke<ShellInfo>('get_default_shell')
    } catch (error) {
      throw new Error(handleError(error, '获取默认shell失败'))
    }
  }

  async validateShellPath(path: string): Promise<boolean> {
    try {
      return await invoke<boolean>('validate_shell_path', { path })
    } catch (error) {
      console.warn('验证shell路径失败:', handleError(error))
      return false
    }
  }

  // ===== 缓冲区操作 =====

  async getTerminalBuffer(paneId: number): Promise<string> {
    try {
      return await invoke<string>('get_terminal_buffer', { paneId })
    } catch (error) {
      console.warn('获取终端缓冲区失败:', handleError(error))
      return ''
    }
  }

  async setTerminalBuffer(paneId: number, content: string): Promise<void> {
    try {
      await invoke('set_terminal_buffer', { paneId, content })
    } catch (error) {
      throw new Error(handleError(error, '设置终端缓冲区失败'))
    }
  }

  // ===== 工具方法 =====

  async terminalExists(paneId: number): Promise<boolean> {
    try {
      const terminals = await this.listTerminals()
      return terminals.includes(paneId)
    } catch (error) {
      console.warn('检查终端存在性失败:', handleError(error))
      return false
    }
  }
}

// 导出单例实例
export const terminalApi = new TerminalApi()

// 导出类型
export type * from './types'

// 默认导出
export default terminalApi
