/**
 * 配置管理 API
 *
 * 提供与后端配置系统交互的 API 接口，包括配置获取、更新、重载等功能。
 */

import { invoke } from '@/utils/request'
import { handleError } from '../../utils/errorHandler'
import type { AppConfig, ConfigFileInfo, ConfigOperationResult, ConfigSectionUpdate } from './types'

// ============================================================================
// 配置管理 API 类
// ============================================================================

/**
 * 配置管理API类
 * 提供配置的获取、更新、保存、重载等功能
 */
export class ConfigAPI {
  /**
   * 获取当前配置
   */
  async getConfig(): Promise<AppConfig> {
    try {
      return await invoke<AppConfig>('get_config')
    } catch (error) {
      throw new Error(handleError(error, '获取配置失败'))
    }
  }

  /**
   * 更新配置
   */
  async updateConfig(config: AppConfig): Promise<void> {
    try {
      return await invoke<void>('update_config', { new_config: config })
    } catch (error) {
      throw new Error(handleError(error, '更新配置失败'))
    }
  }

  /**
   * 保存配置
   */
  async saveConfig(): Promise<void> {
    try {
      return await invoke<void>('save_config')
    } catch (error) {
      throw new Error(handleError(error, '保存配置失败'))
    }
  }

  /**
   * 验证配置
   */
  async validateConfig(): Promise<void> {
    try {
      return await invoke<void>('validate_config')
    } catch (error) {
      throw new Error(handleError(error, '验证配置失败'))
    }
  }

  /**
   * 重置配置为默认值
   */
  async resetConfigToDefaults(): Promise<void> {
    try {
      return await invoke<void>('reset_config_to_defaults')
    } catch (error) {
      throw new Error(handleError(error, '重置配置失败'))
    }
  }

  /**
   * 获取配置文件路径
   */
  async getConfigFilePath(): Promise<string> {
    try {
      return await invoke<string>('get_config_file_path')
    } catch (error) {
      throw new Error(handleError(error, '获取配置文件路径失败'))
    }
  }

  /**
   * 获取配置文件信息
   */
  async getConfigFileInfo(): Promise<ConfigFileInfo> {
    try {
      return await invoke<ConfigFileInfo>('get_config_file_info')
    } catch (error) {
      throw new Error(handleError(error, '获取配置文件信息失败'))
    }
  }

  /**
   * 打开配置文件
   */
  async openConfigFile(): Promise<void> {
    try {
      return await invoke<void>('open_config_file')
    } catch (error) {
      throw new Error(handleError(error, '打开配置文件失败'))
    }
  }

  /**
   * 更新配置的特定部分
   */
  async updateConfigSection<K extends keyof AppConfig>(section: K, updates: Partial<AppConfig[K]>): Promise<void> {
    try {
      const currentConfig = await this.getConfig()
      const updatedConfig = {
        ...currentConfig,
        [section]: {
          ...(currentConfig[section] as object),
          ...(updates as object),
        },
      }
      return await this.updateConfig(updatedConfig)
    } catch (error) {
      throw new Error(handleError(error, '更新配置部分失败'))
    }
  }

  /**
   * 获取配置的特定部分
   */
  async getConfigSection<K extends keyof AppConfig>(section: K): Promise<AppConfig[K]> {
    try {
      const config = await this.getConfig()
      return config[section]
    } catch (error) {
      throw new Error(handleError(error, '获取配置部分失败'))
    }
  }

  /**
   * 安全的配置更新（带错误处理）
   */
  async safeUpdateConfig(config: AppConfig): Promise<ConfigOperationResult> {
    try {
      await this.updateConfig(config)
      return { success: true }
    } catch (error) {
      return {
        success: false,
        error: handleError(error, '配置更新失败'),
      }
    }
  }

  /**
   * 批量更新配置部分
   */
  async updateMultipleConfigSections(updates: ConfigSectionUpdate[]): Promise<void> {
    try {
      const currentConfig = await this.getConfig()
      let updatedConfig = { ...currentConfig }

      for (const update of updates) {
        updatedConfig = {
          ...updatedConfig,
          [update.section]: {
            ...(updatedConfig[update.section as keyof AppConfig] as object),
            ...(update.updates as object),
          },
        }
      }

      return await this.updateConfig(updatedConfig)
    } catch (error) {
      throw new Error(handleError(error, '批量更新配置失败'))
    }
  }
}

/**
 * 配置API实例
 */
export const configAPI = new ConfigAPI()

/**
 * 便捷的配置操作函数
 */
export const config = {
  // 基本操作
  get: () => configAPI.getConfig(),
  update: (config: AppConfig) => configAPI.updateConfig(config),
  save: () => configAPI.saveConfig(),
  validate: () => configAPI.validateConfig(),
  reset: () => configAPI.resetConfigToDefaults(),

  // 文件操作
  getFilePath: () => configAPI.getConfigFilePath(),
  getFileInfo: () => configAPI.getConfigFileInfo(),
  openFile: () => configAPI.openConfigFile(),

  // 部分操作
  getSection: <K extends keyof AppConfig>(section: K) => configAPI.getConfigSection(section),
  updateSection: <K extends keyof AppConfig>(section: K, updates: Partial<AppConfig[K]>) =>
    configAPI.updateConfigSection(section, updates),

  // 高级功能
  safeUpdate: (config: AppConfig) => configAPI.safeUpdateConfig(config),
  updateMultiple: (updates: ConfigSectionUpdate[]) => configAPI.updateMultipleConfigSections(updates),
}

// 重新导出类型
export type * from './types'

// 导出单独的函数以保持向后兼容
export const getConfig = () => configAPI.getConfig()
export const updateConfig = (config: AppConfig) => configAPI.updateConfig(config)
export const saveConfig = () => configAPI.saveConfig()
export const validateConfig = () => configAPI.validateConfig()
export const resetConfigToDefaults = () => configAPI.resetConfigToDefaults()
export const getConfigFilePath = () => configAPI.getConfigFilePath()
export const getConfigFileInfo = () => configAPI.getConfigFileInfo()
export const openConfigFile = () => configAPI.openConfigFile()
export const getConfigSection = <K extends keyof AppConfig>(section: K) => configAPI.getConfigSection(section)
export const updateConfigSection = <K extends keyof AppConfig>(section: K, updates: Partial<AppConfig[K]>) =>
  configAPI.updateConfigSection(section, updates)

// 导出主题相关API
export * from './theme'
