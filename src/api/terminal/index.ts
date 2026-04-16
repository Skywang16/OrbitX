/**
 * Terminal management API
 *
 * Provides unified interface for terminal management, including:
 * - Terminal creation and management
 * - Shell information retrieval
 * - Batch operations
 */

import { invoke } from '@/utils/request'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  CommandEventPayload,
  CreateTerminalWithShellOptions,
  ShellInfo,
  TerminalConfig,
  TerminalConfigValidationResult,
  TerminalCreateOptions,
  TerminalResizeOptions,
  TerminalWriteOptions,
} from './types'

/**
 * Terminal API interface class
 */
export class TerminalApi {
  // ===== Basic operations =====

  createTerminal = async (options: TerminalCreateOptions): Promise<number> => {
    return await invoke<number>('terminal_create', {
      rows: options.rows,
      cols: options.cols,
      cwd: options.cwd,
      threadId: options.threadId,
    })
  }

  createTerminalWithShell = async (options: CreateTerminalWithShellOptions): Promise<number> => {
    return await invoke<number>('terminal_create_with_shell', {
      shellName: options.shellName,
      rows: options.rows,
      cols: options.cols,
      threadId: options.threadId,
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

  // ===== Shell management =====

  getAvailableShells = async (): Promise<ShellInfo[]> => {
    return await invoke<ShellInfo[]>('terminal_get_available_shells')
  }

  getDefaultShell = async (): Promise<ShellInfo> => {
    return await invoke<ShellInfo>('terminal_get_default_shell')
  }

  validateShellPath = async (path: string): Promise<boolean> => {
    return await invoke<boolean>('terminal_validate_shell_path', { path })
  }

  // ===== Utility methods =====

  terminalExists = async (paneId: number): Promise<boolean> => {
    const terminals = await this.listTerminals()
    return terminals.includes(paneId)
  }

  // ===== Terminal configuration management =====

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

  // ===== Event listening =====

  /**
   * Listen to terminal exit event
   */
  onTerminalExit = async (
    callback: (payload: { paneId: number; exitCode: number | null }) => void
  ): Promise<UnlistenFn> => {
    return listen<{ paneId: number; exitCode: number | null }>('terminal_exit', event => callback(event.payload))
  }

  onTerminalCreated = async (callback: (payload: { paneId: number }) => void): Promise<UnlistenFn> => {
    return listen<{ paneId: number }>('terminal_created', event => callback(event.payload))
  }

  onTerminalClosed = async (callback: (payload: { paneId: number }) => void): Promise<UnlistenFn> => {
    return listen<{ paneId: number }>('terminal_closed', event => callback(event.payload))
  }

  /**
   * Listen to CWD change event
   */
  onCwdChanged = async (callback: (payload: { paneId: number; cwd: string }) => void): Promise<UnlistenFn> => {
    return listen<{ paneId: number; cwd: string }>('pane_cwd_changed', event => callback(event.payload))
  }

  /**
   * Listen to window title change event (OSC 0/1/2)
   */
  onTitleChanged = async (callback: (payload: { paneId: number; title: string }) => void): Promise<UnlistenFn> => {
    return listen<{ paneId: number; title: string }>('pane_title_changed', event => callback(event.payload))
  }

  /**
   * Listen to command events (command start, execution, completion, etc.)
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
