/**
 * CK (seek) API - 语义代码搜索接口
 *
 * 提供与CK搜索引擎的交互接口，包括索引管理和语义搜索功能
 */

import { invoke } from '@/utils/request'
import type { CkSearchResult, CkBuildProgress } from './types'

export interface CkSearchParams {
  query: string
  path: string // 路径在搜索时是必需的
  mode?: 'semantic' | 'hybrid' | 'regex'
  maxResults?: number
}

export interface CkIndexStatus {
  isReady: boolean // 后端字段已更改
  path: string
}

/**
 * CK API 类
 */
export class CkApi {
  /**
   * 获取CK索引状态（传入路径）
   */
  async getIndexStatus(params: { path: string }): Promise<CkIndexStatus> {
    try {
      return await invoke<CkIndexStatus>('ck_index_status', { path: params.path })
    } catch (error) {
      console.error('获取CK索引状态失败:', error)
      return {
        isReady: false,
        path: '',
      }
    }
  }

  /**
   * 构建CK索引（传入路径）
   */
  async buildIndex(params: { path: string }): Promise<void> {
    await invoke('ck_build_index', { path: params.path })
  }

  /**
   * 删除CK索引（传入路径）
   */
  async deleteIndex(params: { path: string }): Promise<void> {
    return await invoke('ck_delete_index', { path: params.path })
  }

  /**
   * 获取CK构建进度（传入路径）
   */
  async getBuildProgress(params: { path: string }): Promise<CkBuildProgress> {
    return await invoke('ck_get_build_progress', { path: params.path })
  }

  /**
   * 取消CK构建（传入路径）
   */
  async cancelBuild(params: { path: string }): Promise<void> {
    return await invoke('ck_cancel_build', { path: params.path })
  }

  /**
   * 执行CK代码搜索
   */
  async search(params: CkSearchParams): Promise<CkSearchResult[]> {
    return await invoke('ck_search', {
      query: params.query,
      path: params.path, // 字段名已更正
      mode: params.mode || 'semantic',
      maxResults: params.maxResults || 10,
    })
  }
}

// 导出单例实例
export const ckApi = new CkApi()

// 导出类型
export type * from './types'

// 默认导出
export default ckApi
