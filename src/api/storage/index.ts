/**
 * 存储管理 API
 *
 * 提供存储系统的统一接口，包括：
 * - 配置管理
 * - 会话状态管理
 * - 数据查询和保存
 */

import { invoke } from '@/utils/request'
import { ConfigSection } from '@/types'
import type { SessionState, DataQuery, SaveOptions } from './types'

/**
 * 存储 API 接口类
 */
export class StorageApi {
  // ===== 配置管理 =====

  async getConfig<T = any>(section: ConfigSection | string): Promise<T> {
    const sectionName = typeof section === 'string' ? section : section
    return await invoke<T>('storage_get_config', { section: sectionName })
  }

  async updateConfig(section: ConfigSection | string, data: any): Promise<void> {
    const sectionName = typeof section === 'string' ? section : section
    await invoke<void>('storage_update_config', { section: sectionName, data })
  }

  // ===== 会话状态管理 =====

  async saveSessionState(sessionState: SessionState): Promise<void> {
    await invoke<void>('storage_save_session_state', { sessionState })
  }

  async loadSessionState(): Promise<SessionState | null> {
    return await invoke<SessionState | null>('storage_load_session_state')
  }

  // ===== 数据操作 =====

  async queryData<T = any>(query: DataQuery): Promise<T[]> {
    return await invoke<T[]>('storage_query_data', { query })
  }

  async saveData(data: any, options: SaveOptions): Promise<void> {
    await invoke<void>('storage_save_data', { data, options })
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

export const storageApi = new StorageApi()
export type * from './types'
export default storageApi
