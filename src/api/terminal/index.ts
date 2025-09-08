/**
 * 终端管理 API
 *
 * 提供终端管理的统一接口，包括：
 * - 终端创建和管理
 * - Shell 信息获取
 * - 批量操作
 */

import { invoke } from '@/utils/request'
import type {
  CreateTerminalWithShellOptions,
  ShellInfo,
  TerminalCreateOptions,
  TerminalResizeOptions,
  TerminalWriteOptions,
  TerminalConfig,
  CursorConfig,
  TerminalConfigValidationResult,
  SystemShellsResult,
} from './types'

/**
 * 终端 API 接口类
 */
export class TerminalApi {
  // ===== 基本操作 =====

  async createTerminal(options: TerminalCreateOptions): Promise<number> {
    return await invoke<number>('create_terminal', {
      rows: options.rows,
      cols: options.cols,
      cwd: options.cwd,
    })
  }

  async createTerminalWithShell(options: CreateTerminalWithShellOptions): Promise<number> {
    return await invoke<number>('create_terminal_with_shell', {
      shellName: options.shellName,
      rows: options.rows,
      cols: options.cols,
    })
  }

  async writeToTerminal(options: TerminalWriteOptions): Promise<void> {
    await invoke<void>('write_to_terminal', { paneId: options.paneId, data: options.data })
  }

  async resizeTerminal(options: TerminalResizeOptions): Promise<void> {
    await invoke<void>('resize_terminal', {
      paneId: options.paneId,
      rows: options.rows,
      cols: options.cols,
    })
  }

  async closeTerminal(paneId: number): Promise<void> {
    await invoke<void>('close_terminal', { paneId })
  }

  async listTerminals(): Promise<number[]> {
    return await invoke<number[]>('list_terminals')
  }

  // ===== Shell 管理 =====

  async getAvailableShells(): Promise<ShellInfo[]> {
    return await invoke<ShellInfo[]>('get_available_shells')
  }

  async getDefaultShell(): Promise<ShellInfo> {
    return await invoke<ShellInfo>('get_default_shell')
  }

  async validateShellPath(path: string): Promise<boolean> {
    return await invoke<boolean>('validate_shell_path', { path })
  }

  // ===== 缓冲区操作 =====

  async getTerminalBuffer(paneId: number): Promise<string> {
    return await invoke<string>('get_terminal_buffer', { paneId })
  }

  async setTerminalBuffer(paneId: number, content: string): Promise<void> {
    await invoke<void>('set_terminal_buffer', { paneId, content })
  }

  // ===== 工具方法 =====

  async terminalExists(paneId: number): Promise<boolean> {
    const terminals = await this.listTerminals()
    return terminals.includes(paneId)
  }

  // ===== 终端配置管理 =====

  async getTerminalConfig(): Promise<TerminalConfig> {
    return await invoke<TerminalConfig>('get_terminal_config')
  }

  async updateTerminalConfig(config: TerminalConfig): Promise<void> {
    await invoke<void>('update_terminal_config', { terminalConfig: config })
  }

  async validateTerminalConfig(): Promise<TerminalConfigValidationResult> {
    return await invoke<TerminalConfigValidationResult>('validate_terminal_config')
  }

  async resetTerminalConfigToDefaults(): Promise<void> {
    await invoke('reset_terminal_config_to_defaults')
  }

  async detectSystemShells(): Promise<SystemShellsResult> {
    return await invoke<SystemShellsResult>('detect_system_shells')
  }

  async getShellInfo(shellPath: string): Promise<ShellInfo | null> {
    return await invoke<ShellInfo | null>('get_shell_info', { shellPath })
  }

  async updateCursorConfig(cursorConfig: CursorConfig): Promise<void> {
    await invoke('update_cursor_config', { cursorConfig })
  }
}

// 导出单例实例
export const terminalApi = new TerminalApi()

// 导出类型
export type * from './types'

// 默认导出
export default terminalApi
