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
  CommandEventPayload,
  CreateTerminalWithShellOptions,
  ShellInfo,
  TerminalCreateOptions,
  TerminalResizeOptions,
  TerminalWriteOptions,
  TerminalConfig,
  TerminalConfigValidationResult,
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
    return await invoke<TerminalConfig>('terminal_config_get')
  }

  setTerminalConfig = async (config: TerminalConfig): Promise<void> => {
    await invoke<void>('terminal_config_set', { terminalConfig: config })
  }

  validateTerminalConfig = async (): Promise<TerminalConfigValidationResult> => {
    return await invoke<TerminalConfigValidationResult>('terminal_config_validate')
  }

  resetTerminalConfigToDefaults = async (): Promise<void> => {
    await invoke('terminal_config_reset_to_defaults')
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

  /**
   * 监听窗口标题变化事件 (OSC 0/1/2)
   */
  onTitleChanged = async (callback: (payload: { paneId: number; title: string }) => void): Promise<UnlistenFn> => {
    return listen<{ paneId: number; title: string }>('pane_title_changed', event => callback(event.payload))
  }

  /**
   * 监听命令事件（命令开始、执行、完成等）
   */
  onCommandEvent = async (
    callback: (payload: { paneId: number; command: CommandEventPayload }) => void
  ): Promise<UnlistenFn> => {
    return listen<{ paneId: number; command: CommandEventPayload }>('pane_command_event', event =>
      callback(event.payload)
    )
  }
}

export const terminalApi = new TerminalApi()
export type * from './types'
export default terminalApi
