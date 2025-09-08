/**
 * 向量索引应用级设置 API
 *
 * 管理向量索引功能的全局开关和工作目录配置
 */

import { invoke } from '@/utils/request'

export interface VectorIndexAppSettings {
  enabled: boolean
  workspaces: string[]
}

export class VectorIndexAppSettingsApi {
  // 获取向量索引应用设置
  async getSettings(): Promise<VectorIndexAppSettings> {
    return await invoke<VectorIndexAppSettings>('get_vector_index_app_settings')
  }

  // 保存向量索引应用设置
  async saveSettings(settings: VectorIndexAppSettings): Promise<string> {
    return await invoke<string>('save_vector_index_app_settings', { settings })
  }

  // 检查指定目录是否启用了向量索引
  async isDirectoryIndexed(directory: string): Promise<boolean> {
    return await invoke<boolean>('is_directory_vector_indexed', { directory })
  }

  // 添加工作目录到向量索引配置
  async addWorkspace(workspacePath: string): Promise<string> {
    return await invoke<string>('add_vector_index_workspace', { workspacePath })
  }

  // 移除工作目录从向量索引配置
  async removeWorkspace(workspacePath: string): Promise<string> {
    return await invoke<string>('remove_vector_index_workspace', { workspacePath })
  }
}

export const vectorIndexAppSettingsApi = new VectorIndexAppSettingsApi()
export default vectorIndexAppSettingsApi
