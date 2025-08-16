/**
 * 配置管理 API
 *
 * 提供应用配置的统一接口，包括：
 * - 配置获取和更新
 * - 文件操作
 * - 验证和重置
 * - 主题管理
 */

import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'
import type { AppConfig, ConfigFileInfo, Theme, ThemeInfo, ThemeConfigStatus } from './types'

/**
 * 主题管理 API 类
 */
class ThemeAPI {
  async getThemeConfigStatus(): Promise<ThemeConfigStatus> {
    try {
      return await invoke<ThemeConfigStatus>('get_theme_config_status')
    } catch (error) {
      throw new Error(handleError(error, '获取主题配置状态失败'))
    }
  }

  async getCurrentTheme(): Promise<Theme> {
    try {
      return await invoke<Theme>('get_current_theme')
    } catch (error) {
      throw new Error(handleError(error, '获取当前主题失败'))
    }
  }

  async getAvailableThemes(): Promise<ThemeInfo[]> {
    try {
      return await invoke<ThemeInfo[]>('get_available_themes')
    } catch (error) {
      throw new Error(handleError(error, '获取可用主题失败'))
    }
  }

  async setTerminalTheme(name: string): Promise<void> {
    try {
      await invoke('set_terminal_theme', { themeName: name })
    } catch (error) {
      throw new Error(handleError(error, '设置终端主题失败'))
    }
  }

  async setFollowSystemTheme(followSystem: boolean, lightTheme?: string, darkTheme?: string): Promise<void> {
    try {
      await invoke('set_follow_system_theme', {
        followSystem,
        lightTheme: lightTheme || null,
        darkTheme: darkTheme || null,
      })
    } catch (error) {
      throw new Error(handleError(error, '设置跟随系统主题失败'))
    }
  }
}

/**
 * 配置 API 接口类
 */
export class ConfigApi {
  private themeAPI = new ThemeAPI()

  // ===== 基础配置管理 =====

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

  async resetConfig(): Promise<void> {
    try {
      await invoke('reset_config')
    } catch (error) {
      throw new Error(handleError(error, '重置配置失败'))
    }
  }

  async reloadConfig(): Promise<void> {
    try {
      await invoke('reload_config')
    } catch (error) {
      throw new Error(handleError(error, '重新加载配置失败'))
    }
  }

  // ===== 文件操作 =====

  async getConfigFileInfo(): Promise<ConfigFileInfo> {
    try {
      return await invoke<ConfigFileInfo>('get_config_file_info')
    } catch (error) {
      throw new Error(handleError(error, '获取配置文件信息失败'))
    }
  }

  async exportConfig(): Promise<string> {
    try {
      return await invoke<string>('export_config')
    } catch (error) {
      throw new Error(handleError(error, '导出配置失败'))
    }
  }

  async importConfig(configData: string): Promise<void> {
    try {
      await invoke('import_config', { data: configData })
    } catch (error) {
      throw new Error(handleError(error, '导入配置失败'))
    }
  }

  async resetToDefaults(): Promise<void> {
    try {
      await invoke('reset_config_to_defaults')
    } catch (error) {
      throw new Error(handleError(error, '重置配置为默认值失败'))
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

  // ===== 主题管理（代理到 ThemeAPI） =====

  async getThemeConfigStatus(): Promise<ThemeConfigStatus> {
    return this.themeAPI.getThemeConfigStatus()
  }

  async getCurrentTheme(): Promise<Theme> {
    return this.themeAPI.getCurrentTheme()
  }

  async getAvailableThemes(): Promise<ThemeInfo[]> {
    return this.themeAPI.getAvailableThemes()
  }

  async setTerminalTheme(name: string): Promise<void> {
    return this.themeAPI.setTerminalTheme(name)
  }

  async setFollowSystemTheme(followSystem: boolean, lightTheme?: string, darkTheme?: string): Promise<void> {
    return this.themeAPI.setFollowSystemTheme(followSystem, lightTheme, darkTheme)
  }

  // ===== 便捷访问器 =====

  get theme() {
    return this.themeAPI
  }
}

// 导出单例实例
export const configApi = new ConfigApi()

// 向后兼容的主题 API 导出
export const themeAPI = configApi.theme

// 导出类型
export type * from './types'

// 默认导出
export default configApi
