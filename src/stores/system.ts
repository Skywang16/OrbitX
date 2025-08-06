/**
 * 系统监控Store
 *
 * 管理存储系统的健康检查、缓存统计、性能监控等功能
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { storage } from '@/api/storage'
import {
  formatBytes,
  calculateHitRate,
  type HealthCheckResult,
  type CacheStats,
  type StorageStats,
} from '@/types/storage'
import { handleErrorWithMessage } from '@/utils/errorHandler'

/**
 * 系统状态接口
 */
interface SystemStatus {
  health: HealthCheckResult
  cacheStats: CacheStats
  storageStats: StorageStats
  timestamp: number
}

/**
 * 系统监控Store
 */
export const useSystemStore = defineStore('system', () => {
  // ============================================================================
  // 状态定义
  // ============================================================================

  /** 系统健康状态 */
  const healthStatus = ref<HealthCheckResult | null>(null)

  /** 缓存统计信息 */
  const cacheStats = ref<CacheStats | null>(null)

  /** 存储统计信息 */
  const storageStats = ref<StorageStats | null>(null)

  /** 完整系统状态 */
  const systemStatus = ref<SystemStatus | null>(null)

  /** 是否正在加载健康状态 */
  const healthLoading = ref(false)

  /** 是否正在加载缓存统计 */
  const cacheLoading = ref(false)

  /** 是否正在加载存储统计 */
  const storageLoading = ref(false)

  /** 是否正在加载完整状态 */
  const statusLoading = ref(false)

  /** 错误信息 */
  const error = ref<string | null>(null)

  /** 最后更新时间 */
  const lastUpdated = ref<number>(0)

  /** 自动刷新定时器 */
  let refreshTimer: NodeJS.Timeout | null = null

  /** 自动刷新间隔（毫秒） */
  const AUTO_REFRESH_INTERVAL = 60000 // 1分钟

  // ============================================================================
  // 计算属性
  // ============================================================================

  /** 是否有任何加载操作正在进行 */
  const isLoading = computed(
    () => healthLoading.value || cacheLoading.value || storageLoading.value || statusLoading.value
  )

  /** 系统是否健康 */
  const isHealthy = computed(() => healthStatus.value?.healthy ?? false)

  /** 总缓存命中率 */
  const totalHitRate = computed(() => cacheStats.value?.total_hit_rate ?? 0)

  /** 格式化的总内存使用量 */
  const formattedMemoryUsage = computed(() =>
    cacheStats.value ? formatBytes(cacheStats.value.total_memory_usage) : '0 B'
  )

  /** 格式化的总存储大小 */
  const formattedTotalSize = computed(() => (storageStats.value ? formatBytes(storageStats.value.total_size) : '0 B'))

  /** 缓存效率评级 */
  const cacheEfficiencyRating = computed(() => {
    const hitRate = totalHitRate.value
    if (hitRate >= 0.9) return 'excellent'
    if (hitRate >= 0.7) return 'good'
    if (hitRate >= 0.5) return 'fair'
    return 'poor'
  })

  /** 存储使用情况分析 */
  const storageBreakdown = computed(() => {
    if (!storageStats.value) return []

    const stats = storageStats.value
    return [
      { name: '配置', size: stats.config_size, formatted: formatBytes(stats.config_size) },
      { name: '状态', size: stats.state_size, formatted: formatBytes(stats.state_size) },
      { name: '数据', size: stats.data_size, formatted: formatBytes(stats.data_size) },
      { name: '缓存', size: stats.cache_size, formatted: formatBytes(stats.cache_size) },
      { name: '备份', size: stats.backups_size, formatted: formatBytes(stats.backups_size) },
      { name: '日志', size: stats.logs_size, formatted: formatBytes(stats.logs_size) },
    ].sort((a, b) => b.size - a.size)
  })

  // ============================================================================
  // 核心方法
  // ============================================================================

  /**
   * 执行健康检查
   */
  const checkHealth = async (): Promise<HealthCheckResult> => {
    healthLoading.value = true
    error.value = null

    try {
      const result = await storage.healthCheck()
      healthStatus.value = result
      lastUpdated.value = Date.now()
      return result
    } catch (err) {
      error.value = handleErrorWithMessage(err, '健康检查失败')
      console.error('健康检查失败:', err)
      throw err
    } finally {
      healthLoading.value = false
    }
  }

  /**
   * 获取缓存统计
   */
  const getCacheStatistics = async (): Promise<CacheStats> => {
    cacheLoading.value = true
    error.value = null

    try {
      const stats = await storage.getCacheStats()
      cacheStats.value = stats
      lastUpdated.value = Date.now()
      return stats
    } catch (err) {
      error.value = handleErrorWithMessage(err, '获取缓存统计失败')
      console.error('获取缓存统计失败:', err)
      throw err
    } finally {
      cacheLoading.value = false
    }
  }

  /**
   * 获取存储统计
   */
  const getStorageStatistics = async (): Promise<StorageStats> => {
    storageLoading.value = true
    error.value = null

    try {
      const stats = await storage.getStorageStats()
      storageStats.value = stats
      lastUpdated.value = Date.now()
      return stats
    } catch (err) {
      error.value = handleErrorWithMessage(err, '获取存储统计失败')
      console.error('获取存储统计失败:', err)
      throw err
    } finally {
      storageLoading.value = false
    }
  }

  /**
   * 获取完整系统状态
   */
  const getSystemStatus = async (): Promise<SystemStatus> => {
    statusLoading.value = true
    error.value = null

    try {
      const status = await storage.getSystemStatus()
      systemStatus.value = status

      // 同时更新各个子状态
      healthStatus.value = status.health
      cacheStats.value = status.cacheStats
      storageStats.value = status.storageStats
      lastUpdated.value = status.timestamp

      return status
    } catch (err) {
      error.value = handleErrorWithMessage(err, '获取系统状态失败')
      console.error('获取系统状态失败:', err)
      throw err
    } finally {
      statusLoading.value = false
    }
  }

  /**
   * 刷新所有统计信息
   */
  const refreshAll = async (): Promise<void> => {
    await Promise.allSettled([checkHealth(), getCacheStatistics(), getStorageStatistics()])
  }

  // ============================================================================
  // 缓存管理方法
  // ============================================================================

  /**
   * 预加载缓存
   */
  const preloadCache = async (): Promise<void> => {
    try {
      await storage.preloadCache()
      // 刷新缓存统计
      await getCacheStatistics()
    } catch (err) {
      error.value = handleErrorWithMessage(err, '预加载缓存失败')
      console.error('预加载缓存失败:', err)
      throw err
    }
  }

  /**
   * 清空缓存
   */
  const clearCache = async (): Promise<void> => {
    try {
      await storage.clearCache()
      // 刷新缓存统计
      await getCacheStatistics()
    } catch (err) {
      error.value = handleErrorWithMessage(err, '清空缓存失败')
      console.error('清空缓存失败:', err)
      throw err
    }
  }

  // ============================================================================
  // 自动刷新管理
  // ============================================================================

  /**
   * 启动自动刷新
   */
  const startAutoRefresh = (): void => {
    if (refreshTimer) return

    refreshTimer = setInterval(() => {
      refreshAll().catch(err => {
        console.warn('自动刷新系统状态失败:', err)
      })
    }, AUTO_REFRESH_INTERVAL)
  }

  /**
   * 停止自动刷新
   */
  const stopAutoRefresh = (): void => {
    if (refreshTimer) {
      clearInterval(refreshTimer)
      refreshTimer = null
    }
  }

  /**
   * 清除错误
   */
  const clearError = (): void => {
    error.value = null
  }

  /**
   * 重置所有状态
   */
  const reset = (): void => {
    healthStatus.value = null
    cacheStats.value = null
    storageStats.value = null
    systemStatus.value = null
    error.value = null
    lastUpdated.value = 0
  }

  /**
   * 初始化系统监控
   */
  const initialize = async (): Promise<void> => {
    try {
      await getSystemStatus()
      startAutoRefresh()
    } catch (err) {
      console.error('系统监控初始化失败:', err)
      throw err
    }
  }

  return {
    // 状态
    healthStatus,
    cacheStats,
    storageStats,
    systemStatus,
    healthLoading,
    cacheLoading,
    storageLoading,
    statusLoading,
    error,
    lastUpdated,

    // 计算属性
    isLoading,
    isHealthy,
    totalHitRate,
    formattedMemoryUsage,
    formattedTotalSize,
    cacheEfficiencyRating,
    storageBreakdown,

    // 核心方法
    checkHealth,
    getCacheStatistics,
    getStorageStatistics,
    getSystemStatus,
    refreshAll,

    // 缓存管理
    preloadCache,
    clearCache,

    // 工具方法
    startAutoRefresh,
    stopAutoRefresh,
    clearError,
    reset,
    initialize,
  }
})

export default useSystemStore
