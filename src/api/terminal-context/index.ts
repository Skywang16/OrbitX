/**
 * 终端上下文管理 API
 *
 * 提供终端上下文管理的统一接口，包括：
 * - 活跃终端管理
 * - 终端上下文查询
 * - 上下文缓存管理
 */

import { invoke } from '@/utils/request'
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
  setActivePaneId = async (paneId: number): Promise<void> => {
    await invoke('terminal_context_set_active_pane', { paneId })
  }

  /**
   * 获取当前活跃终端面板ID
   * @returns 活跃终端面板ID，如果没有活跃终端则返回null
   */
  getActivePaneId = async (): Promise<number | null> => {
    return await invoke<number | null>('terminal_context_get_active_pane')
  }

  // ===== 终端上下文查询 =====

  /**
   * 获取终端上下文信息
   * @param paneId 可选的面板ID，如果不提供则获取活跃终端的上下文
   * @returns 终端上下文信息
   */
  getTerminalContext = async (paneId?: number): Promise<TerminalContext> => {
    return await invoke<TerminalContext>('terminal_context_get', { paneId })
  }

  /**
   * 获取活跃终端的上下文信息
   * @returns 活跃终端的上下文信息
   */
  getActiveTerminalContext = async (): Promise<TerminalContext> => {
    return await invoke<TerminalContext>('terminal_context_get_active')
  }

  // ===== 便捷方法 =====

  /**
   * 获取指定终端的当前工作目录
   * @param paneId 可选的面板ID，如果不提供则获取活跃终端的CWD
   * @returns 当前工作目录路径
   */
  getCurrentWorkingDirectory = async (paneId?: number): Promise<string | null> => {
    const context = await this.getTerminalContext(paneId)
    return context.currentWorkingDirectory
  }

  /**
   * 获取指定终端的Shell类型
   * @param paneId 可选的面板ID，如果不提供则获取活跃终端的Shell类型
   * @returns Shell类型
   */
  getShellType = async (paneId?: number): Promise<string | null> => {
    const context = await this.getTerminalContext(paneId)
    return context.shellType
  }

  /**
   * 检查指定终端是否启用了Shell集成
   * @param paneId 可选的面板ID，如果不提供则检查活跃终端
   * @returns 是否启用Shell集成
   */
  isShellIntegrationEnabled = async (paneId?: number): Promise<boolean> => {
    const context = await this.getTerminalContext(paneId)
    return context.shellIntegrationEnabled
  }

  /**
   * 检查终端是否存在且可访问
   * @param paneId 面板ID
   * @returns 终端是否存在
   */
  terminalExists = async (paneId: number): Promise<boolean> => {
    await this.getTerminalContext(paneId)
    return true
  }
}

export const terminalContextApi = new TerminalContextApi()
export type * from './types'
export default terminalContextApi
