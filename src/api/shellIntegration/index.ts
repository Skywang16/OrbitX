/**
 * Shell Integration API
 *
 * 提供Shell Integration相关的统一接口，包括：
 * - Shell Integration设置和状态检查
 * - 工作目录管理
 * - OSC序列处理
 */

import { invoke } from '@/utils/request'

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
    await invoke('shell_setup_integration', { paneId, silent })
  }

  /**
   * 检查Shell Integration状态
   * @param paneId 终端面板ID
   */
  async checkShellIntegrationStatus(paneId: number): Promise<boolean> {
    return await invoke<boolean>('shell_check_integration_status', { paneId })
  }

  // ===== 工作目录管理 =====

  // ===== 高级功能 =====

  /**
   * 获取Shell Integration详细状态
   * @param paneId 终端面板ID
   */
  async getShellIntegrationStatus(paneId: number): Promise<ShellIntegrationStatus> {
    const isEnabled = await this.checkShellIntegrationStatus(paneId)
    return {
      enabled: isEnabled,
      currentCwd: null,
      paneId,
    }
  }

  /**
   * 重新初始化Shell Integration
   * @param paneId 终端面板ID
   */
  async reinitializeShellIntegration(paneId: number): Promise<void> {
    // 先尝试静默设置
    await this.setupShellIntegration(paneId, true)

    // 检查是否成功
    const isEnabled = await this.checkShellIntegrationStatus(paneId)
    if (!isEnabled) {
      // 如果静默设置失败，尝试非静默设置
      await this.setupShellIntegration(paneId, false)
    }
  }
}

export const shellIntegrationApi = new ShellIntegrationApi()

// 默认导出
export default shellIntegrationApi
