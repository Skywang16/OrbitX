import { ref, computed, onMounted } from 'vue'
import { llmRegistryApi, aiApi } from '@/api'
import type { ProviderInfo, ProviderOption, ModelOption, AIModelConfig } from '@/types'

export const useLLMRegistry = () => {
  const providers = ref<ProviderInfo[]>([])
  const userModels = ref<AIModelConfig[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  // 转换后端数据为前端表单使用的格式
  const providerOptions = computed<ProviderOption[]>(() => {
    return providers.value.map(provider => {
      // 获取该 provider 下的所有用户配置模型
      const providerModels = userModels.value
        .filter(m => m.provider.toLowerCase() === provider.providerType.toLowerCase())
        .map(m => m.id)

      return {
        value: provider.providerType.toLowerCase(),
        label: provider.displayName,
        apiUrl: provider.defaultApiUrl,
        models: providerModels,
      }
    })
  })

  // 根据供应商类型获取模型选项
  const getModelOptions = (providerType: string, modelType?: 'chat' | 'embedding'): ModelOption[] => {
    const provider = providers.value.find(p => p.providerType.toLowerCase() === providerType.toLowerCase())

    if (!provider) return []

    // 如果有预设模型，返回预设模型列表
    if (provider.presetModels && provider.presetModels.length > 0) {
      return provider.presetModels.map(model => ({
        value: model.id,
        label: model.name,
      }))
    }

    // 否则返回空数组（表示需要手动输入）
    return []
  }

  // 获取聊天模型选项
  const getChatModelOptions = (providerType: string): ModelOption[] => {
    return getModelOptions(providerType, 'chat')
  }

  // 获取向量模型选项
  const getEmbeddingModelOptions = (providerType: string): ModelOption[] => {
    const options = getModelOptions(providerType, 'embedding')
    if (options.length === 0) {
      return [
        {
          value: '',
          label: '暂无向量模型',
        },
      ]
    }
    return options
  }

  // 根据供应商类型获取供应商信息
  const getProviderInfo = (providerType: string): ProviderInfo | null => {
    return providers.value.find(p => p.providerType.toLowerCase() === providerType.toLowerCase()) || null
  }

  // 加载供应商和模型数据
  const loadProviders = async () => {
    if (isLoading.value) return

    isLoading.value = true
    error.value = null

    try {
      // 并发请求 Provider 元数据和用户模型
      const [providersData, modelsData] = await Promise.all([llmRegistryApi.getProviders(), aiApi.getModels()])

      providers.value = providersData
      userModels.value = modelsData
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
    userModels,
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
