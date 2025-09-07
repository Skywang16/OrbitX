/**
 * 向量索引设置状态管理
 */

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { vectorIndexApi } from '@/api'
import type { VectorIndexConfig, VectorIndexStatus } from '@/api/vector-index'

export const useVectorIndexSettingsStore = defineStore('vectorIndexSettings', () => {
  // 状态
  const isInitialized = ref(false)
  const isLoading = ref(false)
  const isSaving = ref(false)
  const error = ref<string | null>(null)

  const config = ref<VectorIndexConfig | null>(null)
  const indexStatus = ref<VectorIndexStatus | null>(null)

  // 加载设置
  const loadSettings = async () => {
    if (isLoading.value) return

    isLoading.value = true
    error.value = null

    try {
      // 从后端获取向量索引配置
      const vectorConfig = await vectorIndexApi.getConfig()
      config.value = vectorConfig

      // 获取索引状态
      await refreshIndexStatus()

      isInitialized.value = true
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      console.error('Failed to load vector index settings:', err)
    } finally {
      isLoading.value = false
    }
  }

  // 保存配置
  const saveConfig = async (newConfig: Partial<VectorIndexConfig>) => {
    isSaving.value = true
    error.value = null

    try {
      const configToSave: VectorIndexConfig = {
        qdrantUrl: newConfig.qdrantUrl || 'http://localhost:6333',
        qdrantApiKey: newConfig.qdrantApiKey || null,
        collectionName: newConfig.collectionName || 'orbitx-code-vectors',
        embeddingModelId: newConfig.embeddingModelId || 'text-embedding-3-small',
        maxConcurrentFiles: newConfig.maxConcurrentFiles || 4,
      }
      await vectorIndexApi.saveConfig(configToSave)
      config.value = configToSave

      // 重新初始化向量索引服务
      await vectorIndexApi.init(configToSave)
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      isSaving.value = false
    }
  }

  // 测试连接
  const testConnection = async (testConfig: VectorIndexConfig) => {
    try {
      const result = await vectorIndexApi.testConnection(testConfig)
      return result
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    }
  }

  // 构建代码索引
  const buildCodeIndex = async () => {
    if (!config.value?.qdrantUrl) {
      throw new Error('Vector index not configured')
    }

    try {
      const stats = await vectorIndexApi.build()

      return stats
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    }
  }

  // 取消构建索引
  const cancelBuildIndex = async () => {
    try {
      await vectorIndexApi.cancelBuild()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    }
  }

  // 刷新索引状态
  const refreshIndexStatus = async () => {
    try {
      const status = await vectorIndexApi.getStatus()
      indexStatus.value = status
      return status
    } catch (err) {
      // 索引状态获取失败时不设置错误，可能是服务未初始化
      indexStatus.value = {
        isInitialized: false,
        totalVectors: 0,
        lastUpdated: null,
      }
      return indexStatus.value
    }
  }

  // 清除索引
  const clearIndex = async () => {
    if (!config.value?.collectionName) {
      throw new Error('No collection configured')
    }

    try {
      await vectorIndexApi.clear()

      // 刷新状态
      await refreshIndexStatus()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    }
  }

  // 重置到默认值
  const resetToDefaults = async () => {
    const defaultConfig: VectorIndexConfig = {
      qdrantUrl: 'http://localhost:6333',
      qdrantApiKey: null,
      collectionName: 'orbitx-code-vectors',
      vectorSize: 1536,
      batchSize: 50,
      maxConcurrentFiles: 4,
      chunkSizeRange: [10, 2000],
      supportedExtensions: ['.ts', '.tsx', '.js', '.jsx', '.rs', '.py', '.go', '.java', '.c', '.cpp', '.h', '.hpp'],
      ignorePatterns: ['**/node_modules/**', '**/target/**', '**/dist/**', '**/.git/**', '**/build/**'],
      embeddingModelId: undefined,
    }

    await saveConfig(defaultConfig)
  }

  return {
    // 状态
    isInitialized,
    isLoading,
    isSaving,
    error,
    config,
    indexStatus,

    // 方法
    loadSettings,
    saveConfig,
    testConnection,
    buildCodeIndex,
    cancelBuildIndex,
    refreshIndexStatus,
    clearIndex,
    resetToDefaults,
  }
})
