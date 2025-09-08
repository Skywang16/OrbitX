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

    // 从后端获取向量索引配置
    const vectorConfig = await vectorIndexApi.getConfig()
    config.value = vectorConfig

    // 获取索引状态
    await refreshIndexStatus()

    isInitialized.value = true
    isLoading.value = false
  }

  // 保存配置
  const saveConfig = async (newConfig: Partial<VectorIndexConfig>) => {
    isSaving.value = true
    error.value = null

    const configToSave: VectorIndexConfig = {
      qdrantUrl: newConfig.qdrantUrl || 'http://localhost:6334',
      qdrantApiKey: newConfig.qdrantApiKey || null,
      collectionName: newConfig.collectionName || 'orbitx-code-vectors',
      embeddingModelId: newConfig.embeddingModelId || 'text-embedding-3-small',
      maxConcurrentFiles: newConfig.maxConcurrentFiles || 4,
    }
    await vectorIndexApi.saveConfig(configToSave)
    config.value = configToSave

    // 重新初始化向量索引服务
    await vectorIndexApi.init(configToSave)

    // 自动刷新索引状态
    await refreshIndexStatus()
    isSaving.value = false
  }

  // 测试连接
  const testConnection = async (testConfig: VectorIndexConfig) => {
    const result = await vectorIndexApi.testConnection(testConfig)
    return result
  }

  // 构建代码索引
  const buildCodeIndex = async () => {
    if (!config.value?.qdrantUrl) {
      throw new Error('Vector index not configured')
    }

    const stats = await vectorIndexApi.build()
    return stats
  }

  // 取消构建索引
  const cancelBuildIndex = async () => {
    await vectorIndexApi.cancelBuild()
  }

  // 刷新索引状态
  const refreshIndexStatus = async () => {
    const status = await vectorIndexApi.getStatus()
    indexStatus.value = status
    return status
  }

  // 清除索引
  const clearIndex = async () => {
    if (!config.value?.collectionName) {
      throw new Error('No collection configured')
    }

    await vectorIndexApi.clear()
    // 刷新状态
    await refreshIndexStatus()
  }

  // 重置到默认值
  const resetToDefaults = async () => {
    const defaultConfig: VectorIndexConfig = {
      qdrantUrl: 'http://localhost:6334',
      qdrantApiKey: null,
      collectionName: 'orbitx-code-vectors',
      embeddingModelId: 'text-embedding-3-small',
      maxConcurrentFiles: 4,
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
