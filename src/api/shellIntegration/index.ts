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
 * Shell Integration API 接口类
 */
export class ShellIntegrationApi {
  // ===== Shell Integration 设置 =====

  /**
   * 设置Shell Integration
   * @param paneId 终端面板ID
   * @param silent 是否静默设置
   */
  setupShellIntegration = async (paneId: number, silent: boolean = true): Promise<void> => {
    await invoke('shell_setup_integration', { paneId, silent })
  }

  /**
   * 检查Shell Integration状态
   * @param paneId 终端面板ID
   */
  checkShellIntegrationStatus = async (paneId: number): Promise<boolean> => {
    return await invoke<boolean>('shell_check_integration_status', { paneId })
  }

  /**
   * 获取面板的 Shell 状态快照（包含 node_version 等）
   */
  getPaneShellState = async <T = { node_version?: string | null } | null>(paneId: number): Promise<T> => {
    // 直接调用后端命令 get_pane_shell_state
    // 返回 FrontendPaneState，可按需解构 node_version 字段
    return await invoke<T>('get_pane_shell_state', { paneId })
  }
}

export const shellIntegrationApi = new ShellIntegrationApi()

// 默认导出
export default shellIntegrationApi
