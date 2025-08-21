/**
 * 存储管理 API
 *
 * 提供存储系统的统一接口，包括：
 * - 配置管理
 * - 会话状态管理
 * - 数据查询和保存
 */

import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'
import { ConfigSection } from '@/types'
import type { SessionState, DataQuery, SaveOptions } from './types'

/**
 * 存储 API 接口类
 */
export class StorageApi {
  // ===== 配置管理 =====

  async getConfig<T = any>(section: ConfigSection | string): Promise<T> {
    try {
      const sectionName = typeof section === 'string' ? section : section
      return await invoke<T>('storage_get_config', { section: sectionName })
    } catch (error) {
      throw new Error(handleError(error, `获取配置节 ${section} 失败`))
    }
  }

  async updateConfig(section: ConfigSection | string, data: any): Promise<void> {
    try {
      const sectionName = typeof section === 'string' ? section : section
      await invoke('storage_update_config', { section: sectionName, data })
    } catch (error) {
      throw new Error(handleError(error, `更新配置节 ${section} 失败`))
    }
  }

  // ===== 会话状态管理 =====

  async saveSessionState(sessionState: SessionState): Promise<void> {
    try {
      await invoke('storage_save_session_state', { sessionState })
    } catch (error) {
      throw new Error(handleError(error, '保存会话状态失败'))
    }
  }

  async loadSessionState(): Promise<SessionState | null> {
    try {
      return await invoke<SessionState | null>('storage_load_session_state')
    } catch (error) {
      throw new Error(handleError(error, '加载会话状态失败'))
    }
  }

  // ===== 数据操作 =====

  async queryData<T = any>(query: DataQuery): Promise<T[]> {
    try {
      return await invoke<T[]>('storage_query_data', { query })
    } catch (error) {
      throw new Error(handleError(error, '查询数据失败'))
    }
  }

  async saveData(data: any, options: SaveOptions): Promise<void> {
    try {
      await invoke('storage_save_data', { data, options })
    } catch (error) {
      throw new Error(handleError(error, '保存数据失败'))
    }
  }

  // ===== 便捷方法 =====

  async getAppConfig<T = any>(): Promise<T> {
    return this.getConfig<T>(ConfigSection.App)
  }

  async getAppearanceConfig<T = any>(): Promise<T> {
    return this.getConfig<T>(ConfigSection.Appearance)
  }

  async getTerminalConfig<T = any>(): Promise<T> {
    return this.getConfig<T>(ConfigSection.Terminal)
  }

  async getShortcutsConfig<T = any>(): Promise<T> {
    return this.getConfig<T>(ConfigSection.Shortcuts)
  }

  async getAiConfig<T = any>(): Promise<T> {
    return this.getConfig<T>(ConfigSection.Ai)
  }

  async updateAppConfig(data: any): Promise<void> {
    return this.updateConfig(ConfigSection.App, data)
  }

  async updateAppearanceConfig(data: any): Promise<void> {
    return this.updateConfig(ConfigSection.Appearance, data)
  }

  async updateTerminalConfig(data: any): Promise<void> {
    return this.updateConfig(ConfigSection.Terminal, data)
  }

  async updateShortcutsConfig(data: any): Promise<void> {
    return this.updateConfig(ConfigSection.Shortcuts, data)
  }

  async updateAiConfig(data: any): Promise<void> {
    return this.updateConfig(ConfigSection.Ai, data)
  }
}

// 导出单例实例
export const storageApi = new StorageApi()

// 导出类型
export type * from './types'

// 默认导出
export default storageApi
