/**
 * 工作区管理 API
 *
 * 提供最近打开工作区的管理功能
 */

import { invoke } from '@tauri-apps/api/core'

/**
 * 最近工作区条目
 */
export interface RecentWorkspace {
  id: number
  path: string
  last_accessed_at: number
}

/**
 * 工作区 API 封装类
 */
export class WorkspaceApi {
  /**
   * 获取最近打开的工作区列表
   * @param limit 限制返回数量，默认10个，最多50个
   */
  async getRecentWorkspaces(limit?: number): Promise<RecentWorkspace[]> {
    return invoke<RecentWorkspace[]>('workspace_get_recent', { limit })
  }

  /**
   * 添加或更新工作区访问记录
   * @param path 工作区路径
   */
  async addRecentWorkspace(path: string): Promise<void> {
    await invoke('workspace_add_recent', { path })
  }

  /**
   * 删除指定工作区记录
   * @param path 工作区路径
   */
  async removeRecentWorkspace(path: string): Promise<void> {
    await invoke('workspace_remove_recent', { path })
  }

  /**
   * 维护数据：清理过期记录 + 限制总数
   * @returns [old_count, excess_count] 分别表示过期记录数和超量记录数
   */
  async maintainWorkspaces(): Promise<[number, number]> {
    return invoke<[number, number]>('workspace_maintain')
  }
}

export const workspaceApi = new WorkspaceApi()

export default workspaceApi
