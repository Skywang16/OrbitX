/**
 * 存储管理 API
 *
 * 提供与后端存储系统交互的 API 接口，基于新的三层存储架构
 */

import { invoke } from '@/utils/request'
import { handleError } from '@/utils/errorHandler'
import { ConfigSection } from '@/types/storage'
import type {
  SessionState,
  DataQuery,
  SaveOptions,
  CacheStats,
  StorageStats,
  HealthCheckResult,
  StorageOperationResult,
} from './types'

// ============================================================================
// 存储管理 API 类
// ============================================================================

/**
 * 存储管理API类
 * 封装后端的11个存储命令，提供统一的存储管理接口
 */
export class StorageAPI {
  // ============================================================================
  // 配置管理
  // ============================================================================

  /**
   * 获取配置数据
   */
  async getConfig<T = any>(section: ConfigSection | string): Promise<T> {
    try {
      const sectionName = typeof section === 'string' ? section : section
      return await invoke<T>('storage_get_config', { section: sectionName })
    } catch (error) {
      throw new Error(handleError(error, `获取配置节 ${section} 失败`))
    }
  }

  /**
   * 更新配置数据
   */
  async updateConfig(section: ConfigSection | string, data: any): Promise<void> {
    try {
      const sectionName = typeof section === 'string' ? section : section
      await invoke<void>('storage_update_config', { section: sectionName, data })
    } catch (error) {
      throw new Error(handleError(error, `更新配置节 ${section} 失败`))
    }
  }

  // ============================================================================
  // 会话状态管理
  // ============================================================================

  /**
   * 保存会话状态
   */
  async saveSessionState(sessionState: SessionState): Promise<void> {
    try {
      await invoke<void>('storage_save_session_state', { sessionState: sessionState })
    } catch (error) {
      throw new Error(handleError(error, '保存会话状态失败'))
    }
  }

  /**
   * 加载会话状态
   */
  async loadSessionState(): Promise<SessionState | null> {
    try {
      return await invoke<SessionState | null>('storage_load_session_state')
    } catch (error) {
      throw new Error(handleError(error, '加载会话状态失败'))
    }
  }

  // ============================================================================
  // 数据查询和保存
  // ============================================================================

  /**
   * 查询数据
   */
  async queryData<T = any>(query: DataQuery): Promise<T[]> {
    try {
      return await invoke<T[]>('storage_query_data', { query })
    } catch (error) {
      throw new Error(handleError(error, '查询数据失败'))
    }
  }

  /**
   * 保存数据
   */
  async saveData(data: any, options: SaveOptions): Promise<void> {
    try {
      await invoke<void>('storage_save_data', { data, options })
    } catch (error) {
      throw new Error(handleError(error, '保存数据失败'))
    }
  }

  // ============================================================================
  // 系统监控
  // ============================================================================

  /**
   * 执行健康检查
   */
  async healthCheck(): Promise<HealthCheckResult> {
    try {
      return await invoke<HealthCheckResult>('storage_health_check')
    } catch (error) {
      throw new Error(handleError(error, '健康检查失败'))
    }
  }

  /**
   * 获取缓存统计信息
   */
  async getCacheStats(): Promise<CacheStats> {
    try {
      return await invoke<CacheStats>('storage_get_cache_stats')
    } catch (error) {
      throw new Error(handleError(error, '获取缓存统计失败'))
    }
  }

  /**
   * 获取存储统计信息
   */
  async getStorageStats(): Promise<StorageStats> {
    try {
      return await invoke<StorageStats>('storage_get_storage_stats')
    } catch (error) {
      throw new Error(handleError(error, '获取存储统计失败'))
    }
  }

  // ============================================================================
  // 缓存管理
  // ============================================================================

  /**
   * 预加载缓存
   */
  async preloadCache(): Promise<void> {
    try {
      await invoke<void>('storage_preload_cache')
    } catch (error) {
      throw new Error(handleError(error, '预加载缓存失败'))
    }
  }

  /**
   * 清空缓存
   */
  async clearCache(): Promise<void> {
    try {
      await invoke<void>('storage_clear_cache')
    } catch (error) {
      throw new Error(handleError(error, '清空缓存失败'))
    }
  }

  // ============================================================================
  // 便捷方法
  // ============================================================================

  /**
   * 获取应用配置
   */
  async getAppConfig<T = any>(): Promise<T> {
    return this.getConfig<T>(ConfigSection.App)
  }

  /**
   * 获取外观配置
   */
  async getAppearanceConfig<T = any>(): Promise<T> {
    return this.getConfig<T>(ConfigSection.Appearance)
  }

  /**
   * 获取终端配置
   */
  async getTerminalConfig<T = any>(): Promise<T> {
    return this.getConfig<T>(ConfigSection.Terminal)
  }

  /**
   * 获取快捷键配置
   */
  async getShortcutsConfig<T = any>(): Promise<T> {
    return this.getConfig<T>(ConfigSection.Shortcuts)
  }

  /**
   * 获取AI配置
   */
  async getAiConfig<T = any>(): Promise<T> {
    return this.getConfig<T>(ConfigSection.Ai)
  }

  /**
   * 更新应用配置
   */
  async updateAppConfig(data: any): Promise<void> {
    return this.updateConfig(ConfigSection.App, data)
  }

  /**
   * 更新外观配置
   */
  async updateAppearanceConfig(data: any): Promise<void> {
    return this.updateConfig(ConfigSection.Appearance, data)
  }

  /**
   * 更新终端配置
   */
  async updateTerminalConfig(data: any): Promise<void> {
    return this.updateConfig(ConfigSection.Terminal, data)
  }

  /**
   * 更新快捷键配置
   */
  async updateShortcutsConfig(data: any): Promise<void> {
    return this.updateConfig(ConfigSection.Shortcuts, data)
  }

  /**
   * 更新AI配置
   */
  async updateAiConfig(data: any): Promise<void> {
    return this.updateConfig(ConfigSection.Ai, data)
  }

  /**
   * 批量更新配置
   */
  async batchUpdateConfig(updates: Array<{ section: ConfigSection | string; data: any }>): Promise<void> {
    const promises = updates.map(({ section, data }) => this.updateConfig(section, data))
    await Promise.all(promises)
  }

  /**
   * 获取完整的系统状态
   */
  async getSystemStatus() {
    const [health, cacheStats, storageStats] = await Promise.all([
      this.healthCheck(),
      this.getCacheStats(),
      this.getStorageStats(),
    ])

    return {
      health,
      cacheStats,
      storageStats,
      timestamp: Date.now(),
    }
  }

  /**
   * 安全的配置更新（带错误处理）
   */
  async safeUpdateConfig(section: ConfigSection | string, data: any): Promise<StorageOperationResult> {
    try {
      await this.updateConfig(section, data)
      return { success: true }
    } catch (error) {
      return {
        success: false,
        error: handleError(error, '配置更新失败'),
      }
    }
  }
}

/**
 * 存储API实例
 */
export const storageAPI = new StorageAPI()

/**
 * 便捷的存储操作函数
 */
export const storage = {
  // 配置管理
  getConfig: <T = any>(section: ConfigSection | string) => storageAPI.getConfig<T>(section),
  updateConfig: (section: ConfigSection | string, data: any) => storageAPI.updateConfig(section, data),
  batchUpdateConfig: (updates: Array<{ section: ConfigSection | string; data: any }>) =>
    storageAPI.batchUpdateConfig(updates),

  // 会话状态
  saveSessionState: (sessionState: SessionState) => storageAPI.saveSessionState(sessionState),
  loadSessionState: () => storageAPI.loadSessionState(),

  // 数据操作
  queryData: <T = any>(query: DataQuery) => storageAPI.queryData<T>(query),
  saveData: (data: any, options: SaveOptions) => storageAPI.saveData(data, options),

  // 系统监控
  healthCheck: () => storageAPI.healthCheck(),
  getCacheStats: () => storageAPI.getCacheStats(),
  getStorageStats: () => storageAPI.getStorageStats(),
  getSystemStatus: () => storageAPI.getSystemStatus(),

  // 缓存管理
  preloadCache: () => storageAPI.preloadCache(),
  clearCache: () => storageAPI.clearCache(),

  // 便捷方法
  getAppConfig: <T = any>() => storageAPI.getAppConfig<T>(),
  getAppearanceConfig: <T = any>() => storageAPI.getAppearanceConfig<T>(),
  getTerminalConfig: <T = any>() => storageAPI.getTerminalConfig<T>(),
  getShortcutsConfig: <T = any>() => storageAPI.getShortcutsConfig<T>(),
  getAiConfig: <T = any>() => storageAPI.getAiConfig<T>(),
  updateAppConfig: (data: any) => storageAPI.updateAppConfig(data),
  updateAppearanceConfig: (data: any) => storageAPI.updateAppearanceConfig(data),
  updateTerminalConfig: (data: any) => storageAPI.updateTerminalConfig(data),
  updateShortcutsConfig: (data: any) => storageAPI.updateShortcutsConfig(data),
  updateAiConfig: (data: any) => storageAPI.updateAiConfig(data),

  // 高级功能
  safeUpdateConfig: (section: ConfigSection | string, data: any) => storageAPI.safeUpdateConfig(section, data),
}

// 重新导出类型
export type * from './types'
