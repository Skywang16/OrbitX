import { invoke } from '@/utils/request'
import type { ProviderInfo } from '@/types/domain/llm-registry'

export interface LLMRegistryApi {
  /**
   * 获取所有供应商元数据
   */
  getProviders(): Promise<ProviderInfo[]>
}

class LLMRegistryApiImpl implements LLMRegistryApi {
  getProviders = async (): Promise<ProviderInfo[]> => {
    return await invoke('llm_get_providers')
  }
}

export const llmRegistryApi = new LLMRegistryApiImpl()
