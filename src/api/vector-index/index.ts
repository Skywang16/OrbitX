/**
 * 向量索引 API
 *
 * 统一封装与 Tauri 后端交互的向量索引相关命令，供前端调用。
 */

import { invoke } from '@/utils/request'
import type { VectorIndexConfig, VectorIndexStatus, IndexStats, VectorSearchOptions, VectorSearchResult } from './types'

export class VectorIndexApi {
  // 配置
  async getConfig(): Promise<VectorIndexConfig> {
    return await invoke<VectorIndexConfig>('get_vector_index_config')
  }

  async saveConfig(config: VectorIndexConfig): Promise<string> {
    return await invoke<string>('save_vector_index_config', { config })
  }

  async init(config: VectorIndexConfig): Promise<void> {
    await invoke('init_vector_index', { config })
  }

  async testConnection(config: VectorIndexConfig): Promise<string> {
    return await invoke<string>('test_qdrant_connection', { config })
  }

  // 状态与工作空间
  async getStatus(): Promise<VectorIndexStatus> {
    return await invoke<VectorIndexStatus>('get_vector_index_status')
  }

  async getWorkspacePath(): Promise<string> {
    return await invoke<string>('get_current_workspace_path')
  }

  // 构建与维护
  async build(workspacePath?: string): Promise<IndexStats> {
    const path = workspacePath ?? (await this.getWorkspacePath())
    return await invoke<IndexStats>('build_code_index', { workspacePath: path })
  }

  async cancelBuild(): Promise<void> {
    await invoke('cancel_build_index')
  }

  async clear(): Promise<string> {
    return await invoke<string>('clear_vector_index')
  }

  // 搜索
  async search(options: VectorSearchOptions): Promise<VectorSearchResult[]> {
    return await invoke<VectorSearchResult[]>('search_code_vectors', { options })
  }

  // 文件监控
  async startFileMonitoring(workspacePath: string, config: VectorIndexConfig): Promise<string> {
    return await invoke<string>('start_file_monitoring', { workspacePath, config })
  }

  async stopFileMonitoring(): Promise<string> {
    return await invoke<string>('stop_file_monitoring')
  }

  async getFileMonitoringStatus(): Promise<string> {
    return await invoke<string>('get_file_monitoring_status')
  }
}

export const vectorIndexApi = new VectorIndexApi()
export type VectorIndexApiType = typeof vectorIndexApi
export default vectorIndexApi
export type * from './types'
export * from './app-settings'
