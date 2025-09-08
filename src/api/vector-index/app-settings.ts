/**
 * 向量索引应用级设置 API
 *
 * 管理向量索引功能的全局开关和工作目录配置
 */

import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'

export interface VectorIndexAppSettings {
  enabled: boolean
  workspaces: string[]
}

export class VectorIndexAppSettingsApi {
  // 获取向量索引应用设置
  async getSettings(): Promise<VectorIndexAppSettings> {
    try {
      return await invoke<VectorIndexAppSettings>('get_vector_index_app_settings')
    } catch (error) {
      throw new Error(handleError(error, '获取向量索引应用设置失败'))
    }
  }

  // 保存向量索引应用设置
  async saveSettings(settings: VectorIndexAppSettings): Promise<string> {
    try {
      return await invoke<string>('save_vector_index_app_settings', { settings })
    } catch (error) {
      throw new Error(handleError(error, '保存向量索引应用设置失败'))
    }
  }

  // 检查指定目录是否启用了向量索引
  async isDirectoryIndexed(directory: string): Promise<boolean> {
    try {
      return await invoke<boolean>('is_directory_vector_indexed', { directory })
    } catch (error) {
      console.warn('检查目录索引状态失败:', error)
      return false
    }
  }

  // 添加工作目录到向量索引配置
  async addWorkspace(workspacePath: string): Promise<string> {
    try {
      return await invoke<string>('add_vector_index_workspace', { workspacePath })
    } catch (error) {
      throw new Error(handleError(error, '添加向量索引工作目录失败'))
    }
  }

  // 移除工作目录从向量索引配置
  async removeWorkspace(workspacePath: string): Promise<string> {
    try {
      return await invoke<string>('remove_vector_index_workspace', { workspacePath })
    } catch (error) {
      throw new Error(handleError(error, '移除向量索引工作目录失败'))
    }
  }
}

export const vectorIndexAppSettingsApi = new VectorIndexAppSettingsApi()
export default vectorIndexAppSettingsApi
