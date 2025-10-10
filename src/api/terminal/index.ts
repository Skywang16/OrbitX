/**
 * 终端管理 API
 *
 * 提供终端管理的统一接口，包括：
 * - 终端创建和管理
 * - Shell 信息获取
 * - 批量操作
 */

import { invoke } from '@/utils/request'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
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
    return await invoke<number>('terminal_create', {
      rows: options.rows,
      cols: options.cols,
      cwd: options.cwd,
    })
  }

  async createTerminalWithShell(options: CreateTerminalWithShellOptions): Promise<number> {
    return await invoke<number>('terminal_create_with_shell', {
      shellName: options.shellName,
      rows: options.rows,
      cols: options.cols,
    })
  }

  async writeToTerminal(options: TerminalWriteOptions): Promise<void> {
    await invoke<void>('terminal_write', { paneId: options.paneId, data: options.data })
  }

  async resizeTerminal(options: TerminalResizeOptions): Promise<void> {
    await invoke<void>('terminal_resize', {
      paneId: options.paneId,
      rows: options.rows,
      cols: options.cols,
    })
  }

  async closeTerminal(paneId: number): Promise<void> {
    await invoke<void>('terminal_close', { paneId })
  }

  async listTerminals(): Promise<number[]> {
    return await invoke<number[]>('terminal_list')
  }

  // ===== Shell 管理 =====

  async getAvailableShells(): Promise<ShellInfo[]> {
    return await invoke<ShellInfo[]>('terminal_get_available_shells')
  }

  async getDefaultShell(): Promise<ShellInfo> {
    return await invoke<ShellInfo>('terminal_get_default_shell')
  }

  async validateShellPath(path: string): Promise<boolean> {
    return await invoke<boolean>('terminal_validate_shell_path', { path })
  }

  // ===== 工具方法 =====

  async terminalExists(paneId: number): Promise<boolean> {
    const terminals = await this.listTerminals()
    return terminals.includes(paneId)
  }

  // ===== 终端配置管理 =====

  async getTerminalConfig(): Promise<TerminalConfig> {
    return await invoke<TerminalConfig>('config_terminal_get')
  }

  async updateTerminalConfig(config: TerminalConfig): Promise<void> {
    await invoke<void>('config_terminal_update', { terminalConfig: config })
  }

  async validateTerminalConfig(): Promise<TerminalConfigValidationResult> {
    return await invoke<TerminalConfigValidationResult>('config_terminal_validate')
  }

  async resetTerminalConfigToDefaults(): Promise<void> {
    await invoke('config_terminal_reset_to_defaults')
  }

  async detectSystemShells(): Promise<SystemShellsResult> {
    return await invoke<SystemShellsResult>('config_terminal_detect_system_shells')
  }

  async getShellInfo(shellPath: string): Promise<ShellInfo | null> {
    return await invoke<ShellInfo | null>('config_terminal_get_shell_info', { shellPath })
  }

  async updateCursorConfig(cursorConfig: CursorConfig): Promise<void> {
    await invoke('config_terminal_update_cursor', { cursorConfig })
  }

  // ===== 事件监听 =====

  /**
   * 监听终端退出事件
   */
  async onTerminalExit(callback: (payload: { paneId: number; exitCode: number | null }) => void): Promise<UnlistenFn> {
    return listen<{ paneId: number; exitCode: number | null }>('terminal_exit', event => callback(event.payload))
  }

  /**
   * 监听 CWD 变化事件
   */
  async onCwdChanged(callback: (payload: { paneId: number; cwd: string }) => void): Promise<UnlistenFn> {
    return listen<{ paneId: number; cwd: string }>('pane_cwd_changed', event => callback(event.payload))
  }
}

export const terminalApi = new TerminalApi()
export type * from './types'
export default terminalApi
