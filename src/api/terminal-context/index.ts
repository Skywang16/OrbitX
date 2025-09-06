/**
 * 终端上下文管理 API
 *
 * 提供终端上下文管理的统一接口，包括：
 * - 活跃终端管理
 * - 终端上下文查询
 * - 上下文缓存管理
 */

import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'
import type { TerminalContext } from './types'

/**
 * 终端上下文 API 接口类
 */
export class TerminalContextApi {
  // ===== 活跃终端管理 =====

  /**
   * 设置活跃终端面板ID
   * @param paneId 面板ID
   */
  async setActivePaneId(paneId: number): Promise<void> {
    try {
      await invoke('set_active_pane', { paneId })
    } catch (error) {
      throw new Error(handleError(error, '设置活跃终端失败'))
    }
  }

  /**
   * 获取当前活跃终端面板ID
   * @returns 活跃终端面板ID，如果没有活跃终端则返回null
   */
  async getActivePaneId(): Promise<number | null> {
    try {
      return await invoke<number | null>('get_active_pane')
    } catch (error) {
      throw new Error(handleError(error, '获取活跃终端失败'))
    }
  }

  // ===== 终端上下文查询 =====

  /**
   * 获取终端上下文信息
   * @param paneId 可选的面板ID，如果不提供则获取活跃终端的上下文
   * @returns 终端上下文信息
   */
  async getTerminalContext(paneId?: number): Promise<TerminalContext> {
    try {
      return await invoke<TerminalContext>('get_terminal_context', { paneId })
    } catch (error) {
      throw new Error(handleError(error, '获取终端上下文失败'))
    }
  }

  /**
   * 获取活跃终端的上下文信息
   * @returns 活跃终端的上下文信息
   */
  async getActiveTerminalContext(): Promise<TerminalContext> {
    try {
      return await invoke<TerminalContext>('get_active_terminal_context')
    } catch (error) {
      throw new Error(handleError(error, '获取活跃终端上下文失败'))
    }
  }

  // ===== 便捷方法 =====

  /**
   * 获取指定终端的当前工作目录
   * @param paneId 可选的面板ID，如果不提供则获取活跃终端的CWD
   * @returns 当前工作目录路径
   */
  async getCurrentWorkingDirectory(paneId?: number): Promise<string | null> {
    try {
      const context = await this.getTerminalContext(paneId)
      return context.currentWorkingDirectory
    } catch (error) {
      console.warn('获取当前工作目录失败:', handleError(error))
      return null
    }
  }

  /**
   * 获取指定终端的Shell类型
   * @param paneId 可选的面板ID，如果不提供则获取活跃终端的Shell类型
   * @returns Shell类型
   */
  async getShellType(paneId?: number): Promise<string | null> {
    try {
      const context = await this.getTerminalContext(paneId)
      return context.shellType
    } catch (error) {
      console.warn('获取Shell类型失败:', handleError(error))
      return null
    }
  }

  /**
   * 检查指定终端是否启用了Shell集成
   * @param paneId 可选的面板ID，如果不提供则检查活跃终端
   * @returns 是否启用Shell集成
   */
  async isShellIntegrationEnabled(paneId?: number): Promise<boolean> {
    try {
      const context = await this.getTerminalContext(paneId)
      return context.shellIntegrationEnabled
    } catch (error) {
      console.warn('检查Shell集成状态失败:', handleError(error))
      return false
    }
  }

  /**
   * 检查终端是否存在且可访问
   * @param paneId 面板ID
   * @returns 终端是否存在
   */
  async terminalExists(paneId: number): Promise<boolean> {
    try {
      await this.getTerminalContext(paneId)
      return true
    } catch (error) {
      return false
    }
  }
}

// 导出单例实例
export const terminalContextApi = new TerminalContextApi()

// 导出类型
export type * from './types'

// 默认导出
export default terminalContextApi
