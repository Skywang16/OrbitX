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

  async getAvailableShells(): Promise<ShellInfo[]> {
    return await invoke<ShellInfo[]>('get_available_shells')
  }

  async getDefaultShell(): Promise<ShellInfo> {
    return await invoke<ShellInfo>('get_default_shell')
  }

  async validateShellPath(path: string): Promise<boolean> {
    return await invoke<boolean>('validate_shell_path', { path })
  }

  // ===== 查找功能 =====

  async findShellByName(name: string): Promise<ShellInfo | null> {
    const shells = await this.getAvailableShells()
    return shells.find(shell => shell.name.toLowerCase() === name.toLowerCase()) || null
  }

  async findShellByPath(path: string): Promise<ShellInfo | null> {
    const shells = await this.getAvailableShells()
    return shells.find(shell => shell.path === path) || null
  }

  // ===== 后台命令执行功能 =====

  async executeBackgroundCommand(command: string, workingDirectory?: string): Promise<BackgroundCommandResult> {
    return await invoke<BackgroundCommandResult>('execute_background_command', {
      command,
      working_directory: workingDirectory,
    })
  }
}

// 导出单例实例
export const shellApi = new ShellApi()

// 导出类型
export type * from './types'

// 默认导出
export default shellApi
