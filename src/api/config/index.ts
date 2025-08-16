/**
 * 配置管理 API
 *
 * 提供应用配置的统一接口，包括：
 * - 配置获取和更新
 * - 文件操作
 * - 验证和重置
 */

import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'
import type { AppConfig, ConfigFileInfo } from './types'

// 导入主题模块
export * from './theme'

/**
 * 配置 API 接口类
 */
export class ConfigApi {
  async getConfig(): Promise<AppConfig> {
    try {
      return await invoke<AppConfig>('get_config')
    } catch (error) {
      throw new Error(handleError(error, '获取配置失败'))
    }
  }

  async updateConfig(config: AppConfig): Promise<void> {
    try {
      await invoke('update_config', { new_config: config })
    } catch (error) {
      throw new Error(handleError(error, '更新配置失败'))
    }
  }

  async saveConfig(): Promise<void> {
    try {
      await invoke('save_config')
    } catch (error) {
      throw new Error(handleError(error, '保存配置失败'))
    }
  }

  async validateConfig(): Promise<void> {
    try {
      await invoke('validate_config')
    } catch (error) {
      throw new Error(handleError(error, '验证配置失败'))
    }
  }

  async resetToDefaults(): Promise<void> {
    try {
      await invoke('reset_config_to_defaults')
    } catch (error) {
      throw new Error(handleError(error, '重置配置失败'))
    }
  }

  async getFilePath(): Promise<string> {
    try {
      return await invoke<string>('get_config_file_path')
    } catch (error) {
      throw new Error(handleError(error, '获取配置文件路径失败'))
    }
  }

  async getFileInfo(): Promise<ConfigFileInfo> {
    try {
      return await invoke<ConfigFileInfo>('get_config_file_info')
    } catch (error) {
      throw new Error(handleError(error, '获取配置文件信息失败'))
    }
  }

  async openFile(): Promise<void> {
    try {
      await invoke('open_config_file')
    } catch (error) {
      throw new Error(handleError(error, '打开配置文件失败'))
    }
  }

  async updateSection<K extends keyof AppConfig>(section: K, updates: Partial<AppConfig[K]>): Promise<void> {
    try {
      const currentConfig = await this.getConfig()
      const updatedConfig = {
        ...currentConfig,
        [section]: {
          ...(currentConfig[section] as object),
          ...(updates as object),
        },
      }
      await this.updateConfig(updatedConfig)
    } catch (error) {
      throw new Error(handleError(error, '更新配置部分失败'))
    }
  }

  async getSection<K extends keyof AppConfig>(section: K): Promise<AppConfig[K]> {
    try {
      const config = await this.getConfig()
      return config[section]
    } catch (error) {
      throw new Error(handleError(error, '获取配置部分失败'))
    }
  }
}

// 导出单例实例
export const configApi = new ConfigApi()

// 导出类型
export type * from './types'

// 默认导出
export default configApi
