/**
 * Shell 管理 API
 *
 * 提供 Shell 管理的统一接口，包括：
 * - Shell 发现和验证
 * - 配置管理
 * - 功能检测
 */

import { invoke } from '@/utils/request'
import type { ShellInfo, BackgroundCommandResult } from './types'

/**
 * Shell API 接口类
 */
export class ShellApi {
  // ===== 基本操作 =====

  getAvailableShells = async (): Promise<ShellInfo[]> => {
    return await invoke<ShellInfo[]>('terminal_get_available_shells')
  }

  getDefaultShell = async (): Promise<ShellInfo> => {
    return await invoke<ShellInfo>('terminal_get_default_shell')
  }

  validateShellPath = async (path: string): Promise<boolean> => {
    return await invoke<boolean>('terminal_validate_shell_path', { path })
  }

  // ===== 查找功能 =====

  findShellByName = async (name: string): Promise<ShellInfo | null> => {
    const shells = await this.getAvailableShells()
    return shells.find(shell => shell.name.toLowerCase() === name.toLowerCase()) || null
  }

  findShellByPath = async (path: string): Promise<ShellInfo | null> => {
    const shells = await this.getAvailableShells()
    return shells.find(shell => shell.path === path) || null
  }

  // ===== 后台命令执行功能 =====

  executeBackgroundCommand = async (command: string, workingDirectory?: string): Promise<BackgroundCommandResult> => {
    return await invoke<BackgroundCommandResult>('shell_execute_background_command', {
      command,
      working_directory: workingDirectory,
    })
  }

  executeBackgroundProgram = async (
    program: string,
    args: string[],
    workingDirectory?: string
  ): Promise<BackgroundCommandResult> => {
    return await invoke<BackgroundCommandResult>('shell_execute_background_program', {
      program,
      args,
      working_directory: workingDirectory,
    })
  }
}

export const shellApi = new ShellApi()
export type * from './types'
export default shellApi
