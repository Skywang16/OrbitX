/**
 * Shell 管理 API
 *
 * 提供 Shell 管理的统一接口，包括：
 * - Shell 发现和验证
 * - 配置管理
 * - 功能检测
 */

import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'
import type { ShellInfo, BackgroundCommandResult } from './types'

/**
 * Shell API 接口类
 */
export class ShellApi {
  // ===== 基本操作 =====

  async getAvailableShells(): Promise<ShellInfo[]> {
    try {
      return await invoke<ShellInfo[]>('get_available_shells')
    } catch (error) {
      throw new Error(handleError(error, '获取可用Shell列表失败'))
    }
  }

  async getDefaultShell(): Promise<ShellInfo> {
    try {
      return await invoke<ShellInfo>('get_default_shell')
    } catch (error) {
      throw new Error(handleError(error, '获取默认Shell失败'))
    }
  }

  async validateShellPath(path: string): Promise<boolean> {
    try {
      return await invoke<boolean>('validate_shell_path', { path })
    } catch (error) {
      console.warn('验证Shell路径失败:', handleError(error))
      return false
    }
  }

  // ===== 查找功能 =====

  async findShellByName(name: string): Promise<ShellInfo | null> {
    try {
      const shells = await this.getAvailableShells()
      return shells.find(shell => shell.name.toLowerCase() === name.toLowerCase()) || null
    } catch (error) {
      console.warn('根据名称查找Shell失败:', handleError(error))
      return null
    }
  }

  async findShellByPath(path: string): Promise<ShellInfo | null> {
    try {
      const shells = await this.getAvailableShells()
      return shells.find(shell => shell.path === path) || null
    } catch (error) {
      console.warn('根据路径查找Shell失败:', handleError(error))
      return null
    }
  }

  // ===== 后台命令执行功能 =====

  async executeBackgroundCommand(command: string, workingDirectory?: string): Promise<BackgroundCommandResult> {
    try {
      return await invoke<BackgroundCommandResult>('execute_background_command', {
        command,
        working_directory: workingDirectory,
      })
    } catch (error) {
      throw new Error(handleError(error, '后台命令执行失败'))
    }
  }
}

// 导出单例实例
export const shellApi = new ShellApi()

// 导出类型
export type * from './types'

// 默认导出
export default shellApi
