import { ref, computed, onMounted } from 'vue'
import { llmRegistryApi } from '@/api'
import type { ProviderInfo, ProviderOption, ModelOption } from '@/types'

export const useLLMRegistry = () => {
  const providers = ref<ProviderInfo[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  // 转换后端数据为前端表单使用的格式
  const providerOptions = computed<ProviderOption[]>(() => {
    return providers.value.map(provider => ({
      value: provider.providerType.toLowerCase(),
      label: provider.displayName,
      apiUrl: provider.defaultApiUrl,
      models: provider.models.map(model => model.id),
      requiresApiKey: provider.requiresApiKey,
      documentationUrl: provider.documentationUrl,
    }))
  })

  // 根据供应商类型获取模型选项
  const getModelOptions = (providerType: string, modelType?: 'chat' | 'embedding'): ModelOption[] => {
    const provider = providers.value.find(p => p.providerType.toLowerCase() === providerType.toLowerCase())
    if (!provider) return []

    return provider.models
      .filter(model => !model.deprecated) // 过滤掉已弃用的模型
      .filter(model => !modelType || model.modelType === modelType) // 按模型类型过滤
      .map(model => ({
        value: model.id,
        label: model.displayName,
        capabilities: model.capabilities,
        deprecated: model.deprecated,
      }))
  }

  // 获取聊天模型选项
  const getChatModelOptions = (providerType: string): ModelOption[] => {
    return getModelOptions(providerType, 'chat')
  }

  // 获取向量模型选项
  const getEmbeddingModelOptions = (providerType: string): ModelOption[] => {
    const options = getModelOptions(providerType, 'embedding')
    if (options.length === 0) {
      // 如果没有向量模型，返回一个提示选项
      return [
        {
          value: '',
          label: '暂无向量模型',
          capabilities: undefined,
          deprecated: false,
        },
      ]
    }
    return options
  }

  // 根据供应商类型获取供应商信息
  const getProviderInfo = (providerType: string): ProviderInfo | null => {
    return providers.value.find(p => p.providerType.toLowerCase() === providerType.toLowerCase()) || null
  }

  // 加载供应商数据
  const loadProviders = async () => {
    if (isLoading.value) return

    isLoading.value = true
    error.value = null

    try {
      providers.value = await llmRegistryApi.getProviders()
    } catch (err) {
      console.error('加载LLM供应商失败:', err)
      error.value = err instanceof Error ? err.message : '加载失败'
    } finally {
      isLoading.value = false
    }
  }

  // 自动加载数据
  onMounted(() => {
    loadProviders()
  })

  return {
    // 响应式数据
    providers,
    providerOptions,
    isLoading,
    error,

    // 方法
    loadProviders,
    getModelOptions,
    getChatModelOptions,
    getEmbeddingModelOptions,
    getProviderInfo,
  }
}
