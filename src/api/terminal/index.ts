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

  createTerminal = async (options: TerminalCreateOptions): Promise<number> => {
    return await invoke<number>('terminal_create', {
      rows: options.rows,
      cols: options.cols,
      cwd: options.cwd,
    })
  }

  createTerminalWithShell = async (options: CreateTerminalWithShellOptions): Promise<number> => {
    return await invoke<number>('terminal_create_with_shell', {
      shellName: options.shellName,
      rows: options.rows,
      cols: options.cols,
    })
  }

  writeToTerminal = async (options: TerminalWriteOptions): Promise<void> => {
    await invoke<void>('terminal_write', { paneId: options.paneId, data: options.data })
  }

  resizeTerminal = async (options: TerminalResizeOptions): Promise<void> => {
    await invoke<void>('terminal_resize', {
      paneId: options.paneId,
      rows: options.rows,
      cols: options.cols,
    })
  }

  closeTerminal = async (paneId: number): Promise<void> => {
    await invoke<void>('terminal_close', { paneId })
  }

  listTerminals = async (): Promise<number[]> => {
    return await invoke<number[]>('terminal_list')
  }

  // ===== Shell 管理 =====

  getAvailableShells = async (): Promise<ShellInfo[]> => {
    return await invoke<ShellInfo[]>('terminal_get_available_shells')
  }

  getDefaultShell = async (): Promise<ShellInfo> => {
    return await invoke<ShellInfo>('terminal_get_default_shell')
  }

  validateShellPath = async (path: string): Promise<boolean> => {
    return await invoke<boolean>('terminal_validate_shell_path', { path })
  }

  // ===== 工具方法 =====

  terminalExists = async (paneId: number): Promise<boolean> => {
    const terminals = await this.listTerminals()
    return terminals.includes(paneId)
  }

  // ===== 终端配置管理 =====

  getTerminalConfig = async (): Promise<TerminalConfig> => {
    return await invoke<TerminalConfig>('config_terminal_get')
  }

  updateTerminalConfig = async (config: TerminalConfig): Promise<void> => {
    await invoke<void>('config_terminal_update', { terminalConfig: config })
  }

  validateTerminalConfig = async (): Promise<TerminalConfigValidationResult> => {
    return await invoke<TerminalConfigValidationResult>('config_terminal_validate')
  }

  resetTerminalConfigToDefaults = async (): Promise<void> => {
    await invoke('config_terminal_reset_to_defaults')
  }

  detectSystemShells = async (): Promise<SystemShellsResult> => {
    return await invoke<SystemShellsResult>('config_terminal_detect_system_shells')
  }

  getShellInfo = async (shellPath: string): Promise<ShellInfo | null> => {
    return await invoke<ShellInfo | null>('config_terminal_get_shell_info', { shellPath })
  }

  updateCursorConfig = async (cursorConfig: CursorConfig): Promise<void> => {
    await invoke('config_terminal_update_cursor', { cursorConfig })
  }

  // ===== 事件监听 =====

  /**
   * 监听终端退出事件
   */
  onTerminalExit = async (
    callback: (payload: { paneId: number; exitCode: number | null }) => void
  ): Promise<UnlistenFn> => {
    return listen<{ paneId: number; exitCode: number | null }>('terminal_exit', event => callback(event.payload))
  }

  /**
   * 监听 CWD 变化事件
   */
  onCwdChanged = async (callback: (payload: { paneId: number; cwd: string }) => void): Promise<UnlistenFn> => {
    return listen<{ paneId: number; cwd: string }>('pane_cwd_changed', event => callback(event.payload))
  }
}

export const terminalApi = new TerminalApi()
export type * from './types'
export default terminalApi
