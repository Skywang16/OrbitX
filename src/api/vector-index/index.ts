/**
 * 向量索引 API
 *
 * 统一封装与 Tauri 后端交互的向量索引相关命令，供前端调用。
 */

import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'
import type { VectorIndexConfig, VectorIndexStatus, IndexStats, VectorSearchOptions, VectorSearchResult } from './types'

export class VectorIndexApi {
  // 配置
  async getConfig(): Promise<VectorIndexConfig> {
    try {
      return await invoke<VectorIndexConfig>('get_vector_index_config')
    } catch (error) {
      throw new Error(handleError(error, '获取向量索引配置失败'))
    }
  }

  async saveConfig(config: VectorIndexConfig): Promise<string> {
    try {
      return await invoke<string>('save_vector_index_config', { config })
    } catch (error) {
      throw new Error(handleError(error, '保存向量索引配置失败'))
    }
  }

  async init(config: VectorIndexConfig): Promise<void> {
    try {
      await invoke('init_vector_index', { config })
    } catch (error) {
      throw new Error(handleError(error, '初始化向量索引失败'))
    }
  }

  async testConnection(config: VectorIndexConfig): Promise<string> {
    try {
      return await invoke<string>('test_qdrant_connection', { config })
    } catch (error) {
      throw new Error(handleError(error, '测试 Qdrant 连接失败'))
    }
  }

  // 状态与工作空间
  async getStatus(): Promise<VectorIndexStatus> {
    try {
      return await invoke<VectorIndexStatus>('get_vector_index_status')
    } catch (error) {
      // 后端未初始化时返回未初始化状态
      return { isInitialized: false, totalVectors: 0, lastUpdated: null }
    }
  }

  async getWorkspacePath(): Promise<string> {
    try {
      return await invoke<string>('get_current_workspace_path')
    } catch (error) {
      throw new Error(handleError(error, '获取工作空间路径失败'))
    }
  }

  // 构建与维护
  async build(workspacePath?: string): Promise<IndexStats> {
    try {
      const path = workspacePath ?? (await this.getWorkspacePath())
      return await invoke<IndexStats>('build_code_index', { workspace_path: path })
    } catch (error) {
      throw new Error(handleError(error, '构建代码索引失败'))
    }
  }

  async cancelBuild(): Promise<void> {
    try {
      await invoke('cancel_build_index')
    } catch (error) {
      throw new Error(handleError(error, '取消构建失败'))
    }
  }

  async clear(): Promise<string> {
    try {
      return await invoke<string>('clear_vector_index')
    } catch (error) {
      throw new Error(handleError(error, '清空索引失败'))
    }
  }

  // 搜索
  async search(options: VectorSearchOptions): Promise<VectorSearchResult[]> {
    try {
      return await invoke<VectorSearchResult[]>('search_code_vectors', { options })
    } catch (error) {
      throw new Error(handleError(error, '向量搜索失败'))
    }
  }

  // 文件监控
  async startFileMonitoring(workspacePath: string, config: VectorIndexConfig): Promise<string> {
    try {
      return await invoke<string>('start_file_monitoring', { workspace_path: workspacePath, config })
    } catch (error) {
      throw new Error(handleError(error, '启动文件监控失败'))
    }
  }

  async stopFileMonitoring(): Promise<string> {
    try {
      return await invoke<string>('stop_file_monitoring')
    } catch (error) {
      throw new Error(handleError(error, '停止文件监控失败'))
    }
  }

  async getFileMonitoringStatus(): Promise<string> {
    try {
      return await invoke<string>('get_file_monitoring_status')
    } catch (error) {
      throw new Error(handleError(error, '获取文件监控状态失败'))
    }
  }
}

export const vectorIndexApi = new VectorIndexApi()
export type VectorIndexApiType = typeof vectorIndexApi
export default vectorIndexApi
export type * from './types'
