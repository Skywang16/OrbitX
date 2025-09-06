/**
 * Shell Integration API
 *
 * 提供Shell Integration相关的统一接口，包括：
 * - Shell Integration设置和状态检查
 * - 工作目录管理
 * - OSC序列处理
 */

import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'

/**
 * Shell Integration 状态
 */
export interface ShellIntegrationStatus {
  /** 是否启用 */
  enabled: boolean
  /** 当前工作目录 */
  currentCwd: string | null
  /** 终端面板ID */
  paneId: number
  /** 最后更新时间 */
  lastUpdated?: Date
}

/**
 * Shell Integration API 接口类
 */
export class ShellIntegrationApi {
  // ===== Shell Integration 设置 =====

  /**
   * 设置Shell Integration
   * @param paneId 终端面板ID
   * @param silent 是否静默设置
   */
  async setupShellIntegration(paneId: number, silent: boolean = true): Promise<void> {
    try {
      await invoke('setup_shell_integration', { paneId, silent })
    } catch (error) {
      throw new Error(handleError(error, '设置Shell Integration失败'))
    }
  }

  /**
   * 检查Shell Integration状态
   * @param paneId 终端面板ID
   */
  async checkShellIntegrationStatus(paneId: number): Promise<boolean> {
    try {
      return await invoke<boolean>('check_shell_integration_status', { paneId })
    } catch (error) {
      console.warn('检查Shell Integration状态失败:', handleError(error))
      return false
    }
  }

  // ===== 工作目录管理 =====

  // Note: updatePaneCwd method removed - backend is now the single source of truth for CWD
  // Frontend should only subscribe to CWD change events, not write back to backend

  /**
   * 获取面板工作目录
   * @param paneId 终端面板ID
   */
  async getPaneCwd(paneId: number): Promise<string | null> {
    try {
      return await invoke<string | null>('get_pane_cwd', { paneId })
    } catch (error) {
      console.warn('获取面板工作目录失败:', handleError(error))
      return null
    }
  }

  // ===== 高级功能 =====

  /**
   * 获取Shell Integration详细状态
   * @param paneId 终端面板ID
   */
  async getShellIntegrationStatus(paneId: number): Promise<ShellIntegrationStatus> {
    try {
      const isEnabled = await this.checkShellIntegrationStatus(paneId)
      const currentCwd = await this.getPaneCwd(paneId)

      return {
        enabled: isEnabled,
        currentCwd,
        paneId,
      }
    } catch (error) {
      console.warn('获取Shell Integration详细状态失败:', handleError(error))
      return {
        enabled: false,
        currentCwd: null,
        paneId,
      }
    }
  }

  /**
   * 重新初始化Shell Integration
   * @param paneId 终端面板ID
   */
  async reinitializeShellIntegration(paneId: number): Promise<void> {
    try {
      // 先尝试静默设置
      await this.setupShellIntegration(paneId, true)

      // 检查是否成功
      const isEnabled = await this.checkShellIntegrationStatus(paneId)
      if (!isEnabled) {
        // 如果静默设置失败，尝试非静默设置
        await this.setupShellIntegration(paneId, false)
      }
    } catch (error) {
      throw new Error(handleError(error, '重新初始化Shell Integration失败'))
    }
  }
}

// 导出单例实例
export const shellIntegrationApi = new ShellIntegrationApi()

// 默认导出
export default shellIntegrationApi
