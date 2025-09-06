import { invoke } from '@tauri-apps/api/core'
import type { ProviderInfo, ModelInfo, LLMProviderType } from '@/types/domain/llm-registry'

export interface LLMRegistryApi {
  /**
   * 获取所有供应商信息
   */
  getProviders(): Promise<ProviderInfo[]>

  /**
   * 获取指定供应商的模型列表
   */
  getProviderModels(providerType: LLMProviderType): Promise<ModelInfo[]>

  /**
   * 根据模型ID获取模型信息
   */
  getModelInfo(modelId: string): Promise<{ provider: ProviderInfo; model: ModelInfo } | null>

  /**
   * 检查模型是否支持指定功能
   */
  checkModelFeature(modelId: string, feature: string): Promise<boolean>
}

class LLMRegistryApiImpl implements LLMRegistryApi {
  async getProviders(): Promise<ProviderInfo[]> {
    return await invoke('llm_get_providers')
  }

  async getProviderModels(providerType: LLMProviderType): Promise<ModelInfo[]> {
    return await invoke('llm_get_provider_models', { providerType })
  }

  async getModelInfo(modelId: string): Promise<{ provider: ProviderInfo; model: ModelInfo } | null> {
    const result = await invoke<[ProviderInfo, ModelInfo] | null>('llm_get_model_info', { modelId })
    if (result) {
      return {
        provider: result[0],
        model: result[1],
      }
    }
    return null
  }

  async checkModelFeature(modelId: string, feature: string): Promise<boolean> {
    return await invoke('llm_check_model_feature', { modelId, feature })
  }
}

export const llmRegistryApi = new LLMRegistryApiImpl()
export type { LLMRegistryApi }
